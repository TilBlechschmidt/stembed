use super::{state::InputState, KeypressGrouper};
use crate::time::*;
use futures::{
    future::{select, Either},
    stream, Stream, StreamExt,
};

pub struct KeypressRepeater<T: TimeDriver> {
    interval: T::Duration,
    max_tap_delay: T::Duration,
    trigger_delay: T::Duration,

    time_driver: T,
}

struct RepeatState<'s, S: Stream<Item = InputState> + Unpin, T: TimeDriver> {
    grouper: &'s mut KeypressGrouper,
    repeater: &'s KeypressRepeater<T>,

    state_stream: &'s mut S,

    /// When the next repeat should occur
    next_repeat: Option<T::Instant>,
    /// Whether something was emitted for the currently active repeat
    repeated_once: bool,

    /// Which state was last emitted by the grouper
    last_emit: Option<(T::Instant, InputState)>,
    /// Whether repeat is active and for what state
    repeat: Option<InputState>,
}

impl<T: TimeDriver> KeypressRepeater<T> {
    pub fn new(
        interval: impl Into<T::Duration>,
        max_tap_delay: impl Into<T::Duration>,
        trigger_delay: impl Into<T::Duration>,
        time_driver: T,
    ) -> Self {
        Self {
            interval: interval.into(),
            max_tap_delay: max_tap_delay.into(),
            trigger_delay: trigger_delay.into(),
            time_driver,
        }
    }

    // Takes an input stream of keyboard states, groups it using the provided grouper, and repeats with the configured interval if a key is tapped and then held.
    pub fn apply_grouped_repeat<'s>(
        &'s self,
        state_stream: &'s mut (impl Stream<Item = InputState> + Unpin),
        grouper: &'s mut KeypressGrouper,
    ) -> impl Stream<Item = InputState> + 's {
        let repeat_state = RepeatState {
            grouper,
            repeater: self,

            state_stream,
            next_repeat: None,
            repeated_once: false,
            last_emit: None,
            repeat: None,
        };

        stream::unfold(repeat_state, |mut repeat_state| async move {
            // Repeat until we actually have something to emit :)
            loop {
                // Fetch the next item from the input
                let next_input_state = repeat_state.state_stream.next();

                if let Some(repeated_state) = repeat_state.repeat {
                    // Build a future that resolves when the next repeat is due
                    let repeat_instant = repeat_state.next_repeat.unwrap_or_else(|| {
                        repeat_state.repeater.time_driver.now()
                            + repeat_state.repeater.interval
                            + repeat_state.repeater.trigger_delay
                    });
                    let repeat_timer = repeat_state.repeater.time_driver.wait_until(repeat_instant);

                    // Wait for either the next value from the input or the repeat timer
                    match select(next_input_state, repeat_timer).await {
                        // Input yielded nothing, thus the stream is completed
                        Either::Left((None, _)) => return None,
                        // Input yielded something new, process and emit it if applicable
                        Either::Left((Some(state), _)) => {
                            if let Some(state) = repeat_state.push(state) {
                                // If the yielded stroke equals the repeated stroke (and thus ends the repeat)
                                // then do not emit it. This feels more natural instead of emitting the stroke again.
                                //
                                // When repeat is active but has not repeated yet, then emit the release as this is just a regular double-tap.
                                if state == repeated_state && repeat_state.repeated_once {
                                    continue;
                                } else {
                                    return Some((state, repeat_state));
                                }
                            }
                        }
                        // Repeat timer expired, emit a repeated stroke and reset the timer
                        Either::Right(_) => {
                            repeat_state.repeated_once = true;
                            repeat_state.next_repeat = Some(
                                repeat_state.repeater.time_driver.now()
                                    + repeat_state.repeater.interval,
                            );
                            return Some((repeated_state, repeat_state));
                        }
                    }
                } else if let Some(state) = next_input_state.await {
                    // Group the input, emit if applicable or just repeat
                    if let Some(state) = repeat_state.push(state) {
                        return Some((state, repeat_state));
                    } else {
                        continue;
                    }
                } else {
                    // Input yielded nothing, thus the stream is completed
                    return None;
                }
            }
        })
    }
}

impl<'s, S: Stream<Item = InputState> + Unpin, T: TimeDriver> RepeatState<'s, S, T> {
    // Forwards a state to the grouper and updates the repeat state
    fn push(&mut self, state: InputState) -> Option<InputState> {
        let mut inhibit_emit = false;

        // Update the repeating state
        if let Some((timestamp, last_emit)) = self.last_emit {
            let state_matches = last_emit == state;
            let time_qualifies = timestamp.elapsed() < self.repeater.max_tap_delay;

            if state_matches && time_qualifies {
                self.repeat = Some(state);
            } else if self.repeat.is_some() {
                self.repeat = None;
                self.last_emit = None;
                self.next_repeat = None;
                self.repeated_once = false;

                // Make sure that `last_emit` stays empty so a full tap-tap-hold is required to trigger it again instead of just a tap-hold
                inhibit_emit = true;
            }
        }

        // Push the state into the grouper and handle any emitted strokes
        if let Some(emit) = self.grouper.push(state) {
            if !inhibit_emit {
                self.last_emit = Some((self.repeater.time_driver.now(), emit));
            }
            Some(emit)
        } else {
            None
        }
    }
}

use super::KeypressGrouper;
use embassy::{
    time::{Duration, Instant, Timer},
    util::{select, Either},
};
use futures::{stream, Stream, StreamExt};

pub struct KeypressRepeater {
    interval: Duration,
    max_tap_delay: Duration,
}

struct RepeatState<'s, const KEY_COUNT: usize, S: Stream<Item = [bool; KEY_COUNT]> + Unpin> {
    grouper: &'s mut KeypressGrouper<KEY_COUNT>,
    repeater: &'s KeypressRepeater,

    state_stream: &'s mut S,

    /// When the next repeat should occur
    next_repeat: Option<Instant>,
    /// Whether something was emitted for the currently active repeat
    repeated_once: bool,

    /// Which stroke was last emitted by the grouper
    last_emit: Option<(Instant, [bool; KEY_COUNT])>,
    /// Whether repeat is active and for what stroke
    repeat: Option<[bool; KEY_COUNT]>,
}

impl KeypressRepeater {
    pub fn new(interval: Duration, max_tap_delay: Duration) -> Self {
        Self {
            interval,
            max_tap_delay,
        }
    }

    // Takes an input stream of keyboard states, groups it using the provided grouper, and repeats with the configured interval if a key is tapped and then held.
    pub fn apply_grouped_repeat<'s, const KEY_COUNT: usize>(
        &'s self,
        state_stream: &'s mut (impl Stream<Item = [bool; KEY_COUNT]> + Unpin),
        grouper: &'s mut KeypressGrouper<KEY_COUNT>,
    ) -> impl Stream<Item = [bool; KEY_COUNT]> + 's {
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
                    let repeat_instant = repeat_state
                        .next_repeat
                        .unwrap_or_else(|| Instant::now() + repeat_state.repeater.interval);
                    let repeat_timer = Timer::at(repeat_instant);

                    // Wait for either the next value from the input or the repeat timer
                    let next_state_future = select(next_input_state, repeat_timer);
                    match next_state_future.await {
                        // Input yielded nothing, thus the stream is completed
                        Either::First(None) => return None,
                        // Input yielded something new, process and emit it if applicable
                        Either::First(Some(state)) => {
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
                        Either::Second(_) => {
                            repeat_state.repeated_once = true;
                            repeat_state.next_repeat =
                                Some(Instant::now() + repeat_state.repeater.interval);
                            return Some((repeated_state, repeat_state));
                        }
                    }
                } else if let Some(state) = next_input_state.await {
                    repeat_state.repeated_once = false;

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

impl<'s, const KEY_COUNT: usize, S: Stream<Item = [bool; KEY_COUNT]> + Unpin>
    RepeatState<'s, KEY_COUNT, S>
{
    // Forwards a state to the grouper and updates the repeat state
    fn push(&mut self, state: [bool; KEY_COUNT]) -> Option<[bool; KEY_COUNT]> {
        // Update the repeating state
        if let Some((timestamp, last_emit)) = self.last_emit {
            let state_matches = last_emit == state;
            let time_qualifies = timestamp.elapsed() < self.repeater.max_tap_delay;

            if state_matches && time_qualifies {
                self.repeat = Some(state);
            } else {
                self.repeat = None;
            }
        }

        // Push the state into the grouper and handle any emitted strokes
        if let Some(emit) = self.grouper.push(state) {
            self.last_emit = Some((Instant::now(), emit));
            Some(emit)
        } else {
            None
        }
    }
}

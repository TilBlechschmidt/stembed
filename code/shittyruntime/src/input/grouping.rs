use super::state::{InputState, KeyEvent};

pub enum GroupingMode {
    FirstUp,
    LastUp,
}

pub struct KeypressGrouper {
    mode: GroupingMode,
    flagged: InputState,
    previous: InputState,
}

impl KeypressGrouper {
    pub fn new(mode: GroupingMode) -> Self {
        Self {
            mode,
            flagged: InputState::EMPTY,
            previous: InputState::EMPTY,
        }
    }

    /// Adds a keyboard state to the grouper and optionally emits a state if the grouping criterum is fulfilled
    pub fn push(&mut self, state: InputState) -> Option<InputState> {
        match self.mode {
            GroupingMode::FirstUp => self.push_first_up(state),
            GroupingMode::LastUp => self.push_last_up(state),
        }
    }

    // This function repurposes the `flagged` state as an accumulator for keys until the last one is released
    fn push_last_up(&mut self, state: InputState) -> Option<InputState> {
        let nothing_pressed = state.is_empty();
        let accumulator_filled = !self.flagged.is_empty();

        if nothing_pressed && accumulator_filled {
            let accumulated_stroke = self.flagged;
            self.flagged = InputState::EMPTY;
            Some(accumulated_stroke)
        } else {
            self.flagged += state;
            None
        }
    }

    fn push_first_up(&mut self, state: InputState) -> Option<InputState> {
        let mut emit = None;

        for (position, edge) in self.previous.edges_toward(state) {
            if !self.flagged.is_set(position) && edge == KeyEvent::Up {
                // Whenever a non-flagged key is released, emit a stroke containing all previously pressed keys.
                emit = Some(self.previous);

                // Flag all emitted keys
                self.flagged += self.previous;
            } else if self.flagged.is_set(position) && edge == KeyEvent::Down {
                // When pressing a flagged key, remove any flag
                self.flagged.unset(position);
            }
        }

        self.previous = state;

        emit
    }
}

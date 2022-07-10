pub enum GroupingMode {
    FirstUp,
    LastUp,
}

pub struct KeypressGrouper<const KEY_COUNT: usize> {
    mode: GroupingMode,
    flagged: [bool; KEY_COUNT],
    previous: [bool; KEY_COUNT],
}

#[derive(PartialEq, Eq)]
enum KeyEvent {
    Up,
    Down,
}

impl<const KEY_COUNT: usize> KeypressGrouper<KEY_COUNT> {
    pub fn new(mode: GroupingMode) -> Self {
        Self {
            mode,
            flagged: [false; KEY_COUNT],
            previous: [false; KEY_COUNT],
        }
    }

    // Adds a keyboard state to the grouper and optionally emits a state if the grouping criterum is fulfilled
    pub fn push(&mut self, state: [bool; KEY_COUNT]) -> Option<[bool; KEY_COUNT]> {
        match self.mode {
            GroupingMode::FirstUp => self.push_first_up(state),
            GroupingMode::LastUp => self.push_last_up(state),
        }
    }

    // This function repurposes the `flagged` state as an accumulator for keys until the last one is released
    fn push_last_up(&mut self, state: [bool; KEY_COUNT]) -> Option<[bool; KEY_COUNT]> {
        let nothing_pressed: bool = !state
            .iter()
            .cloned()
            .reduce(|acc, key| acc || key)
            .unwrap_or(true);

        let accumulator_filled = self
            .flagged
            .iter()
            .cloned()
            .reduce(|acc, key| acc || key)
            .unwrap_or(true);

        if nothing_pressed && accumulator_filled {
            let accumulated_stroke = self.flagged;
            self.flagged = [false; KEY_COUNT];
            Some(accumulated_stroke)
        } else {
            state
                .iter()
                .enumerate()
                .filter(|(_, key)| **key)
                .for_each(|(i, _)| self.flagged[i] = true);
            None
        }
    }

    fn push_first_up(&mut self, state: [bool; KEY_COUNT]) -> Option<[bool; KEY_COUNT]> {
        let mut emit = None;

        for (i, edge) in self.edges_between(&self.previous, &state) {
            if !self.flagged[i] && edge == KeyEvent::Up {
                // Whenever a non-flagged key is released, emit a stroke containing all previously pressed keys.
                emit = Some(self.previous);

                // Flag all emitted keys
                for (i, state) in self.previous.iter().enumerate() {
                    self.flagged[i] |= state;
                }
            } else if self.flagged[i] && edge == KeyEvent::Down {
                // When pressing a flagged key, remove any flag
                self.flagged[i] = false;
            }
        }

        self.previous = state;

        emit
    }

    fn edges_between<'s>(
        &self,
        prev: &'s [bool; KEY_COUNT],
        next: &'s [bool; KEY_COUNT],
    ) -> impl Iterator<Item = (usize, KeyEvent)> + 's {
        prev.iter()
            .zip(next.iter())
            .enumerate()
            .filter_map(|(i, states)| match states {
                (false, true) => Some((i, KeyEvent::Down)),
                (true, false) => Some((i, KeyEvent::Up)),
                (false, false) | (true, true) => None,
            })
    }
}

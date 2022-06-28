//! Stenography state machine for resolving strokes into outlines
// It is not *really* a state machine but it rather borrows some concepts from the pattern.

mod entry;
mod mutator;
mod resolver;
mod state;

use mutator::StateMutator;
use resolver::StateResolver;
use state::State;

pub use entry::{HistoryEntry, OutlineInformation};
pub use resolver::{CommitType, TrailingOutline};

// Wrapper around all the lower-level structs to make the API more concise
pub struct OutlineMatcher<Stroke, const HISTORY_SIZE: usize> {
    state: State<Stroke, HISTORY_SIZE>,
    mutator: StateMutator,
    resolver: StateResolver,
}

impl<Stroke, const HISTORY_SIZE: usize> OutlineMatcher<Stroke, HISTORY_SIZE> {
    pub fn new(longest_outline_length: usize) -> Self {
        Self {
            state: State::new(),
            mutator: StateMutator::new(longest_outline_length),
            resolver: StateResolver::new(),
        }
    }

    pub fn add(&mut self, stroke: Stroke) {
        self.mutator.add(stroke, &mut self.state)
    }

    pub fn pop(&mut self) -> Option<OutlineInformation> {
        self.mutator.pop(&mut self.state)
    }

    pub fn commit(
        &mut self,
        prefix_length: usize,
        command_count: usize,
    ) -> Result<CommitType, TrailingOutline<'_, Stroke, HISTORY_SIZE>> {
        self.resolver
            .commit(prefix_length, command_count, &mut self.state)
    }

    pub fn uncommitted_count(&self) -> usize {
        self.state.uncommitted_count
    }

    pub fn uncommitted_strokes(&mut self) -> impl Iterator<Item = &Stroke> + Clone {
        self.state.uncommitted_strokes()
    }

    pub fn committed_strokes(&mut self) -> impl Iterator<Item = &HistoryEntry<Stroke>> + Clone {
        self.state.committed_strokes()
    }
}

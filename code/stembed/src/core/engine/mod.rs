use super::dict::DictExt;
use crate::constants::{AVG_OUTLINE_RATIO, AVG_STROKE_COUNT};
use alloc::collections::VecDeque;
use smallvec::SmallVec;

mod command;
pub use command::*;

mod outline;
pub use outline::*;

pub struct Engine<D>
where
    D: DictExt,
    D::Stroke: Clone,
{
    // TODO Replace with fixed-size stack-allocated version :)
    history: VecDeque<MatchedOutline<D::Stroke>>,
    dictionary: D,
}

impl<D> Engine<D>
where
    D: DictExt,
    D::Stroke: Clone + core::fmt::Debug,
{
    pub fn new(dictionary: D) -> Self {
        Self {
            history: VecDeque::new(),
            dictionary,
        }
    }

    pub fn dictionary(&mut self) -> &mut D {
        &mut self.dictionary
    }

    pub fn push(&mut self, stroke: D::Stroke) -> CommandDelta<D::OutputCommand> {
        self.mutate_stroke_history(|strokes| strokes.push(stroke)).0
    }

    pub fn pop(&mut self) -> Option<(CommandDelta<D::OutputCommand>, D::Stroke)> {
        match self.mutate_stroke_history(|strokes| strokes.pop()) {
            (instructions, Some(stroke)) => Some((instructions, stroke)),
            (_, None) => None,
        }
    }

    fn mutate_stroke_history<M, R>(&mut self, mutator: M) -> (CommandDelta<D::OutputCommand>, R)
    where
        M: FnOnce(&mut SmallVec<[D::Stroke; AVG_OUTLINE_RATIO * AVG_STROKE_COUNT]>) -> R,
    {
        // Pop outlines from the history stack till the point where older ones would not affect the outline matching
        let mut old_outlines: SmallVec<[MatchedOutline<D::Stroke>; AVG_OUTLINE_RATIO]> =
            SmallVec::new();
        let mut stroke_count = 0;
        let target_count = self.dictionary.longest_outline_length();

        while stroke_count < target_count {
            match self.history.pop_back() {
                Some(outline) => {
                    stroke_count += outline.strokes.len();
                    old_outlines.push(outline);
                }
                None => break,
            }
        }

        // Since we pushed the outlines in reverse order, we have to flip the vector around
        old_outlines.reverse();

        // Build an array of strokes from the previous outlines and our new stroke
        let mut strokes: SmallVec<[D::Stroke; AVG_OUTLINE_RATIO * AVG_STROKE_COUNT]> =
            SmallVec::new();

        for outline in old_outlines.iter() {
            for stroke in outline.strokes.iter() {
                strokes.push(stroke.clone());
            }
        }

        // Change the stroke array as desired
        let return_value = mutator(&mut strokes);

        // Re-match the newly built stroke array
        let new_outlines: SmallVec<
            [FetchedOutline<'_, D::Stroke, D::OutputCommand>; AVG_OUTLINE_RATIO],
        > = self.dictionary.find_outlines(&strokes);

        // Run through `old_outlines` and `new_outlines` simultaneously and compare along the way.
        // When we hit the "diversion point", undo all remaining old_outlines and apply all new outlines.
        // While iterating, we push the unchanged outlines directly onto the history stack.
        let mut output = CommandDelta::default();
        let mut old_iter = old_outlines.into_iter().peekable();
        let mut new_iter = new_outlines.into_iter().peekable();

        // TODO Figure out a nicer way of doing this (iterator extension which takes two peekable iterators, zips them, takes while condition is true using peek)
        while let (Some(_), Some(_)) = (old_iter.peek(), new_iter.peek()) {
            let old = old_iter.next().unwrap();
            let new = new_iter.next().unwrap();

            // Since both the old outlines and new outlines have been matched over the same stroke sequence,
            // we only have to compare their length to assert equality – saving on a couple of instructions ;)
            if old.strokes.len() == new.strokes.len() {
                // Both are equal, just put it back on the history stack
                self.history.push_back(old);
            } else {
                // They diverged! Undo the old one, apply the new one.
                output.to_undo += old.command_count as usize;
                self.add_new_outline(new, &mut output);
            }
        }

        // Handle the remaining outlines (undo old, apply new)
        for old in old_iter {
            output.to_undo += old.command_count as usize;
        }

        for new in new_iter {
            self.add_new_outline(new, &mut output);
        }

        // PROFIT! :D
        (output, return_value)
    }

    /// Helper function which processes a new outline, executes its EngineCommands,
    /// collects its OutputCommands, and pushes it onto the history stack
    fn add_new_outline(
        &mut self,
        new: FetchedOutline<D::Stroke, D::OutputCommand>,
        output: &mut CommandDelta<D::OutputCommand>,
    ) {
        // Execute the commands and count the number out output commands
        let command_count = new.commands.into_iter().fold(0, |acc, command| {
            acc + self.execute(command, output) as usize
        });

        // Treat "empty" commands (mostly EngineCommands) as non-existent in terms of the stroke history
        if command_count > 0 {
            self.history
                .push_back(MatchedOutline::new(new.strokes, command_count));
        }
    }

    /// Helper function which executes a command and/or adds its instructions to the output.
    /// Returns whether the command was an OutputCommand that has been forwarded.
    fn execute(
        &mut self,
        command: Command<D::OutputCommand>,
        output: &mut CommandDelta<D::OutputCommand>,
    ) -> bool {
        match command {
            Command::Output(output_command) => {
                output.to_push.push(output_command);
                true
            }
            Command::Engine(EngineCommand::UndoPrevious) => {
                if let Some((instructions, _)) = self.pop() {
                    output.assimilate(instructions);
                }
                false
            }
        }
    }
}

use super::engine::Command;
use crate::constants::AVG_CMD_COUNT;
use core::future::Future;
use smallvec::SmallVec;

mod ext;
pub(crate) use ext::*;

pub(crate) mod binary;
pub use binary::BinaryDictionary;

pub type CommandList<OutputCommand> = SmallVec<[Command<OutputCommand>; AVG_CMD_COUNT]>;

pub trait Dictionary {
    type Stroke;
    type OutputCommand;
    type LookupFuture<'a>: Future<Output = Option<CommandList<Self::OutputCommand>>> + 'a
    where
        Self: 'a;

    fn lookup<'a>(&'a self, outline: &'a [Self::Stroke]) -> Self::LookupFuture<'_>;
    fn fallback_commands(&self, stroke: &Self::Stroke) -> CommandList<Self::OutputCommand>;
    fn longest_outline_length(&self) -> usize;
}

impl<D> Dictionary for &D
where
    D: Dictionary,
{
    type Stroke = D::Stroke;
    type OutputCommand = D::OutputCommand;
    type LookupFuture<'a> = D::LookupFuture<'a> where Self: 'a;

    fn lookup<'a>(&'a self, outline: &'a [Self::Stroke]) -> Self::LookupFuture<'a> {
        (*self).lookup(outline)
    }

    fn fallback_commands(&self, stroke: &Self::Stroke) -> CommandList<Self::OutputCommand> {
        (*self).fallback_commands(stroke)
    }

    fn longest_outline_length(&self) -> usize {
        (*self).longest_outline_length()
    }
}

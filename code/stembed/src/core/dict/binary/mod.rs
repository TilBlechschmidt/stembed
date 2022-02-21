use super::Dictionary;
use crate::{
    constants::{BINARY_DICT_PREAMBLE, FNV_HASH_KEY, HASH_TABLE_BUCKET_SIZE, HASH_TABLE_SIZE},
    core::{engine::Command, processor::text_formatter::TextOutputCommand, Stroke, StrokeContext},
    io::{self, Read, Seek, SeekFrom},
    serialize::{Deserialize, StringSerializationError},
};
use alloc::string::ToString;
use core::{
    cell::RefCell,
    hash::{Hash, Hasher},
};
use smallvec::smallvec;

mod entry;
pub use entry::*;

mod fnv;
use fnv::FnvHasher;

#[derive(Debug)]
pub enum BinaryDictionaryError {
    IOError(io::Error),
    InvalidPreamble,
    CorruptedStrokeContext(StringSerializationError),
}

pub struct BinaryDictionary<'d, D: Read + Seek> {
    data: RefCell<&'d mut D>,
    context: StrokeContext,
    table_offset: u64,
    data_offset: u64,
    longest_outline_length: u8,
}

impl<'d, D: Read + Seek> BinaryDictionary<'d, D> {
    pub fn new(data: &'d mut D) -> Result<Self, BinaryDictionaryError> {
        // Go to the beginning, just in case we are not there already
        data.seek(SeekFrom::Start(0))
            .map_err(BinaryDictionaryError::IOError)?;

        // Check the magic number and version
        for expected in BINARY_DICT_PREAMBLE {
            let read = data.read_u8().map_err(BinaryDictionaryError::IOError)?;
            if read != *expected {
                return Err(BinaryDictionaryError::InvalidPreamble);
            }
        }

        // Read the longest outline length
        let longest_outline_length = data.read_u8().map_err(BinaryDictionaryError::IOError)?;

        // Read the StrokeContext
        let context = StrokeContext::deserialize(data, ())
            .map_err(BinaryDictionaryError::CorruptedStrokeContext)?;

        // Calculate the location of the data section
        // (Hash table contains a 32-bit number for each bucket so we have to multiply to get a size in bytes)
        let table_offset = data
            .stream_position()
            .map_err(BinaryDictionaryError::IOError)?;
        let hash_table_size = (HASH_TABLE_SIZE * HASH_TABLE_BUCKET_SIZE) as u64;
        let data_offset = table_offset + hash_table_size;

        Ok(Self {
            data: RefCell::new(data),
            context,
            table_offset,
            data_offset,
            longest_outline_length,
        })
    }

    pub fn stroke_context(&self) -> &StrokeContext {
        &self.context
    }
}

impl<'d, D: Read + Seek> Dictionary for BinaryDictionary<'d, D> {
    type Stroke = Stroke<'d>;
    type OutputCommand = TextOutputCommand;

    fn lookup(&self, outline: &[Self::Stroke]) -> Option<super::CommandList<Self::OutputCommand>> {
        let mut data = self.data.borrow_mut();

        // Calculate the memory location of the bucket
        let bucket_index = calculate_bucket_index(outline);
        let bucket_offset = self.table_offset + (bucket_index * HASH_TABLE_BUCKET_SIZE) as u64;

        // Fetch the pointer into the data section
        data.seek(SeekFrom::Start(bucket_offset))
            .expect("seek failure during lookup");

        let data_offset =
            self.data_offset + data.read_u32().expect("read failure during lookup") as u64;

        // Run through the data section until we find what we are looking for
        data.seek(SeekFrom::Start(data_offset))
            .expect("seek failure during lookup");

        // Parse entries from our current position until we either reach EOF or the end of the current buckets collision list
        while let Ok(entry) = BinaryDictionaryEntry::deserialize(*data, &self.context) {
            // Check if we are still in the collision area for our initial bucket
            let entry_bucket_index = calculate_bucket_index(entry.outline());
            if entry_bucket_index != bucket_index {
                break;
            }

            // Check if we have found a matching stroke
            // TODO Add filtering by tag
            if &entry.outline()[..] == outline {
                return Some(entry.into_commands());
            }
        }

        None
    }

    fn fallback_commands(&self, stroke: &Self::Stroke) -> super::CommandList<Self::OutputCommand> {
        let formatted_stroke = stroke.to_string();
        let command = Command::Output(TextOutputCommand::Write(formatted_stroke));
        smallvec![command]
    }

    fn longest_outline_length(&self) -> usize {
        self.longest_outline_length as usize
    }
}

fn hash(outline: &[Stroke]) -> u64 {
    let mut state = FnvHasher::with_key(FNV_HASH_KEY);
    for stroke in outline {
        stroke.hash(&mut state);
    }
    state.finish()
}

pub(crate) fn calculate_bucket_index(outline: &[Stroke]) -> usize {
    let hash = hash(outline);
    (hash % HASH_TABLE_SIZE as u64) as usize
}

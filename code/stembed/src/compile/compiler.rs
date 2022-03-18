use crate::core::dict::binary::{
    calculate_bucket_index, BinaryDictionaryEntry, BinaryDictionaryEntryError, Outline,
};
use crate::io::{Write, WriteExt};
use crate::serialize::BinaryDictionaryEntrySerializationError;
use crate::{
    constants::{BINARY_DICT_PREAMBLE, HASH_TABLE_EMPTY_BUCKET, HASH_TABLE_SIZE},
    core::{dict::CommandList, processor::text_formatter::TextOutputCommand, StrokeContext},
    io::util::{CountingWriter, HeapFile},
    serialize::StringSerializationError,
};
use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt::{Debug, Display};

pub struct DictionaryStatistics {
    occupancy: BTreeMap<usize, usize>,
    stroke_length: BTreeMap<usize, usize>,
    collisions: usize,
    entries: usize,
    load: usize,
}

impl DictionaryStatistics {
    fn new() -> Self {
        Self {
            occupancy: BTreeMap::new(),
            stroke_length: BTreeMap::new(),
            collisions: 0,
            entries: 0,
            load: 0,
        }
    }

    /// Number of buckets with more than one entry
    pub fn collisions(&self) -> usize {
        self.collisions
    }

    /// Total number of entries in the dictionary
    pub fn entries(&self) -> usize {
        self.entries
    }

    /// Number of buckets in the hash table that contain at least one entry
    pub fn load(&self) -> usize {
        self.load
    }

    /// Ratio of filled bucket count over total bucket count
    pub fn load_factor(&self) -> f64 {
        self.load as f64 / HASH_TABLE_SIZE as f64
    }
}

impl Display for DictionaryStatistics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            r#"DictionaryStatistics {{
    entries: {}
    load: {} ({}%)
    collisions: {}
    occupancy:
"#,
            self.entries,
            self.load,
            self.load_factor() * 100.0,
            self.collisions,
        ))?;

        for (key, value) in self.occupancy.iter() {
            f.write_fmt(format_args!("      {} -> {}\n", key, value))?;
        }

        f.write_str("    stroke length:\n")?;

        for (key, value) in self.stroke_length.iter() {
            f.write_fmt(format_args!("      {} -> {}\n", key, value))?;
        }

        f.write_str("}}")
    }
}

pub struct BinaryDictionaryCompiler<'c> {
    context: &'c StrokeContext,
    stats: DictionaryStatistics,
    hash_table: Vec<Option<usize>>,
    buckets: Vec<Vec<BinaryDictionaryEntry<'c>>>,
    longest_outline_length: u8,
}

impl<'c> BinaryDictionaryCompiler<'c> {
    pub fn new(context: &'c StrokeContext) -> Self {
        Self {
            stats: DictionaryStatistics::new(),
            context,
            hash_table: core::iter::repeat(None).take(HASH_TABLE_SIZE).collect(),
            buckets: Vec::new(),
            longest_outline_length: 0,
        }
    }

    pub fn add(
        &mut self,
        outline: Outline<'c>,
        commands: CommandList<TextOutputCommand>,
        tag: u16,
    ) -> Result<(), BinaryDictionaryEntryError> {
        self.longest_outline_length = self.longest_outline_length.max(outline.len() as u8);
        self.stats
            .stroke_length
            .entry(outline.len())
            .and_modify(|x| *x += 1)
            .or_insert(1);

        let bucket_index = calculate_bucket_index(&outline);
        let entry = BinaryDictionaryEntry::new(tag, outline, commands)?;
        self.stats.entries += 1;

        match self.hash_table[bucket_index] {
            Some(bucket_address) => {
                // Hash collision => Append our entry to the existing bucket
                let bucket = self
                    .buckets
                    .get_mut(bucket_address)
                    .expect("attempted to fetch non-existent bucket during collision handling");

                bucket.push(entry);
                self.stats.collisions += 1;

                self.stats
                    .occupancy
                    .entry(bucket.len() - 1)
                    .and_modify(|v| *v -= 1);

                self.stats
                    .occupancy
                    .entry(bucket.len())
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
            }
            None => {
                // Allocate a new bucket and set its address
                let bucket_address = self.buckets.len();
                self.hash_table[bucket_index] = Some(bucket_address);
                self.buckets.push(vec![entry]);
                self.stats.load += 1;

                self.stats
                    .occupancy
                    .entry(1)
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
            }
        }

        Ok(())
    }

    pub fn stats(&self) -> &DictionaryStatistics {
        &self.stats
    }
}

#[derive(Debug)]
pub enum BinaryDictionarySerializationError {
    IOError(crate::io::Error),
    ContextUnserializable(StringSerializationError),
    EntryUnserializable(BinaryDictionaryEntrySerializationError),
}

impl<'c> BinaryDictionaryCompiler<'c> {
    pub async fn serialize(
        &self,
        writer: &mut impl crate::io::Write,
    ) -> Result<(), BinaryDictionarySerializationError> {
        // -- Begin by remapping some data and calculating offsets
        let mut writer = CountingWriter::new(writer);

        // Write the dictionary entries sorted by hash and build a map
        // from in-memory bucket indices (index in self.buckets) to
        // on-disk bucket offsets relative to the start of the bucket data area.
        let mut bucket_area = HeapFile::new();
        let mut bucket_offsets = [0u32; HASH_TABLE_SIZE];

        {
            let mut bucket_writer = CountingWriter::new(&mut bucket_area);
            for bucket_address in self.hash_table.iter().filter_map(|b| b.as_ref()) {
                let bucket = &self.buckets[*bucket_address];
                bucket_offsets[*bucket_address] = bucket_writer.position() as u32;

                for entry in bucket {
                    (*entry)
                        .serialize(&mut bucket_writer)
                        .await
                        .map_err(BinaryDictionarySerializationError::EntryUnserializable)?;
                }
            }
        }

        // Write the hash table, translating bucket indices to bucket offsets
        let mut hash_table = HeapFile::new();
        for bucket in self.hash_table.iter() {
            let bucket_pointer = match *bucket {
                Some(bucket_index) => bucket_offsets[bucket_index],
                None => HASH_TABLE_EMPTY_BUCKET,
            };

            hash_table
                .write_u32(bucket_pointer)
                .await
                .map_err(BinaryDictionarySerializationError::IOError)?;
        }

        // -- Start writing to the output file

        // Write the preamble
        for byte in BINARY_DICT_PREAMBLE {
            writer
                .write(*byte)
                .await
                .map_err(BinaryDictionarySerializationError::IOError)?;
        }

        println!("Preamble: {}", writer.position());

        // Write the longest stroke length
        writer
            .write(self.longest_outline_length)
            .await
            .map_err(BinaryDictionarySerializationError::IOError)?;

        println!("Strokelen: {}", writer.position());

        // Write the StrokeContext
        self.context
            .serialize(&mut writer)
            .await
            .map_err(BinaryDictionarySerializationError::ContextUnserializable)?;

        println!("StrokeCon: {}", writer.position());

        // Write the hash table
        let hash_table_data = hash_table.into_inner();
        assert_eq!(
            hash_table_data.len(),
            HASH_TABLE_SIZE * u32::BITS as usize / 8
        );

        for byte in hash_table_data {
            writer
                .write(byte)
                .await
                .map_err(BinaryDictionarySerializationError::IOError)?;
        }

        println!("HashTbl: {}", writer.position());

        // Write the bucket data
        let bucket_area_data = bucket_area.into_inner();
        for byte in bucket_area_data {
            writer
                .write(byte)
                .await
                .map_err(BinaryDictionarySerializationError::IOError)?;
        }
        println!("DataBlob: {}", writer.position());

        Ok(())
    }
}

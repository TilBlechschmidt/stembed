use defmt::Format;

use crate::{ClusterID, BLOCK_SIZE};

const ENTRY_TYPE_LONG_NAME_MASK: u8 = 0b00001111;
const ENTRY_TYPE_DIRECTORY_MASK: u8 = 0b00010000;
const ENTRY_TYPE_VOLUME_ID_MASK: u8 = 0b00001000;
pub(crate) const DIRECTORY_ENTRY_SIZE: usize = 32;
pub(crate) const DIRECTORY_ENTRIES_PER_BLOCK: usize = BLOCK_SIZE / DIRECTORY_ENTRY_SIZE;

#[derive(Format, PartialEq)]
pub struct Name([u8; 11]);

#[derive(Format, Debug, PartialEq)]
pub struct File {
    name: Name,
    attributes: u8,
    cluster: ClusterID,
    size: u32,
}

#[derive(Format, Debug, PartialEq)]
pub struct Directory {
    pub(crate) name: Name,
    pub(crate) cluster: ClusterID,
}

#[derive(Format, Debug, PartialEq)]
pub enum DirectoryEntry {
    Directory(Directory),
    File(File),
    VolumeID,
    LFN,
}

#[derive(Debug, PartialEq)]
pub enum DirectoryIndexEntry {
    Entry(DirectoryEntry),
    Unused,
    EndOfDirectory,
}

impl Name {
    pub(crate) fn new(data: [u8; 11]) -> Self {
        Self(data)
    }

    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        // TODO Add support for long filenames
        // TODO Drop spaces (0x20) at the end of the slice (use a subslice for the returned string)
        // TODO Support extensions and stuff ... we probably need a dedicated Path struct for handling this mess
        core::str::from_utf8(&self.0[..8]).map(str::trim_end)
    }

    pub fn extension(&self) -> Result<&str, core::str::Utf8Error> {
        // TODO Add support for long filenames
        // TODO Drop spaces (0x20) at the end of the slice (use a subslice for the returned string)
        // TODO Support extensions and stuff ... we probably need a dedicated Path struct for handling this mess
        core::str::from_utf8(&self.0[8..])
    }
}

impl File {
    pub fn cluster_address(&self) -> ClusterID {
        self.cluster
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn name(&self) -> &Name {
        &self.name
    }
}

impl core::fmt::Debug for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;

        match core::str::from_utf8(&self.0) {
            Ok(data) => {
                f.write_char('\"')?;
                f.write_str(&data[..8].trim_end())?;
                if !data[8..].trim_end().is_empty() {
                    f.write_char('.')?;
                    f.write_str(&data[8..])?;
                }
                f.write_char('\"')
            }
            Err(error) => f.write_fmt(format_args!("{:?}", error)),
        }
    }
}

impl From<&[u8]> for DirectoryIndexEntry {
    fn from(data: &[u8]) -> Self {
        assert_eq!(data.len(), DIRECTORY_ENTRY_SIZE);

        if data[0] == 0xE5 {
            DirectoryIndexEntry::Unused
        } else if data[0] == 0x00 {
            DirectoryIndexEntry::EndOfDirectory
        } else {
            let mut raw_name = [0u8; 11];
            raw_name.copy_from_slice(&data[..11]);
            let name = Name(raw_name);

            let attributes = data[0x0B];

            let cluster = ClusterID(u32::from_le_bytes([
                data[0x1A], data[0x1B], data[0x14], data[0x15],
            ]));

            let size = u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]);

            let entry = if attributes & ENTRY_TYPE_LONG_NAME_MASK > 1 {
                DirectoryEntry::LFN
            } else if attributes & ENTRY_TYPE_VOLUME_ID_MASK > 1 {
                DirectoryEntry::VolumeID
            } else if attributes & ENTRY_TYPE_DIRECTORY_MASK > 1 {
                DirectoryEntry::Directory(Directory { name, cluster })
            } else {
                DirectoryEntry::File(File {
                    name,
                    attributes,
                    cluster,
                    size,
                })
            };

            DirectoryIndexEntry::Entry(entry)
        }
    }
}

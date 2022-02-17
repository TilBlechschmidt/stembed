pub const HISTORY_SIZE: usize = 100;
pub const AVG_CMD_COUNT: usize = 2;
pub const AVG_STROKE_COUNT: usize = 2;
pub const AVG_STROKE_BIT_COUNT: usize = 24;
/// Average number of outlines that will fit into a number of strokes equal to twice the longest outline
pub const AVG_OUTLINE_RATIO: usize = 10;
pub const AVG_OUTPUT_INSTRUCTIONS: usize = 4;

pub const FNV_HASH_KEY: u64 = 0xcbf29ce484222325;
pub const HASH_TABLE_SIZE: usize = 125_000;
pub const HASH_TABLE_BUCKET_SIZE: usize = (u32::BITS / u8::BITS) as usize;
pub const HASH_TABLE_EMPTY_BUCKET: u32 = u32::MAX;

pub const BINARY_DICT_PREAMBLE: &[u8] = b"stembedDict1";

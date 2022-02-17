## Conceptual overview
- Operational dictionary represented by a compiled binary representation
    - Can be extended by a reverse-lookup index
    - Contains all the necessary information for conversion back into JSON/RTF/whatev

- Bound to a specific environment (see [Stroke format](./Stroke format.md))
- Each stroke is defined by a 32-bit integer, translations given as 16-bit length-prefixed unicode strings
- Lookup supported by a HashMap located at the beginning of the dictionary
    - Collisions handled by separate chaining (linked list for each entry)

## Binary format
- Reference section
    - 32-bit offset of environment vector
    - 32-bit offset of lookup hashmap
    - 32-bit offset of entry data
    - 16-bit word list identifier
    - 8-bit max strokes per outline
    - TODO: Some hash identifying the dictionary/making it comparable?
- Data section
    - Environment vector
        - Vector of 8-bit length-prefixed key identifiers
        - Must be a subset of the steno engines active environment
    - Lookup hashmap array
        - 32-bit array element count
        - Array of 32-bit offsets relative to the start of the entry data section
            - UINT32::MAX == no value in this bucket
    - Entry data
        - Contains translations for each dictionary entry
        - Each entry is comprised of multiple parts
            - 32-bit offset for linked-list hashmap collision handling
                - UINT32::MAX == no further value
            - 8-bit outline vector element count
            - 16-bit outline value length
            - Outline vector (32-bit unsigned integer array)
            - Outline value (variable length unicode string)
                - Will be replaced by coded version of engine commands later on

## Ideas
- Instead of adding multiple dict data structures, tag each entry
- Reference count entry data
    - Deduplication + editing

I am slowly getting into stenography and learning how to type any word I could possibly imagine. Funnily enough it is getting easier and easier to come up with new words even though I never encountered the outline! Not going to lie, this is pretty cool wink emoji



Everything below assumes big-endian integers

## Bitwise encoding of commands

First byte encodes all the variants, further bytes contain additional data.

```
Command
0 EngineCommand
    No further data
1 TextOutputCommand
    000 Write
        4+8-bit text length + UTF-8 byte sequence
    001 ChangeDelimiter
        4-bit padding + 4-byte char primitive
    010 ChangeCapitalization
        000 None
        001 Lowercase
        010 Capitalize
        011 Uppercase
        100 LowerThenCapitalize
        101 LowercaseNext
        110 CapitalizeNext
        111 UppercaseNext
    011 ChangeAttachment
        00 Delimited
        01 Glue
        10 Next
        11 Always
    111 ResetFormatting
        No further data
```

## Bytewise encoding of dictionary entries

```
BinaryDictionaryEntry
- 16-bit entry information
    - 5-bit dictionary tag  (up to 32)
    - 5-bit stroke count    (up to 32)
    - 6-bit command count   (up to 64)
- <byte-aligned serialized strokes back-to-back>
- <byte-aligned serialized commands back-to-back>
```

## Binary structure of the dictionary

- Preamble
    - Magic identifier
    - Format version number
- LongestOutlineLength (u8)
- StrokeContext
- Hash table
    - List of HASH_TABLE_SIZE * 32-bit pointers into the data area
- Bucket area
    - Sequentially stored DictionaryEntries
    - Sorted by hash value (buckets are continous memory sequences)

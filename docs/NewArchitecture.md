# Dictionary compilation

## Goals & Non-Goals

- Use as little memory and CPU as possible, potentially at the cost of more flash utilization
    - Random writes / rewrites are considered okay as a tradeoff for reduced MCU requirements
- Should only parse the input file once
    - Size estimations may be required for hash table allocation
    - JSON dictionary size can be approximated through heuristics over file size
- Output file should not contain duplicated data if avoidable
    - Command deduplication shall be employed
    - Reverse lookup & forward lookup hash buckets should reference same data
- No internal or external fragmentation post compilation
    - During compilation, this is an accepted trade-off
    - Final result may be copied together
    - Data locality matters
        - Bucket content linked-list should be copied together when writing final data structure
        - Allows prefixing with length in final hash table to allow a single continous read
- Reverse lookup hash table may not be required for normal operation
    - As long as nobody attempts to do reverse search
    - Flashing the dictionary into constrained-space environments by leaving this out
- All input dictionaries will be compiled into the same data structure
    - Differentiation through internal tagging
    - Disambiguation through closed addressing & linked-lists
- Ensure safety
    - Entry conflicts should throw at least a warning at compile time

- Full cache or bust
    - There is no point in partially loading data into memory
    - Hash tables w/o buckets might fit but they are the least read intensive part
    - Dynamically sized structures (buckets, command data) would yield largest benefit







# Engine stuff

- Strokes are recorded fully
- Outlines are represented through stroke count
    - Derivation of outlines is thus possible by taking a slice out of the stroke stack
- Commands are not stored (could be derived again but usually not needed)
- State of the command processor is stored on its own stack
    - Outlines contain a command count to allow for undo operations
- Whole state is encapsulated in its own struct which can be exchanged if needed
    - Modifications only allowed through API to ensure consistency of stacks
- The command processor may indicate that a command can not be undone
    - State contains an undo-barrier for each stack
    - When processing new inputs, only stack content after the barrier is considered

TL;DR We have three stacks:
1. Stroke stack `Vec<Stroke>` + `UndoBarrier`
    - One entry per physical stroke
2. Outline stack `Vec<(OutlineLength, CommandCount)>` + `UndoBarrier`
    - One per longest matched outline
3. State stack `Vec<(State, UndoInformation)>` + `UndoBarrier`
    - One per command produced by an outline
    - UndoInformation contains everything required to undo a previously issued command (num of chars should suffice)

- Stroke = u32
- OutlineLength = u8
- CommandCount = u8
- State = ?
- UndoInformation = u16
- UndoBarrier = usize

Question remains: How to deal with orthography which requires knowledge of previous output?
=> Probably just fetch/re-calculate the previous outlines output on-demand
=> OutlineMatcher may implement caching

```
VecDeque<Outline>
=> Outline {
    strokes: SmallVec<[Stroke; 2]>,
    command_count: u8,
    undoable: bool,
}

enum Command {
    Output(OutputCommand),
    Engine(EngineCommand),
}

enum EngineCommand {
    UndoPrevious,
    PrefixPrevious(Vec<OutputCommand>),
}

struct OutputInstruction {
    to_undo: usize,
    to_push: SmallVec<[OutputCommand; 4]>
}
```

- Separate the output command processor
- Each operation returns a number of output commands to undo and apply
    - Generalized into `OutputInstruction` struct
- 

https://lib.rs/crates/smol_str
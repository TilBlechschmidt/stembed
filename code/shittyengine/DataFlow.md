# Stenography engine

## Data flow

1. Input is scanned
2. Stroke is "accumulated" (max over keypresses)
3. Preprocessor hooks executed for stroke
    - Can capture stroke and prevent further propagation
    - Useful for undo, system commands (e.g. enable BT), or similar
4. Preprocessor side-effects are executed
    - Operations like undo may cause outlines to be removed
    - Undo these respective outlines
5. Stroke is added to stenography state using StateMutator
6. StateResolver is executed repeatedly to commit pending strokes
    1. Dictionary lookup performed to find longest matching prefix in uncommitted strokes
        - If no match can be found, a fallback match will be generated (usually just printing the stroke)
    2. Undo previously applied outlines if applicable
        - If the outline does not match a previously matched outline at the same position, it and each following outline is no longer valid. Thus they need to be undone before the new outline can be applied.
        - When an undo is pending, the StateResolver marks the commit as unsuccessful and instead returns an outline to undo
        - Outlines are undone by fetching their associated commands and backpropagating the formatter state, modifying the output accordingly
    3. Upon successfully committing the stroke, apply its commands to the formatter
        - Formatter does not keep a history of strokes, it just propagates them forward/backward and generates output
        - Orthography is implemented by indexing all possible prefixes that match
            - When undoing an orthographically-attached suffix, the previous outline is fetched to reconstruct the index and thus gain an understanding on what to undo

## TODO

- Name suggestion "wren"
    - https://en.wikipedia.org/wiki/Eurasian_wren
    - Smallest bird in Europe (almost)
    - Very fast staccato song
        - https://en.wikipedia.org/wiki/File:Eurasian_Wren_(Troglodytes_troglodytes)_(W1CDR0001470_BD11).ogg
    - Easy for making jokes about the wrengine

### Important

- [x] Evaluate whether storing the number of commands associated with an outline in the core state would make sense
    - Could save a round-trip when undoing and since the formatter does not necessarily need the exact commands to undo but just the count ...
- [x] Write tests for the formatter
    - [x] AttachmentMode
    - [x] CapitalizationMode
    - [x] Formatter integrated test

### Progress

- [x] History buffer
- [x] State container
- [x] Mutator
- [x] Resolver
- [x] Dictionary trait
    - [x] BTreeMap based impl
- [x] Command enum (contains every formatting/output instruction)
- [x] TextFormatter
    - Think about orthography integration in more detail, maybe implement commands as a "plugin" system where orthography is just yet-another-plugin (definitely not)?
- [x] OutputDriver
    - [x] Stub based on String for testing
    - [x] macOS Cocoa (autopilot based)
- [ ] Compiler
    - [ ] Parse plover JSON
    - [ ] Build Stroke datatype (3 bytes, statically assigned during compilation, fixed in the engine)
    - [ ] Figure out a dictionary data structure
    - [ ] Implement serialization
    - [ ] Implement deserialization

### TechDebt

- [ ] Redo state indexing with an enum that is based on offsets relative to the committed/uncommitted area (improves code readability)
- [ ] Document stenography state modus operandi and how the matching works
- [ ] Formatter tests are pretty lax

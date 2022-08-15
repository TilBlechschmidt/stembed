# Detailed TODOs
- Add diags output to `ExecutionQueue::optimize`
- Have derive macro for queue generate const order-checking code
- Reverse order of values in ValueSet (because otherwise its *really* unintuitive ...)

# High-level TODOs
- Add remaining docs
    - `Executor`
    - High-level explanation of core concepts in crate root
- Build serialization/deserialization API for embedded/desktop
    - Behind `serde` feature flag, make the algorithm itself exchangeable!
    - "Serialize" on embedded by either transmuting into byte slices or just straight up storing raw pointer+len and "forgetting" the memory temporarily in regards to the borrow checker
- Build stack debugging tools <3
- Build supporting crate for inputs (stabg-input?)
    - `InputProcessor` for desktop
    - `make_input_processor` macro for embedded
- Develop a concept of `ProcessorHost`s for desktop
    - Hosts load clients
    - Clients register processors
    - Supporting crate, WASM specific (stabg-wasm?)
        - `ProcessorHost` impl based on wasmer
        - Some kind of client configuration, parsed for and passed into processors
        - Granular permission/capability system to allow only required stuff (duh)
            - Maybe with an auth middleware that is called each time a processor is loaded
            - On first load, it asks the user — subsequent loads use cached permissions
        - Communication via stdin/stdout
        - Client-side SDK

# Proposed features & ideas
- Classes of processor ordering diagnostics
    - Output XYZ will never be used
    - Will never be executed because inputs can't be available
        - Analyse the dependency graph of all broken processors to detect transitive & circular dependencies
- API features
    - Some sort of finalizer i.e. "this is the final execution"?
        - Problematic because it breaks the paradigm
        - Plugins should instead be of a streaming nature
        - Instead, provide cycle functions like "pre_cycle_start" so that a plugin can know when one batch of execs has finished

# Type implementations & explanations

## Registry ✅
- Associates `&'static str` with `u8` for the runtime duration
- Used for more efficient storage in temporary data structures
- Trait for stack & heap based impls

## TypeRegistry ✅
- Based on Registry
- Plugins can register types, associating a string identifier `core.stroke` with a numeric identifier that is valid for the runtime of the application
- Used when storing/retrieving values from the stack to translate from binary type codes to string type codes
- Non-registered types can not be stored or retrieved! Results in Err()

## ProcessorRegistry ✅
- Implicitly defined through processor order in ExecutionQueue instead of having an explicit Registry
- Each processor will be registered before execution
- Used for creating processor annotations
- Aids in reverse lookup of value creator

## Stack (LIFO) ✅
- Stores & retrieves slices with associated type codes
- Basic building block to allow both non-alloc and alloc impls
- Very simple interface
    - `push(type_code, data)`
    - `pop() -> (type_code, data)`
    - `get(type_code) -> data`
    - `iter()` (follows pop order)

## ExecutionContext ✅
- Wraps `Stack`
- Created individually for each processor invocation
    - Passed over to processor either directly or through FFI/WASM host fns
- ValueSets
    - Branching primitive: All remaining processors are executed for each value
    - Just a `u8` for the number of contained values, works like a bracket
    - Always the last thing pushed by a plugin
        - Otherwise it would be unclear what should happen with the values after it
        - In Rust plugins, the context is consumed!
    - Yet-another-type™ to the StackAllocator
        - Registered in TypeRegistry like any other type!
- Processor annotations
    - Automatically pushed upon drop of context
    - Used to determine until when a processor was run
        - Injected after each ValueSet by default
        - Value ownership can be deduced for debugging purposes by always injecting them when switching processors
            - By always injecting them, it gives plugins the power of asking "who injected the last value of this type". Worth the overhead?
            - Mode can be set upon creation of the context, by default disabled on embedded, enabled on desktop

## InitializationContext ✅
- Passed when a plugin is first instantiated
- Requires the plugin to define used input/output types
	- Allows the plugin to register new types
- Additional side-effects (like spawning threads) might occur
	- They should be cleaned up when the destroy fn is called!

## Processor / EmbeddedProcessor ✅
- Two traits to differentiate needs and make embedded support opt-in
	- `impl<T> Processor for T where T: EmbeddedProcessor + AutoImplProcessor`
- Embedded processor expose more details about themselves `const`
    - Maximum stack usage
    - No explicit registering of types required
        - Input/Output types are provided through constants
- Processors register types at runtime and do not need to provide a stack usage estimate
- Serialization algorithm is dependent on context
    - Embedded => postcard (plugins are Rust only anyways so we might as well)
    - Desktop => json (best interop and ease of impl for other languages)
    - Client API should abstract this away!
        - These traits should probably be called `RawProcessor` or smth like that with `Processor` being reserved for the dynamic dispatch
- Execution mode is dependent on platform
    - Embedded uses async to not block other tasks (e.g. BLE, USB)
        - Not possible to build an array of processors, thus a macro is used. See `ProcessorCollection` docs!
    - Desktop uses blocking invocation
        - Simpler code & thought process, easier for beginners
        - No runtime required, no nightly feature gates
        - Async not possible with WASM or FFI (within reason)
        - Threads allow offloading of execution to not block other workloads
            - It is one linear execution path, no concurrency anyways
            - Thread is spawned once and then lives forever, nothing lost by not going async

## ProcessorCollection ✅
- Tries to find a sensible order for processors, based on input/output specs
    - "Stable sort" so that input order can define preferences
    - Returns warnings if a processor will never get the values it wants or circular dependencies exist
- Only on desktop, no array can be built from async traits as they are not object-safe

## ExecutionQueue ✅ / AsyncExecutionQueue ✅
- Basically a `fn` which runs all processors
	- Gets `&mut stack, &registry` and builds ExecutionContext instances internally for each processor
- Can skip processors / start at a given processor based on its ID
- Dynamically defined on desktop based on `ProcessorCollection::build`
- Statically defined on embedded through a macro `make_async_processor_stack![p1, p2, p3]`
    - The macro does the verifications described in the processor collection internally and at compile-time
    - Likely no reordering possible, compiler warning will be emitted instead

## Executor ✅
- Takes a `Stack`, and a `ExecutionQueue`
    - TODO What about input sources?
	    - TL;DR They don't exist. Not on this crates' level.
	    - All inputs will be hidden behind one processors that pushes values onto the stack initially
	    - This processor gets special handling
	    - Plugins can register inputs which this processor will race internally
	    - It will then spawn threads on desktop and use select on embedded
- Creates a value stack, runs the execution queue
- Branches execution based on ValueSets
    - Runs all remaining processors
    - Reverts the stack to the previous state
    - Removes the top-most value from the set
    - If it was the last value
        - Search for the next ValueSet
        - If there is none, execution finishes
        - If these is some, repeat from the top
    - Otherwise, repeat from the top
- Uses Processor annotations to determine what to execute post revert

## StackDebugger
- Primary job: Capture & visualize the stack contents
- Annotates regions with type & ownership
- Shows ValueSet brackets
- Stack frames exported by runtime after each processor invocation, dumped to disk, later read by viewer
	- Registries have to be dumped as well to make sense of it
- Potential for interactive debugging in the future

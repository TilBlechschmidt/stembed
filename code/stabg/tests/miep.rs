use stabg::{
    context::ExecutionContext,
    processor::{InitializationContext, InitializationError, Processor, TypeUsage::*},
    registry::FixedSizeRegistry,
    stack::FixedSizeStack,
    Executor, Identifiable, Identifier, ProcessorCollection,
};

struct TestType1(u8);
struct TestType2(u8);

impl Identifiable for TestType1 {
    const IDENTIFIER: Identifier = "com.example.type.test1";
}

impl Identifiable for TestType2 {
    const IDENTIFIER: Identifier = "com.example.type.test2";
}

struct TestProcessor1;
struct TestProcessor2;

#[test]
fn something() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Build the data storage structures
    let mut stack = FixedSizeStack::<256>::new();
    let mut registry = FixedSizeRegistry::<16>::default();

    // Build an execution stack
    let mut processors = ProcessorCollection::new(&mut registry);
    processors.push(Box::new(TestProcessor2)).unwrap();
    processors.push(Box::new(TestProcessor1)).unwrap();
    // Note: They are specified in reverse order, but due to their input/output dependency
    //          the ::build() command reorders them so they work properly!
    let mut execution_queue = processors.build();

    // Construct an executor, give it the necessary things, and run it all :)
    let mut executor = Executor::new(&mut stack, &mut registry);

    loop {
        executor.execute_sync(&mut execution_queue);
        break; // this wouldn't be there in a real scenario
    }
}

// ————————————— Implementations of processors below —————————————

impl Processor for TestProcessor1 {
    fn identifier(&self) -> Identifier {
        "com.example.processor.test1"
    }

    fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError> {
        context
            .register::<TestType1>(Output)?
            .register::<TestType2>(Output)?;

        Ok(())
    }

    fn process(&mut self, mut context: ExecutionContext) {
        // Currently, there is no strongly typed APIs to push/get values. There will be some, they should become the default!
        //
        // This is an open question at the momeny because the serialization method is dependent on the context (i.e. embedded or desktop)
        // The serialization algorithm might become a generic arg to the context, or maybe it will be hidden internally.
        context
            .push(TestType1::IDENTIFIER, &[42])
            .unwrap()
            .push(TestType2::IDENTIFIER, &[69])
            .unwrap();
    }
}

impl Processor for TestProcessor2 {
    fn identifier(&self) -> Identifier {
        "com.example.processor.test2"
    }

    fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError> {
        context
            .register::<TestType1>(Input)?
            .register::<TestType2>(Input)?;

        Ok(())
    }

    fn process(&mut self, context: ExecutionContext) {
        assert_eq!(context.get(TestType1::IDENTIFIER).unwrap()[0], 42);
        assert_eq!(context.get(TestType2::IDENTIFIER).unwrap()[0], 69);
    }
}

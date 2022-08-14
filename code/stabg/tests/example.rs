use stabg::{
    processor::{TypeUsage::*, *},
    *,
};

#[derive(Identifiable)]
#[identifiable(name = "test.type", version = "1")]
struct TestType1(u8);

#[derive(Identifiable)]
#[identifiable(name = "test.type", version = "2")]
struct TestType2(u8);

struct TestProcessor1;
struct TestProcessor2;

#[test]
fn full_stack_example() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Build the data storage structures
    let mut stack = FixedSizeStack::<256>::new();
    let mut executor = Executor::new(&mut stack);

    // Build an execution stack
    let mut queue = DynamicExecutionQueue::new();

    // Schedule some processors
    // Note: They are specified in reverse order, but due to their input/output dependency
    //          the ::build() command reorders them so they work properly!
    queue
        .schedule(TestProcessor2)
        .unwrap()
        .schedule(TestProcessor1)
        .unwrap()
        .optimize();

    // Run it in an "infinite" loop :)
    loop {
        executor.execute_sync(&mut queue).unwrap();
        break; // this wouldn't be there in a real scenario
    }
}

// ————————————— Implementations of processors below —————————————

impl Processor for TestProcessor1 {
    fn identifier(&self) -> Identifier {
        "test.processor1"
    }

    fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError> {
        context
            .register::<TestType1>(Output)?
            .register::<TestType2>(Output)?;

        Ok(())
    }

    fn process(&mut self, mut context: ExecutionContext) -> Result<(), ExecutionError> {
        // Currently, there is no strongly typed APIs to push/get values. There will be some, they should become the default!
        //
        // This is an open question at the momeny because the serialization method is dependent on the context (i.e. embedded or desktop)
        // The serialization algorithm might become a generic arg to the context, or maybe it will be hidden internally.
        context
            .push(TestType1::IDENTIFIER, &[42])?
            .push(TestType2::IDENTIFIER, &[69])?;

        Ok(())
    }
}

impl Processor for TestProcessor2 {
    fn identifier(&self) -> Identifier {
        "test.processor2"
    }

    fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError> {
        context
            .register::<TestType1>(Input)?
            .register::<TestType2>(Input)?;

        Ok(())
    }

    fn process(&mut self, context: ExecutionContext) -> Result<(), ExecutionError> {
        assert_eq!(context.get(TestType1::IDENTIFIER)?[0], 42);
        assert_eq!(context.get(TestType2::IDENTIFIER)?[0], 69);

        Ok(())
    }
}

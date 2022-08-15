#[cfg(all(feature = "alloc", feature = "derive"))]
mod alloc {
    use serde::{Deserialize, Serialize};
    use stabg::{
        processor::{TypeUsage::*, *},
        *,
    };

    #[derive(Identifiable, Serialize, Deserialize, PartialEq, Debug)]
    #[identifier(name = "test.type", version = "1")]
    struct TestType1(u8);

    #[derive(Identifiable, Serialize, Deserialize, PartialEq, Debug)]
    #[identifier(name = "test.type", version = "2")]
    struct TestType2(u8);

    struct TestProcessor1;
    struct TestProcessor2;

    #[test]
    fn full_stack_example() {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut stack = DynamicStack::new();
        let mut executor = Executor::new(&mut stack);
        let mut queue = DynamicExecutionQueue::new();

        // Note: They are specified in reverse order, but due to their input/output dependency
        //          the ::optimize() command reorders them so they work properly!
        queue
            .schedule(TestProcessor2)
            .unwrap()
            .schedule(TestProcessor1)
            .unwrap()
            .optimize();

        loop {
            executor.execute_sync(&mut queue).unwrap();
            break;
        }
    }

    impl Processor for TestProcessor1 {
        fn identifier(&self) -> Identifier {
            "test.processor1"
        }

        fn load(&mut self, ctx: &mut InitializationContext) -> Result<(), String> {
            ctx.register::<TestType1>(Output)
                .register::<TestType2>(Output);

            Ok(())
        }

        fn process(&mut self, mut ctx: ExecutionContext) -> Result<(), ExecutionError> {
            // Currently, there is no strongly typed APIs to push/get values. There will be some, they should become the default!
            //
            // This is an open question at the momeny because the serialization method is dependent on the context (i.e. embedded or desktop)
            // The serialization algorithm might become a generic arg to the context, or maybe it will be hidden internally.
            ctx.push(TestType1(42))?.push(TestType2(69))?;

            Ok(())
        }
    }

    impl Processor for TestProcessor2 {
        fn identifier(&self) -> Identifier {
            "test.processor2"
        }

        fn load(&mut self, ctx: &mut InitializationContext) -> Result<(), String> {
            ctx.register::<TestType1>(Input)
                .register::<TestType2>(Input);

            Ok(())
        }

        fn process(&mut self, ctx: ExecutionContext) -> Result<(), ExecutionError> {
            assert_eq!(ctx.get::<TestType1>()?, TestType1(42));
            assert_eq!(ctx.get::<TestType2>()?, TestType2(69));

            Ok(())
        }
    }
}

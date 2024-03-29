#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![no_std]

#[cfg(all(feature = "nightly", feature = "derive"))]
mod does {
    use serde::{Deserialize, Serialize};
    use stabg::{
        processor::{
            EmbeddedExecutionContext as Context, EmbeddedExecutionError as Error, EmbeddedProcessor,
        },
        *,
    };

    #[derive(Identifiable, Serialize, Deserialize, PartialEq, Debug)]
    #[identifier(name = "test.type", version = "1")]
    struct TestType1(u8);

    #[derive(Identifiable, Serialize, Deserialize, PartialEq, Debug)]
    #[identifier(name = "test.type", version = "2")]
    struct TestType2(u16);

    #[derive(Default, EmbeddedProcessor)]
    #[stack_usage(items = 2)]
    #[type_usage(outputs(TestType1, TestType2))]
    #[skip_phase(load, unload)]
    struct TestProcessor1;

    #[derive(Default, EmbeddedProcessor)]
    #[type_usage(inputs(TestType1, TestType2))]
    #[skip_phase(load, unload)]
    struct TestProcessor2;

    #[derive(Default, AsyncExecutionQueue)]
    struct EmbeddedExecutionQueue {
        input: TestProcessor1,
        output: TestProcessor2,
    }

    #[test]
    fn async_full_stack_example() {
        futures::executor::block_on(async move {
            let _ = env_logger::builder().is_test(true).try_init();

            let mut queue = EmbeddedExecutionQueue::default();
            let mut stack = FixedSizeStack::<{ EmbeddedExecutionQueue::STACK_USAGE }>::new();
            let mut executor = Executor::new(&mut stack);

            loop {
                executor.execute_async(&mut queue).await.unwrap();
                break;
            }
        });
    }

    impl TestProcessor1 {
        async fn process(&mut self, mut ctx: Context<'_, '_>) -> Result<(), Error> {
            ctx.push(TestType1(42))?;
            ctx.push(TestType2(69))?;
            Ok(())
        }
    }

    impl TestProcessor2 {
        async fn process(&mut self, ctx: Context<'_, '_>) -> Result<(), Error> {
            assert_eq!(ctx.get::<TestType1>()?, TestType1(42));
            assert_eq!(ctx.get::<TestType2>()?, TestType2(69));
            Ok(())
        }
    }
}

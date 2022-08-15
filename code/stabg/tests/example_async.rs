#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![no_std]

use stabg::{processor::*, *};

#[derive(Identifiable)]
#[identifier(name = "test.type", version = "1")]
struct TestType1(u8);

#[derive(Identifiable)]
#[identifier(name = "test.type", version = "2")]
struct TestType2(u8);

#[derive(Default, EmbeddedProcessor)]
#[stack_usage(items = 2, bytes = 2)]
#[type_usage(outputs(TestType1, TestType2))]
struct TestProcessor1;

#[derive(Default, EmbeddedProcessor)]
#[type_usage(inputs(TestType1, TestType2))]
struct TestProcessor2;

#[derive(Default, AsyncExecutionQueue)]
struct EmbeddedExecutionQueue {
    proc1: TestProcessor1,
    proc2: TestProcessor2,
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
    async fn load(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    async fn process(&mut self, mut ctx: ExecutionContext<'_, '_>) -> Result<(), ExecutionError> {
        ctx.push(TestType1::IDENTIFIER, &[42])?
            .push(TestType2::IDENTIFIER, &[69])?;
        Ok(())
    }

    async fn unload(&mut self) {}
}

impl TestProcessor2 {
    async fn load(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    async fn process(&mut self, ctx: ExecutionContext<'_, '_>) -> Result<(), ExecutionError> {
        assert_eq!(ctx.get(TestType1::IDENTIFIER)?[0], 42);
        assert_eq!(ctx.get(TestType2::IDENTIFIER)?[0], 69);
        Ok(())
    }

    async fn unload(&mut self) {}
}

#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

use core::future::Future;
use stabg::{processor::*, *};

#[derive(Identifiable)]
#[identifier(name = "test.type", version = "1")]
struct TestType1(u8);

#[derive(Identifiable)]
#[identifier(name = "test.type", version = "2")]
struct TestType2(u8);

#[derive(Default)]
struct TestProcessor1;

#[derive(Default)]
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

        // Build the data structures
        let mut stack = FixedSizeStack::<{ EmbeddedExecutionQueue::STACK_USAGE + 10 * 2 }>::new();
        let mut executor = Executor::new(&mut stack);
        let mut queue = EmbeddedExecutionQueue::default();

        // Run the whole thing :)
        loop {
            executor.execute_async(&mut queue).await.unwrap();
            break; // this wouldn't be there in a real scenario
        }
    });
}

// ————————————— Implementations of processors below —————————————

impl EmbeddedProcessor for TestProcessor1 {
    const TYPES_INPUT: &'static [Identifier] = &[];
    const TYPES_OUTPUT: &'static [Identifier] = &[TestType1::IDENTIFIER, TestType2::IDENTIFIER];
    const STACK_USAGE: usize = 2 + FixedSizeStack::<0>::OVERHEAD * 2;

    type Fut<'s> = impl Future<Output = Result<(), ExecutionError>> + 's
    where
        Self: 's;

    fn process<'s>(&'s mut self, mut context: ExecutionContext<'s, 's>) -> Self::Fut<'s> {
        async move {
            context
                .push(TestType1::IDENTIFIER, &[42])?
                .push(TestType2::IDENTIFIER, &[69])?;

            Ok(())
        }
    }
}

impl EmbeddedProcessor for TestProcessor2 {
    const TYPES_INPUT: &'static [Identifier] = &[TestType1::IDENTIFIER, TestType2::IDENTIFIER];
    const TYPES_OUTPUT: &'static [Identifier] = &[];
    const STACK_USAGE: usize = 0;

    type Fut<'s> = impl Future<Output = Result<(), ExecutionError>> + 's
    where
        Self: 's;

    fn process<'s>(&'s mut self, context: ExecutionContext<'s, 's>) -> Self::Fut<'s> {
        async move {
            assert_eq!(context.get(TestType1::IDENTIFIER)?[0], 42);
            assert_eq!(context.get(TestType2::IDENTIFIER)?[0], 69);

            Ok(())
        }
    }
}

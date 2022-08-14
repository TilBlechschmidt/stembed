#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

use core::future::Future;
use stabg::{
    desktop::{TypeUsage::*, *},
    embedded::*,
    error::*,
    *,
};

#[derive(Identifiable)]
#[identifiable(name = "test.type", version = "1")]
struct TestType1(u8);

#[derive(Identifiable)]
#[identifiable(name = "test.type", version = "2")]
struct TestType2(u8);

#[derive(Identifiable)]
#[identifiable(name = "test.processor", version = "1")]
struct TestProcessor1;

#[derive(Identifiable)]
#[identifiable(name = "test.processor", version = "2")]
struct TestProcessor2;

#[test]
fn async_full_stack_example() {
    futures::executor::block_on(async move {
        actual_code().await;
    });
}

async fn actual_code() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Build the data storage structures
    let mut stack = FixedSizeStack::<256>::new();
    let mut registry = FixedSizeRegistry::<16>::default();

    // Construct an executor, give it the necessary things, and run it all :)
    let mut executor = Executor::new(&mut stack, &mut registry);

    let mut execution_queue = move |start_id, stack, registry| {
        // Hello!
        async move { Ok(()) }
    };

    loop {
        executor.execute_async(execution_queue).await.unwrap();
        break; // this wouldn't be there in a real scenario
    }
}

// fn execution_queue<'r, 's>(
//     start_id: Option<ShortID>,
//     stack: &'s mut (dyn Stack + 's),
//     registry: &'r (dyn Registry + 'r),
// ) -> impl Future<Output = Result<(), ExecutionError>> + 'r + 's
// where
//     's: 'r,
//     'r: 's,
// {
//     async move {
//         let mut id: ShortID = 0;
//         let mut running = start_id.is_none();

//         // START REPEAT BLOCK
//         if !running && Some(id) == start_id {
//             running = true;
//         }

//         if running {
//             let mut processor = TestProcessor1;
//             let context = ExecutionContext::new(stack, id, registry);
//             processor.process(context).await?;
//         }

//         id += 1;
//         // END REPEAT BLOCK

//         Ok(())
//     }
// }

// ————————————— Implementations of processors below —————————————

impl EmbeddedProcessor for TestProcessor1 {
    const TYPES_INPUT: &'static [Identifier] = &[];
    const TYPES_OUTPUT: &'static [Identifier] = &[TestType1::IDENTIFIER, TestType2::IDENTIFIER];
    const STACK_USAGE: usize = 2 + 12;

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

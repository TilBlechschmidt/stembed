use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use stembed::core::{
    dict::Dictionary,
    engine::{Command, CommandDelta, Engine, EngineCommand},
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct TestStroke(usize);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum TestCommand {
    Indexed(usize),
    Fallback(TestStroke),
}

struct TestDict(HashMap<Vec<TestStroke>, Vec<Command<TestCommand>>>);

impl TestDict {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add(&mut self, outline: Vec<TestStroke>, commands: Vec<Command<TestCommand>>) {
        self.0.insert(outline, commands);
    }
}

impl Dictionary for TestDict {
    type Stroke = TestStroke;
    type OutputCommand = TestCommand;

    fn lookup(
        &self,
        outline: &[Self::Stroke],
    ) -> Option<SmallVec<[Command<Self::OutputCommand>; 2]>> {
        self.0.get(outline).map(|v| {
            let mut output = SmallVec::new();
            output.extend_from_slice(&v);
            output
        })
    }

    fn fallback_commands(
        &self,
        stroke: &Self::Stroke,
    ) -> SmallVec<[Command<Self::OutputCommand>; 2]> {
        smallvec![Command::Output(TestCommand::Fallback(stroke.clone()))]
    }

    fn longest_outline_length(&self) -> usize {
        self.0.keys().map(|v| v.len()).max().unwrap_or_default()
    }
}

const STROKE_A: TestStroke = TestStroke(0);
const STROKE_B: TestStroke = TestStroke(1);
const STROKE_C: TestStroke = TestStroke(2);

const COMMAND_0: TestCommand = TestCommand::Indexed(0);
const COMMAND_1: TestCommand = TestCommand::Indexed(1);
const COMMAND_2: TestCommand = TestCommand::Indexed(2);

#[test]
fn miep() {
    let mut dict = TestDict::new();
    dict.add(vec![STROKE_A], vec![Command::Output(COMMAND_0)]);
    dict.add(vec![STROKE_B], vec![Command::Output(COMMAND_1)]);
    dict.add(vec![STROKE_A, STROKE_B], vec![Command::Output(COMMAND_2)]);
    dict.add(
        vec![STROKE_C],
        vec![Command::Engine(EngineCommand::UndoPrevious)],
    );

    let mut engine = Engine::new(&dict);

    assert_eq!(
        engine.push(STROKE_A),
        CommandDelta {
            to_undo: 0,
            to_push: smallvec![COMMAND_0]
        }
    );

    assert_eq!(
        engine.push(STROKE_B),
        CommandDelta {
            to_undo: 1,
            to_push: smallvec![COMMAND_2]
        }
    );

    assert_eq!(
        engine.push(STROKE_C),
        CommandDelta {
            to_undo: 1,
            to_push: smallvec![COMMAND_0]
        }
    );
}

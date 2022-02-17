use smallvec::smallvec;
use stembed::core::{
    engine::CommandDelta,
    processor::{
        text_formatter::{
            AttachmentMode, CapitalizationMode, TextFormatter, TextOutputCommand,
            TextOutputInstruction,
        },
        CommandProcessor,
    },
};

#[test]
fn something() {
    let mut processor = TextFormatter::new();
    let delta = CommandDelta {
        to_undo: 0,
        to_push: smallvec![
            TextOutputCommand::ChangeCapitalization(CapitalizationMode::CapitalizeNext),
            TextOutputCommand::ChangeAttachment(AttachmentMode::Next),
            TextOutputCommand::Write("hello".into()),
            TextOutputCommand::Write("world".into()),
            TextOutputCommand::ChangeAttachment(AttachmentMode::Next),
            TextOutputCommand::Write("!".into()),
        ],
    };

    assert_eq!(
        processor.consume(delta).as_slice(),
        vec![
            TextOutputInstruction::Write("Hello".into()),
            TextOutputInstruction::Write(" world".into()),
            TextOutputInstruction::Write("!".into())
        ]
    );
}

use core::future::Future;
use embedded_storage_async::nor_flash::AsyncNorFlash;
use engine::{
    input::{GroupingMode, KeyPosition, KeypressGrouper},
    InputState, OutputCommand,
};
use futures::{pin_mut, Sink, SinkExt, Stream, StreamExt};
use repeat::KeypressRepeater;
pub use repeat::{DurationDriver, InstantDriver, TimeDriver};
use shittyengine::{
    dict::{DataSource, RadixTreeDictionary},
    formatter::Formatter,
    matcher::{CommitType, OutlineMatcher},
    Stroke,
};

use super::mutex::Mutex;

mod repeat;

const REPEAT_INTERVAL: u64 = 75;
const REPEAT_TRIGGER_DELAY: u64 = 150;
const REPEAT_MAX_TAP_DIST: u64 = 250;

macro_rules! make_strokemap {
    ( $([ $($position_str:expr),* ]),* ) => {
        &[
            $(
                &[$( KeyPosition::from($position_str), )*],
            )*
        ]
    };
}

const STROKE_MAP: &[&[Option<KeyPosition>]] = make_strokemap![
    ["LM3", "RM3"],                   // #
    ["LP1", "LP2"],                   // S-
    ["LR1"],                          // T-
    ["LR2"],                          // K-
    ["LM1"],                          // P-
    ["LM2"],                          // W-
    ["LI1"],                          // H-
    ["LI2"],                          // R-
    ["LI3"],                          // A
    ["LET3"],                         // O
    ["LET1", "LET2", "REL1", "REL2"], // *
    ["REL3"],                         // E
    ["RI3"],                          // U
    ["RI1"],                          // -F
    ["RI2"],                          // -R
    ["RM1"],                          // -P
    ["RM2"],                          // -B
    ["RR1"],                          // -L
    ["RR2"],                          // -G
    ["RP1"],                          // -T
    ["RP2"],                          // -S
    ["RET1"],                         // -D
    ["RET2"]                          // -Z
];

fn stroke_from_input(input: InputState) -> Stroke {
    let mut state = 0u32;

    for (keys, i) in STROKE_MAP.iter().zip((0..STROKE_MAP.len()).rev()) {
        for key in keys.iter() {
            if Some(true) == key.map(|k| input.is_set(k)) {
                state |= 1 << i;
            }
        }
    }

    Stroke::from_right_aligned(state)
}

pub async fn run<T: TimeDriver>(
    input: impl Stream<Item = InputState>,
    output: impl Sink<OutputCommand>,
    flash: &Mutex<impl AsyncNorFlash>,
    time_driver: T,
) {
    pin_mut!(output);

    let data_source = FlashDataSource(flash);
    let mut output = SinkOutput(&mut output);

    let mut dict = RadixTreeDictionary::new(data_source)
        .await
        .expect("failed to open dictionary");
    let mut matcher = OutlineMatcher::<Stroke, 32>::new(11);
    let mut formatter = Formatter::<32>::new();

    let mut grouper = KeypressGrouper::new(GroupingMode::FirstUp);
    let repeater = KeypressRepeater::new(
        T::Duration::from_millis(REPEAT_INTERVAL),
        T::Duration::from_millis(REPEAT_MAX_TAP_DIST),
        T::Duration::from_millis(REPEAT_TRIGGER_DELAY),
        time_driver,
    );

    pin_mut!(input);

    let grouped_input = repeater
        .apply_grouped_repeat(&mut input, &mut grouper)
        .map(stroke_from_input);

    pin_mut!(grouped_input);

    while let Some(stroke) = grouped_input.next().await {
        // 1. Add the stroke to the matcher
        defmt::info!("Adding stroke");
        matcher.add(stroke);

        while matcher.uncommitted_count() > 0 {
            // 2. Search the dictionary for the uncommitted strokes and commit matching prefixes
            let dict_match = dict
                .match_prefix(matcher.uncommitted_strokes())
                .await
                .unwrap();

            // The following section can be externalised into a crate contained struct really well.
            // Take everything but the dictionary, stuff it into a struct. Add a method to call
            // --------- SECTION START ---------
            if let Some((prefix_length, translation)) = dict_match {
                // Try committing the outline and undo any trailing outlines until the commit succeeds
                loop {
                    let commit_result = matcher.commit(prefix_length, translation.len());

                    match commit_result {
                        Ok(CommitType::FastForward) => break,
                        Ok(CommitType::Regular) => {
                            // Submit the translation to the output
                            for formatter_command in translation.iter() {
                                if let Some(output_command) = formatter.apply(&formatter_command) {
                                    output.apply(output_command).await;
                                }
                            }

                            break;
                        }
                        Err(trailing_outline) => {
                            // Undo the trailing outline
                            for _ in 0..trailing_outline.outline().commands {
                                if let Some(command) = formatter.undo() {
                                    output.apply(command).await;
                                }
                            }

                            trailing_outline.remove();

                            continue;
                        }
                    }
                }
            } else {
                // In a "real" engine implementation you would have a fallback dictionary that outputs the human readable representation
                // TODO Write a default impl for such a fallback dictionary
                defmt::warn!("Omitting trailing strokes for now until more strokes are received");
                break;
            }
            // --------- SECTION END ---------
        }
    }
}

struct SinkOutput<'s, S: Sink<OutputCommand> + Unpin>(&'s mut S);

impl<'s, S: Sink<OutputCommand> + Unpin> SinkOutput<'s, S> {
    async fn apply<I: Iterator<Item = char>>(
        &mut self,
        command: shittyengine::output::OutputCommand<I>,
    ) {
        match command {
            shittyengine::output::OutputCommand::Backspace(count) => {
                self.0.send(OutputCommand::Backspace(count)).await.ok();
            }
            shittyengine::output::OutputCommand::Write(characters) => {
                for c in characters {
                    self.0.send(OutputCommand::Write(c)).await.ok();
                }
            }
        }
    }
}

struct FlashDataSource<'f, F: AsyncNorFlash>(&'f Mutex<F>);

impl<'f, F: AsyncNorFlash + 'f> DataSource for FlashDataSource<'f, F> {
    type Error = F::Error;

    type ReadFut<'s> = impl Future<Output = Result<(), Self::Error>> + 's
    where
        Self: 's;

    fn read_exact<'s>(&'s mut self, location: u32, buffer: &'s mut [u8]) -> Self::ReadFut<'s> {
        // It just reads at offset 0 for now :shrug:
        async move { self.0.lock().await.read(location, buffer).await }
    }
}

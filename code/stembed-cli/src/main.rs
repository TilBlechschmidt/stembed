#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use clap::{Parser, Subcommand};
use std::{
    collections::HashMap,
    fs::File,
    future::Future,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};
use stembed::{
    compile::BinaryDictionaryCompiler,
    core::{
        dict::{BinaryDictionary, Dictionary},
        engine::Engine,
        processor::{text_formatter::TextFormatter, CommandProcessor},
        Stroke, StrokeContext,
    },
    import::plover::parse_dict,
    input::{
        serial::{GeminiPR, SerialPort},
        InputSource,
    },
    io::util::HeapFile,
    output::{OSOutput, OutputSink},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a Plover dictionary into the internal format
    Compile {
        #[clap(short, long = "input")]
        inputs: Vec<PathBuf>,
        #[clap(short, long)]
        output: PathBuf,
    },

    Translate {
        #[clap(short, long = "dictionary")]
        dictionary_path: PathBuf,
    },

    TestLookup {
        #[clap(short, long = "dictionary")]
        dictionary_path: PathBuf,
    },
}

async fn async_main(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::TestLookup { dictionary_path } => {
            let mut dictionary_file = FileReader::open(dictionary_path)?;
            let dictionary = BinaryDictionary::new(&mut dictionary_file).await.unwrap();
            let outline = [Stroke::from_str("H-L", dictionary.stroke_context()).unwrap()];
            let result = dictionary.lookup(&outline).await;
            println!("{:?}", result);
        }
        Commands::Compile { inputs, output } => {
            let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &[])
                .expect("default stroke context");
            let mut compiler = BinaryDictionaryCompiler::new(&context);

            let mut bytes: HashMap<Stroke, usize> = HashMap::new();

            for (tag, input) in inputs.into_iter().enumerate() {
                let content = std::fs::read_to_string(input)?;
                let entries = parse_dict(&content[..], &context).unwrap();
                for entry in entries {
                    let (outline, commands) = entry.unwrap();

                    if let Some(stroke) = outline.first().cloned() {
                        bytes.entry(stroke).and_modify(|x| *x += 1).or_insert(1);
                    }

                    compiler.add(outline, commands, tag as u16).unwrap();
                }
            }

            println!("{}", compiler.stats());
            println!(
                "Average nodes below first node: {}",
                bytes.values().fold(0, |acc, x| acc + x) / bytes.len()
            );
            println!(
                "Maximum nodes below first node: {:?}",
                bytes.values().max().unwrap()
            );

            let mut x: Vec<_> = bytes.into_iter().collect();
            x.sort_by_key(|(_k, v)| *v);
            for (key, value) in x {
                println!("{} => {}", value, key);
            }

            let mut dict_blob = HeapFile::new();
            compiler.serialize(&mut dict_blob).await.unwrap();

            std::fs::write(output, &mut dict_blob.into_inner())?;
        }
        Commands::Translate { dictionary_path } => {
            let mut dictionary_file = FileReader::open(dictionary_path)?;
            let dictionary = BinaryDictionary::new(&mut dictionary_file).await.unwrap();

            let mut engine = Engine::new(&dictionary);
            let mut formatter = TextFormatter::new();

            let mut input_source = GeminiPR::new(SerialPort::new("/dev/tty.usbserial-0001")?);
            let mut output_sink = OSOutput;

            loop {
                let input = input_source.scan()?;
                let stroke = Stroke::from_input(
                    input,
                    &GeminiPR::DEFAULT_KEYMAP,
                    dictionary.stroke_context(),
                );
                let delta = engine.push(stroke).await;
                let output = formatter.consume(delta);
                output_sink.send(output);
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    smol::block_on(async move {
        async_main(cli).await.unwrap();
    });

    Ok(())
}

struct FileReader {
    file: File,
}

impl FileReader {
    fn open(path: PathBuf) -> std::io::Result<Self> {
        // TODO Use actual async file I/O instead of this cheated madness
        Ok(Self {
            file: File::open(path)?,
        })
    }
}

impl stembed::io::Read for FileReader {
    type ReadFuture<'a> = impl Future<Output = Result<u8, stembed::io::Error>> + 'a where Self: 'a;

    fn read(&mut self) -> Self::ReadFuture<'_> {
        async move {
            let mut buf = [0u8; 1];
            self.file
                .read_exact(&mut buf)
                .map_err(|_| stembed::io::Error::EOF)?;
            Ok(buf[0])
        }
    }
}

impl stembed::io::Seek for FileReader {
    type SeekFuture<'a> = impl Future<Output = Result<u64, stembed::io::Error>> + 'a where Self: 'a;
    fn seek(&mut self, pos: stembed::io::SeekFrom) -> Self::SeekFuture<'_> {
        async move {
            let pos = match pos {
                stembed::io::SeekFrom::Start(offset) => SeekFrom::Start(offset),
                stembed::io::SeekFrom::End(offset) => SeekFrom::End(offset),
                stembed::io::SeekFrom::Current(offset) => SeekFrom::Current(offset),
            };

            self.file.seek(pos).map_err(|_| stembed::io::Error::Unknown)
        }
    }
}

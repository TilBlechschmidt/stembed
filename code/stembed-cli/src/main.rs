use clap::{Parser, Subcommand};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};
use stembed::{
    compile::BinaryDictionaryCompiler,
    core::{
        dict::BinaryDictionary,
        engine::Engine,
        processor::{text_formatter::TextFormatter, CommandProcessor},
        Stroke, StrokeContext,
    },
    import::plover::parse_dict,
    input::{
        serial::{GeminiPR, SerialPort},
        InputSource,
    },
    io::HeapFile,
    output::{OSOutput, OutputSink},
    serialize::Serialize,
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { inputs, output } => {
            let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &[])
                .expect("default stroke context");
            let mut compiler = BinaryDictionaryCompiler::new(&context);

            for (tag, input) in inputs.into_iter().enumerate() {
                let content = std::fs::read_to_string(input)?;
                let entries = parse_dict(&content[..], &context).unwrap();
                for entry in entries {
                    let (outline, commands) = entry.unwrap();
                    compiler.add(outline, commands, tag as u16).unwrap();
                }
            }

            println!("{}", compiler.stats());

            let mut dict_blob = HeapFile::new();
            compiler.serialize(&mut dict_blob).unwrap();

            std::fs::write(output, &mut dict_blob.into_inner())?;
        }
        Commands::Translate { dictionary_path } => {
            let mut dictionary_file = FileReader::open(dictionary_path)?;
            let dictionary = BinaryDictionary::new(&mut dictionary_file).unwrap();

            let mut engine = Engine::new(dictionary);
            let mut formatter = TextFormatter::new();

            let mut input_source = GeminiPR::new(SerialPort::new("/dev/tty.usbserial-0001")?);
            let mut output_sink = OSOutput;

            loop {
                let context = engine.dictionary().stroke_context().clone();
                let input = input_source.scan()?;
                let stroke = Stroke::from_input(input, &GeminiPR::DEFAULT_KEYMAP, context);
                let delta = engine.push(stroke);
                let output = formatter.consume(delta);
                output_sink.send(output);
            }
        }
    }

    Ok(())
}

struct FileReader {
    file: File,
}

impl FileReader {
    fn open(path: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
        })
    }
}

impl stembed::io::Read for FileReader {
    fn read_u8(&mut self) -> Result<u8, stembed::io::Error> {
        let mut buf = [0u8; 1];
        self.file
            .read_exact(&mut buf)
            .map_err(|_| stembed::io::Error::EOF)?;
        Ok(buf[0])
    }
}

impl stembed::io::Seek for FileReader {
    fn seek(&mut self, pos: stembed::io::SeekFrom) -> Result<u64, stembed::io::Error> {
        let pos = match pos {
            stembed::io::SeekFrom::Start(offset) => SeekFrom::Start(offset),
            stembed::io::SeekFrom::End(offset) => SeekFrom::End(offset),
            stembed::io::SeekFrom::Current(offset) => SeekFrom::Current(offset),
        };

        self.file.seek(pos).map_err(|_| stembed::io::Error::Unknown)
    }
}

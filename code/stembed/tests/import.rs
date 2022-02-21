use stembed::{
    compile::BinaryDictionaryCompiler,
    core::{
        dict::{BinaryDictionary, Dictionary},
        engine::Engine,
        processor::{text_formatter::TextFormatter, CommandProcessor},
        Stroke, StrokeContext,
    },
    import::plover::parse_dict,
    io::{HeapFile, Seek},
    serialize::Serialize,
};

#[test]
fn plover() {
    let raw_json =
        std::fs::read_to_string("/Users/tibl/Library/Application Support/plover/main.json")
            .unwrap();

    let context = StrokeContext::new("#STKPWHR", "AO*EU", "FRPBLGTSDZ", &["FN1", "FN2"]).unwrap();
    let mut compiler = BinaryDictionaryCompiler::new(&context);

    let entries = parse_dict(&raw_json[..], &context).unwrap();
    for entry in entries {
        let (outline, commands) = entry.unwrap();
        compiler.add(outline, commands, 0).unwrap();
    }

    let mut dict_blob = HeapFile::new();
    compiler.serialize(&mut dict_blob).unwrap();
    println!("Dict size: {} bytes", dict_blob.stream_len().unwrap());

    let dictionary = BinaryDictionary::new(&mut dict_blob).unwrap();
    println!(
        "{} vs. {}",
        dictionary.stroke_context().byte_count(),
        context.byte_count()
    );

    println!("{:?}", dictionary.stroke_context());
    println!("{:?}", dictionary.longest_outline_length());
    println!(
        "{:?}",
        dictionary.lookup(&[Stroke::from_str("KPA*", &dictionary.stroke_context()).unwrap()])
    );
    let mut engine = Engine::new(&dictionary);
    let mut processor = TextFormatter::new();

    let strokes = "KPA*/H-L/WORLD/TP-BG/PO/TAEU/TOE/SADZ"
        .split('/')
        .map(|stroke| Stroke::from_str(stroke, &dictionary.stroke_context()).unwrap());

    for stroke in strokes {
        println!("Stroke: {}", stroke);
        let delta = engine.push(stroke);
        println!("\t{:?}", delta);
        let output = processor.consume(delta);
        for instruction in output {
            println!("\t => {:?}", instruction);
        }
    }

    panic!();
}

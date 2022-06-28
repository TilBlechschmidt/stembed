use shittyengine::{
    compile::{BufferedSource, Compiler},
    dict::RadixTreeDictionary,
    formatter::Formatter,
    matcher::{CommitType, OutlineMatcher},
    output::{OSOutput, OutputProcessor},
    Stroke,
};

// TODO List
// - Store longest outline length in dict

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load and compile the JSON dictionary
    let path = "/Users/tibl/Library/Application Support/plover/main.json";
    let json_dict_content = std::fs::read_to_string(path)?;
    let (_, dict_buffer) = Compiler::compile_from_json(json_dict_content.as_str());
    let mut dict_source = BufferedSource::new(&dict_buffer);

    // Build all the needed structures
    let input = geminipr_stroke_iterator();
    let mut dict = RadixTreeDictionary::new(&mut dict_source).unwrap();
    let mut matcher = OutlineMatcher::<Stroke, 32>::new(11);
    let mut formatter = Formatter::<32>::new();
    let mut output = OSOutput::new();

    // Run the loop
    for stroke in input {
        // 1. Add the stroke to the matcher
        println!("Adding stroke {stroke:?}");
        matcher.add(stroke);

        while matcher.uncommitted_count() > 0 {
            // 2. Search the dictionary for the uncommitted strokes and commit matching prefixes
            let dict_match = dict.match_prefix(matcher.uncommitted_strokes()).unwrap();

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
                                    output.apply(output_command);
                                }
                            }

                            break;
                        }
                        Err(trailing_outline) => {
                            // Undo the trailing outline
                            for _ in 0..trailing_outline.outline().commands {
                                if let Some(command) = formatter.undo() {
                                    output.apply(command);
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
                println!("Omitting trailing strokes for now until more strokes are received");
                continue;
            }
            // --------- SECTION END ---------
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

fn geminipr_stroke_iterator() -> impl Iterator<Item = Stroke> {
    let strokes = vec![
        //             #STKPWHR AO*EU FRPBLGTSDZ
        Stroke::from(0b00000010_00000_0000100000_0), // Hello
        Stroke::from(0b00000100_01000_0100100010_0), // world
        Stroke::from(0b00101000_00000_0001010000_0), // !
        // Part of the commands.json dict and using the currently unsupported ~| operator 🤷‍♂️
        // Stroke::from(0b00000001_00000_0100000000_0), // \n
        Stroke::from(0b00000010_01001_0000000000_0), // How
        Stroke::from(0b00000001_00000_0000000000_0), // are
        Stroke::from(0b00000000_00001_0000000000_0), // you
        Stroke::from(0b00100000_01010_0000000010_0), // today
        Stroke::from(0b00010100_00000_0010100000_0), // ?
        Stroke::from(0b00001000_01000_0000000000_0), // po
        Stroke::from(0b00100000_10011_0000000000_0), // {^ta}
        Stroke::from(0b00100000_01010_0000000000_0), // {^to}
                                                     // Stroke::from(0b00000000_00000_0000000000_0),
    ];
    strokes.into_iter()
}

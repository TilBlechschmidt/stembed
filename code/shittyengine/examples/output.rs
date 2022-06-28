use shittyengine::output::{OSOutput, OutputCommand, OutputProcessor};

fn main() {
    let mut output = OSOutput::new();
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("Writing!");
    output.apply(OutputCommand::Write("Hello world!".chars()));
    output.apply(OutputCommand::Write("Hope this works ...".chars()));
    println!("Done!");
    std::thread::sleep(std::time::Duration::from_secs(1));
}

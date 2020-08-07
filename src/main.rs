use jsonatr::helpers::*;
use jsonatr::transformer::*;

use gumdrop::Options;
use serde_json::Value;
use simple_error::*;

#[derive(Debug, Options)]
struct CliOptions {
    #[options(no_short, help = "print this help and exit")]
    help: bool,
    #[options(no_short, help = "provide detailed usage instructions")]
    usage: bool,
    #[options(long="use", no_short, help = "include input-output spec from FILE", meta="FILE")]
    include: Vec<String>,
    #[options(no_short, help = "read main input from STDIN")]
    stdin: bool,
    #[options(no_short, long="in", help = "read main input from FILE", meta="FILE")]
    input: Option<String>,
    #[options(no_short, long = "out", help = "write generated output into FILE instead of STDOUT", meta="FILE")]
    output: Option<String>,
    #[options(free, help = "provide output spec inline")]
    output_spec: Option<String>
}


fn run() -> Result<(), SimpleError> {
    let opts = CliOptions::parse_args_default_or_exit();
    if opts.stdin  && opts.input.is_some() {
        bail!("both --stdin and --input are given, but only one main input can be accepted")
    }

    let current_dir = std::env::current_dir().unwrap().to_str().unwrap().to_owned();
    let mut spec = Transformer::empty(&current_dir);
    for path in &opts.include {
        spec.add_use(path.to_string())?;
    }

    if let Some(output_spec) = opts.output_spec {
        let output = parse_string(&output_spec)?;
        spec.add_output(output)?
    }

    // The 'main' input, i.e. the one that can be addressed in the output spec with unnamed $
    let main: Value;
    if opts.stdin {
        main = parse_stdin()?
    }
    else if let Some(input) = opts.input {
        main = parse_file(&input)?
    }
    else {
        main = Value::Null;
    }

    let res = spec.transform(&main)?;
    if let Some(path) = opts.output {
        try_with!(std::fs::write(path, res), "failed to write output")
    }
    else {
        println!("{}", res);
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => println!("Error: {}", e)
    }
}

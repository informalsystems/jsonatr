mod jsonatr;
mod helpers;

extern crate jsonpath_lib as jsonpath;
extern crate shell_words;

extern crate simple_error;
#[macro_use]
extern crate lazy_static;

use std::process::{Command};
use gumdrop::Options;
use jsonatr::Jsonatr;
use serde_json::Value;
use simple_error::*;
use crate::helpers::*;

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

    let mut spec = Jsonatr::empty();
    for include in opts.include {
        let file = read_file(&include)?;
        let other = Jsonatr::new(&file)?;
        spec.merge(&other)?;
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

mod tests {
    use crate::*;
    fn test_expect(file: &str, expect: &str) {
        let input = std::fs::read_to_string(file).unwrap();
        let mut spec = Jsonatr::new(&input).unwrap();
        let res = spec.transform(&Value::Null).unwrap();
        assert_eq!(res, expect)
    }

    #[test]
    fn test_simple()  {
        test_expect("tests/support/simple.json",r#"{
  "tool": "jsonatr",
  "version": 0.1,
  "stable": false,
  "features": [
    "read",
    "write"
  ]
}"#);
    }

    #[test]
    fn test_simple_with_version()  {
        test_expect("tests/support/simple_with_version.json",r#"{
  "tool": "jsonatr",
  "version": "0.1",
  "stable": false,
  "features": [
    "read",
    "write"
  ]
}"#);
    }

    #[test]
    fn test_simple_with_command()  {
        let output = Command::new("date").args(&["-I"]).output().unwrap();
        let date = serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim_end().to_string());
        test_expect("tests/support/simple_with_command.json",&format!(r#"{{
  "tool": "jsonatr",
  "version": 0.1,
  "date": {},
  "stable": false,
  "features": [
    "read",
    "write"
  ]
}}"#, date));
    }
}
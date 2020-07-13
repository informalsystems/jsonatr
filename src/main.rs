mod jsonatr;

extern crate jsonpath_lib as jsonpath;
extern crate shell_words;

#[macro_use]
extern crate simple_error;
#[macro_use]
extern crate lazy_static;

use std::env;
use std::process::{Command, Stdio};
use gumdrop::Options;
use jsonatr::Jsonatr;

#[derive(Debug, Options)]
struct CliOptions {
    #[options(no_short, help = "print this help and exit")]
    help: bool,
    #[options(no_short, help = "provide detailed usage instructions")]
    usage: bool,
    #[options(no_short, help = "include input-output spec from FILE", meta="FILE")]
    include: Vec<String>,
    #[options(no_short, help = "read 'main' input from STDIN")]
    stdin: bool,
    #[options(no_short, long="in", help = "read 'main' input from FILE", meta="FILE")]
    input: Option<String>,
    #[options(no_short, long = "out", help = "write generated output into FILE instead of STDOUT", meta="FILE")]
    output: Option<String>,
    #[options(free, help = "provide output spec inline")]
    output_spec: Option<String>
}



fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Error: expecting JSON transformation spec");
        std::process::exit(1);
    }
    let input = std::fs::read_to_string(&args[1])?;
    let mut spec = Jsonatr::new(&input)?;
    let res = spec.transform()?;
    println!("{}", res);
    Ok(())
}

fn main() {
    // let opts = CliOptions::parse_args_default_or_exit();
    // println!("{:?}", opts);
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
        let res = spec.transform().unwrap();
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
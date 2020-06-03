use serde::Deserialize;
use serde_json::Error;
use std::env;

#[derive(Debug, Deserialize)]
struct Spec {
    description: String,
    inputs: Vec<Input>,
    output: serde_json::Value
}

#[derive(Debug, Deserialize)]
struct Input {
    name: String,
    source: String
}

fn transform(input: &str) -> Result<String, Error> {
    let spec: Result<Spec, Error> = serde_json::from_str(input);
    match spec {
        Ok(spec) => serde_json::to_string_pretty(&spec.output),
        Err(e) => Err(e)
    }
}

#[test]
fn test_simple_output()  {
    let input =
r#"{
  "description": "Test simple output",
  "inputs": [],
   "output": {
     "tool": "jonatr",
     "version": 0.1,
     "stable": false,
     "features": ["read", "write"]
  }
}"#;
    let expected =
r#"{
  "features": [
    "read",
    "write"
  ],
  "stable": false,
  "tool": "jonatr",
  "version": 0.1
}"#;
    assert_eq!(transform(input).unwrap(), expected.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Error: expecting JSON transformation spec");
        std::process::exit(1);
    }
    let input = std::fs::read_to_string(&args[1])?;
    let res = transform(&input)?;
    println!("{}", res);
    Ok(())
}
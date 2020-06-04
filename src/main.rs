use serde::Deserialize;
use serde_json::Error;
use serde_json::Value;
use std::env;
use crate::InputKind::FILE;

#[derive(Debug, Deserialize)]
struct Jsonatr {
    description: String,
    inputs: Vec<Input>,
    output: Value,

    #[serde(skip)]
    files: std::collections::HashMap<String, Value>
}

#[derive(Debug, Deserialize, PartialEq)]
enum InputKind {
    FILE,
    COMMAND
}

#[derive(Debug, Deserialize)]
struct Input {
    name: String,
    kind: InputKind,
    source: String
}

impl Jsonatr {
    fn new(spec: &str) -> Result<Jsonatr, Box<dyn std::error::Error>> {
        let mut spec: Jsonatr = serde_json::from_str(spec)?;
        for input in &spec.inputs {
            if input.kind == FILE {
                let file = std::fs::read_to_string(&input.source)?;
                let value: Value = serde_json::from_str(&file)?;
                spec.files.insert(input.name.clone(), value);
            }
        }
        Ok(spec)
    }

    fn transform(&self) -> Result<String, Error> {
        let transformed_output = self.transform_value(&self.output);
        serde_json::to_string_pretty(&transformed_output)
    }

    fn transform_value(&self, v: &Value) -> Value {
        match v {
            Value::String(string) => {
                if string.starts_with("$") {
                    let key = &string[1..];
                    if self.files.contains_key(key) {
                        return self.files.get(key).unwrap().clone()
                    }
                }
                Value::String(string.to_string())
            }
            Value::Array(values) => {
                let new_values = values.iter().map(|x| self.transform_value(x)).collect();
                Value::Array(new_values)
            },
            Value::Object(values) => {
                let mut new_values: serde_json::map::Map<String, Value> = serde_json::map::Map::new();
                for (k,v) in values.iter() {
                    new_values.insert(k.to_string(),self.transform_value(v));
                }
                Value::Object(new_values)
            },
            _ => v.clone()
        }
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
  "tool": "jonatr",
  "version": 0.1,
  "stable": false,
  "features": [
    "read",
    "write"
  ]
}"#;
    assert_eq!(Jsonatr::new(input).unwrap().transform().unwrap(), expected.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Error: expecting JSON transformation spec");
        std::process::exit(1);
    }
    let input = std::fs::read_to_string(&args[1])?;
    let spec = Jsonatr::new(&input)?;
    let res = spec.transform()?;
    println!("{}", res);
    Ok(())
}
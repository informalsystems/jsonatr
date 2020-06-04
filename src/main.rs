
use serde::Deserialize;
use serde_json::Error;
use serde_json::Value;
use std::env;
use std::process::Command;

use crate::InputKind::*;

#[derive(Debug, Deserialize)]
struct Jsonatr {
    description: String,
    inputs: Vec<Input>,
    output: Value,

    #[serde(skip)]
    files: std::collections::HashMap<String, Value>,

    #[serde(skip)]
    commands: std::collections::HashMap<String, Value>

}

#[derive(Debug, Deserialize, PartialEq)]
enum InputKind {
    FILE,
    COMMAND
}

#[derive(Debug, Deserialize)]
pub struct Input {
    name: String,
    kind: InputKind,
    source: String,

    #[serde(default)]
    args: Vec<String>
}

impl Jsonatr {
    fn new(spec: &str) -> Result<Jsonatr, Box<dyn std::error::Error>> {
        let mut spec: Jsonatr = serde_json::from_str(spec)?;
        for input in &spec.inputs {
            match &input.kind {
                FILE => {
                    let file = std::fs::read_to_string(&input.source)?;
                    let value: Value = serde_json::from_str(&file)?;
                    spec.files.insert(input.name.clone(), value);
                }
                COMMAND => {
                    let output = Command::new(&input.source).output()?;
                    spec.commands.insert(input.name.clone(), Value::String(String::from_utf8_lossy(&output.stdout).trim_end().to_string()));
                }
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
                    else if self.commands.contains_key(key) {
                        return self.commands.get(key).unwrap().clone()
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

mod tests {
    use crate::*;
    fn test_expect(file: &str, expect: &str) {
        let input = std::fs::read_to_string(file).unwrap();
        let spec = Jsonatr::new(&input).unwrap();
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
        let output = Command::new("date").output().unwrap();
        let date = Value::String(String::from_utf8_lossy(&output.stdout).trim_end().to_string());
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

use serde::Deserialize;
use serde_json::Error;
use serde_json::Value;
use std::env;
use std::process::Command;
use regex::Regex;

extern crate jsonpath_lib as jsonpath;
use crate::InputKind::*;


struct Expr {
    input: String,
    jpath: String,
    transforms: Vec<String>
}

#[derive(Debug, Deserialize)]
struct Jsonatr {
    input: Vec<Input>,
    output: Value,

    #[serde(default)]
    description: String,

    #[serde(skip)]
    inputs: std::collections::HashMap<String, Value>
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
        for input in &spec.input {
            match &input.kind {
                FILE => {
                    let file = std::fs::read_to_string(&input.source)?;
                    let value: Value = serde_json::from_str(&file)?;
                    spec.inputs.insert(input.name.clone(), value);
                }
                COMMAND => {
                    let output = Command::new(&input.source).output()?;
                    spec.inputs.insert(input.name.clone(), Value::String(String::from_utf8_lossy(&output.stdout).trim_end().to_string()));
                }
            }
        }
        Ok(spec)
    }

    // parses a Jsonatr expression, which is of the form
    // $<input><jsonpath> (| <transform>)*
    //   <input> is an identifier, referring to an some of the inputs
    //   <jsonpath> is a JsonPath expression, interpreted by the jsonpath_lib
    //   (| <transform>)* is a pipe-separated sequence of transforms, each being an identifier
    fn parse_expr(&self, text: &str) -> Option<Expr> {
        let input_re = Regex::new(r"^\$([[:alpha:]_][[:word:]_]*)").unwrap();
        let input_cap = input_re.captures(text)?;
        let transform_re = Regex::new(r"[ \t]*\|[ \t]*([[:alpha:]_][[:word:]_]*)[ \t]*$").unwrap();
        let start = input_cap[0].len();
        let mut end = text.len();
        let mut transforms: Vec<String> = Vec::new();
        while let Some(transform_cap) = transform_re.captures(&text[start..end]) {
            transforms.insert(0, transform_cap[1].to_string());
            end -= transform_cap[0].len();
        }
        Some(Expr {
            input: input_cap[1].to_string(),
            jpath: "$".to_string() + &text[start..end],
            transforms
        })
    }

    fn transform(&self) -> Result<String, Error> {
        let transformed_output = self.transform_value(&self.output);
        serde_json::to_string_pretty(&transformed_output)
    }

    fn transform_string(&self, text: &String) -> Option<Value> {
        let expr = self.parse_expr(text)?;
        let json = self.inputs.get(&expr.input)?;
        let mut selector = jsonpath::selector(&json);
        match selector(&expr.jpath) {
            Ok(values) => {
                Some(Value::Array(values.into_iter().cloned().collect()))
            }
            Err(_) => {
                eprintln!("Error applying JsonPath expression {}", expr.jpath);
                None
            }
        }
    }

    fn transform_value(&self, v: &Value) -> Value {
        match v {
            Value::String(string) => {
                if let Some(value) = self.transform_string(string) {
                    value
                }
                else {
                    v.clone()
                }
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
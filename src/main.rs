
use serde::Deserialize;
use serde_json::Error;
use serde_json::Value;
use std::env;
use std::process::Command;
use regex::Regex;

extern crate jsonpath_lib as jsonpath;
use crate::InputKind::*;

#[macro_use]
extern crate simple_error;

struct Expr {
    input: String,
    jpath: String,
    transforms: Vec<String>
}

type Transforms = std::collections::HashMap<String, fn(Value) -> Option<Value>>;

#[derive(Debug, Deserialize)]
struct Jsonatr {
    input: Vec<Input>,
    output: Value,

    #[serde(default)]
    description: String,

    #[serde(skip)]
    inputs: std::collections::HashMap<String, Value>,

    #[serde(skip)]
    builtins: Transforms
}

#[derive(Debug, Deserialize, PartialEq)]
enum InputKind {
    INLINE, // inline JSON
    FILE, // external JSON file
    COMMAND // external command; its output should either be a valid JSON, or otherwise is converted to a JSON string
  //  INLINE_TRANSFORM // inline JSON transformation, that can reference the input via $.
}

#[derive(Debug, Deserialize)]
pub struct Input {
    name: String,
    kind: InputKind,
    source: Value,

    #[serde(default)]
    args: Vec<String>
}

impl Jsonatr {
    fn new(spec: &str) -> Result<Jsonatr, Box<dyn std::error::Error>> {
        let mut spec: Jsonatr = serde_json::from_str(spec)?;
        spec.builtins.insert("unwrap".to_string(),Jsonatr::builtin_unwrap);
        for input in &spec.input {
            if spec.builtins.contains_key(&input.name) {
                bail!("can't define input '{}' because of the builtin function with the same name", input.name)
            }
            if spec.inputs.contains_key(&input.name) {
                bail!("double definition of input '{}'", input.name)
            }
            match &input.kind {
                FILE => {
                    let file = std::fs::read_to_string(&input.source.as_str().unwrap())?; // TODO
                    let value: Value = serde_json::from_str(&file)?;
                    let transformed = spec.transform_value(&value, &Value::Null);
                    spec.inputs.insert(input.name.clone(), transformed);
                }
                COMMAND => {
                    let output = Command::new(&input.source.as_str().unwrap()).output()?; // TODO
                    let value = Value::String(String::from_utf8_lossy(&output.stdout).trim_end().to_string());
                    let transformed = spec.transform_value(&value, &Value::Null);
                    spec.inputs.insert(input.name.clone(), transformed);
                },
                INLINE => {
                    let transformed = spec.transform_value(&input.source, &Value::Null);
                    spec.inputs.insert(input.name.clone(), transformed);
                }
            }
        }
        Ok(spec)
    }

    // assumes that the value is a singleton array; transforms array into its single element
    fn builtin_unwrap(v: Value) -> Option<Value> {
        let arr = v.as_array()?;
        match arr.len() {
            1 => Some(arr[0].clone()),
            _ => None
        }
    }

    // parses a Jsonatr expression, which is of the form
    // $<input>.<jsonpath> (| <transform>)*
    //   <input> is an identifier, referring to an some of the inputs
    //   $.<jsonpath> is a JsonPath expression, interpreted by the jsonpath_lib
    //   (| <transform>)* is a pipe-separated sequence of transforms, each being an identifier
    fn parse_expr(&self, text: &str) -> Option<Expr> {
        let input_re = Regex::new(r"^\$([[:word:]]*)").unwrap();
        let input_cap = input_re.captures(text)?; // parsing fails if text doesn't contain input
        let transform_re = Regex::new(r"[ \t]*\|[ \t]*([[:word:]]+)[ \t]*$").unwrap();
        let start = input_cap[0].len();
        let mut end = text.len();
        let mut transforms: Vec<String> = Vec::new();
        while let Some(transform_cap) = transform_re.captures(&text[start..end]) {
            transforms.insert(0, transform_cap[1].to_string());
            end -= transform_cap[0].len();
        }
        Some(Expr {
            input: input_cap[1].to_string(),
            jpath: text[start..end].to_string(),
            transforms
        })
    }

    fn transform(&self) -> Result<String, Error> {
        let transformed_output = self.transform_value(&self.output, &Value::Null);
        serde_json::to_string_pretty(&transformed_output)
    }

    fn transform_string(&self, text: &String, root: &Value) -> Option<Value> {
        println!("transform text: {}", text);
        println!("transform root: {}", root);

        let expr = self.parse_expr(text)?;
        let json = match expr.input.as_str() {
            "" => match root {
                Value::Null => None,
                x => Some(x)
            }
            _ => self.inputs.get(&expr.input)
        }?;
        let mut value: Value;
        if expr.jpath.is_empty() {
            value = json.clone()
        }
        else {
            value = match jsonpath::select(&json, ("$".to_string() + &expr.jpath).as_str()) {
                Ok(values) => {
                    Some(Value::Array(values.into_iter().cloned().collect()))
                }
                Err(_) => {
                    eprintln!("Error applying JsonPath expression {}", expr.jpath);
                    None
                }
            }?;
        }
        for transform_name in expr.transforms {
            if let Some(builtin) = self.builtins.get(&transform_name) {
                value = builtin(value)?
            }
            else if let Some(transform) = self.inputs.get(&transform_name) {
                value = self.transform_value(transform, &value);
            }
        }
        Some(value)
    }

    fn transform_value(&self, v: &Value, input: &Value) -> Value {
        match v {
            Value::String(string) => {
                if let Some(value) = self.transform_string(string, input) {
                    println!("transform result: {}", value);
                    value
                }
                else {
                    println!("transform result: {}", v);
                    v.clone()
                }
            }
            Value::Array(values) => {
                let new_values = values.iter().map(|x| self.transform_value(x, input)).collect();
                Value::Array(new_values)
            },
            Value::Object(values) => {
                let mut new_values: serde_json::map::Map<String, Value> = serde_json::map::Map::new();
                for (k,v) in values.iter() {
                    new_values.insert(k.to_string(),self.transform_value(v, input));
                }
                Value::Object(new_values)
            },
            _ => v.clone()
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
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

use serde::Deserialize;
use serde_json::Error;
use serde_json::Value;
use std::env;
use std::process::{Command, Stdio};
use regex::Regex;

extern crate jsonpath_lib as jsonpath;
extern crate shell_words;
use crate::InputKind::*;
use std::io::{Write, Read};

#[macro_use]
extern crate simple_error;
#[macro_use]
extern crate lazy_static;

struct Expr {
    input: String,
    jpath: String,
    transforms: Vec<(String,Vec<String>)>
}

type Locals = Vec<std::collections::HashMap<String, Value>>;
type Builtin = fn(&mut Jsonatr, Value, &Vec<String>) -> Option<Value>;
type Builtins = std::collections::HashMap<String, Builtin>;

#[derive(Deserialize)]
struct Jsonatr {
    input: Vec<Input>,
    output: Value,

    #[serde(skip)]
    inputs: std::collections::HashMap<String, Input>,

    #[serde(skip)]
    locals: Locals,

    #[serde(skip)]
    builtins: Builtins
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
enum InputKind {
    INLINE, // inline JSON
    FILE, // external JSON file
    COMMAND // external command; its output should either be a valid JSON, or otherwise is converted to a JSON string
}

#[derive(Debug, Deserialize, Clone)]
pub struct Input {
    name: String,
    kind: InputKind,
    #[serde(rename = "let")]
    lets: Option<Value>,
    source: Value,

    #[serde(default)]
    args: Vec<String>
}

lazy_static! {
    static ref INPUT_RE: Regex = Regex::new(r"^\$([[:word:]]*)").unwrap();
    static ref TRANSFORM_RE: Regex = Regex::new(r"[ \t]*\|[ \t]*([[:word:]]+)[ \t]*(?:\([ \t]*([^)]*?)[ \t]*\))?[ \t]*$").unwrap();
    static ref SEP_RE: Regex = Regex::new(r"[ \t]*,[ \t]*").unwrap();
}

impl Jsonatr {
    fn new(spec: &str) -> Result<Jsonatr, Box<dyn std::error::Error>> {
        let mut spec: Jsonatr = serde_json::from_str(spec)?;
        spec.builtins.insert("unwrap".to_string(),Jsonatr::builtin_unwrap);
        spec.builtins.insert("map".to_string(),Jsonatr::builtin_map);
        for input in &spec.input {
            if spec.builtins.contains_key(&input.name) {
                bail!("can't define input '{}' because of the builtin function with the same name", input.name)
            }
            if spec.inputs.contains_key(&input.name) {
                bail!("double definition of input '{}'", input.name)
            }
            if let Some(l) = &input.lets {
                if l.as_object().is_none() {
                    bail!("wrong 'let' clause of input '{}': should be an object", input.name)
                }
            }
            spec.inputs.insert(input.name.clone(), input.clone());
        }
        Ok(spec)
    }

    // assumes that the value is a singleton array; transforms array into its single element
    fn builtin_unwrap(&mut self, v: Value, _args: &Vec<String>) -> Option<Value> {
        let arr = v.as_array()?;
        match arr.len() {
            1 => Some(arr[0].clone()),
            _ => None
        }
    }

    // assumes that the value is an array, and there is a single argument, which is an input name
    fn builtin_map(&mut self, v: Value, args: &Vec<String>) -> Option<Value> {
        let arr = v.as_array()?;
        match args.len() {
            1 => {
                let new_arr: Vec<Value> = arr.iter().map(
                    |x|
                        match self.apply_input(&args[0], &x) {
                            Ok(res) => res,
                            Err(e) => {
                                eprintln!("Error: failed to apply input transform '{}'; reason: {}", args[0], e.to_string());
                                x.clone()
                            }
                        }
                ).collect();
                Some(Value::Array(new_arr))
            },
            _ => None
        }
    }

    // parses a Jsonatr expression, which is of the form
    // $<input>.<jsonpath>  [| <transform> [(arg,...)]]*
    //   <input> is an identifier, referring to an some of the inputs
    //   $.<jsonpath> is a JsonPath expression, interpreted by the jsonpath_lib
    //   [| <transform> [(arg,...)]]* is a pipe-separated sequence of transforms,
    // each transform being an identifier with optional arguments
    fn parse_expr(&self, text: &str) -> Option<Expr> {
        let input_cap = INPUT_RE.captures(text)?; // parsing fails if text doesn't contain input
        let start = input_cap[0].len();
        let mut end = text.len();
        let mut transforms: Vec<(String,Vec<String>)> = Vec::new();
        while let Some(transform_cap) = TRANSFORM_RE.captures(&text[start..end]) {
            let name = transform_cap[1].to_string();
            end -= transform_cap[0].len();
            let mut args: Vec<String> = Vec::new();
            if let Some(args_match) = transform_cap.get(2) {
                args = SEP_RE.split(args_match.as_str()).into_iter().map(|s| s.to_string()).collect();
            }
            transforms.insert(0, (name, args));
        }
        Some(Expr {
            input: input_cap[1].to_string(),
            jpath: text[start..end].to_string(),
            transforms
        })
    }

    fn apply_input(&mut self, name: &String, root: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        // first try to find the reference in some local scope
        for scope in self.locals.iter().rev() {
            if scope.contains_key(name) {
                return Ok(scope.get(name).unwrap().clone());
            }
        }
        // if none is found, it should be present in the inputs
        let input = require_with!(self.inputs.get(name), "found reference to unknown input '{}'", name).clone();
        let lets: serde_json::Map<String, Value> =
            match input.lets {
                None => serde_json::Map::new(),
                Some(lets) => require_with!(lets.as_object(),"let clause of input '{}' is not an object", name).clone()
            };
        let mut locals = std::collections::HashMap::new();
        for (k, v) in lets {
            locals.insert(k.clone(), self.transform_value(&v, root));
        }
        self.locals.push(locals);
        let result: Value;
        match input.kind {
            INLINE => {
                result = self.transform_value(&input.source, root);
            },
            FILE => {
                if let Some(path) = input.source.as_str() {
                    let file = std::fs::read_to_string(path)?;
                    let value = serde_json::from_str(&file)?;
                    result = self.transform_value(&value, &root);
                }
                else {
                    bail!("non-string provided as source for input '{}'", input.name)
                }
            }
            COMMAND => {
                if let Some(command) = input.source.as_str() {
                    match shell_words::split(command) {
                        Ok(args) => {
                            if args.len() < 1 {
                                bail!("failed to parse command for input '{}'", input.name);
                            }

                            let mut process = match Command::new(&args[0])
                                .args(&args[1..])
                                .stdin(Stdio::piped())
                                .stdout(Stdio::piped())
                                .spawn() {
                                Err(e) => bail!("failed to run command for input '{}'; reason: {}", input.name, e.to_string()),
                                Ok(process) => process,
                            };

                            match &process.stdin.as_mut().unwrap().write(serde_json::to_string(root).unwrap().as_bytes()) {
                                Err(_) => bail!("couldn't write to command stdin for input '{}'", input.name),
                                Ok(_) => (),
                            }
                            let status = process.wait()?;
                            if !status.success() {
                                bail!("failed to execute command for input '{}': {}", input.name, status.to_string())
                            }
                            let mut output = String::new();
                            match process.stdout.unwrap().read_to_string(&mut output) {
                                Err(_) => bail!("couldn't read from command stdout for input '{}", input.name),
                                Ok(_) => (),
                            }
                            match serde_json::from_str(&output) {
                                Err(_) => result = Value::String(output.trim_end().to_string()),
                                Ok(value) => result = value
                            }
                        }
                        Err(_) => bail!("failed to parse command for input '{}'", input.name)
                    }

                }
                else {
                    bail!("non-string provided as source for input '{}'", input.name)
                }
            }
        };
        self.locals.pop(); // TODO: this should be done on all exit branches
        Ok(result)
    }

    fn transform(&mut self) -> Result<String, Error> {
        let transformed_output = self.transform_value(&self.output.clone(), &Value::Null);
        serde_json::to_string_pretty(&transformed_output)
    }

    fn transform_string(&mut self, text: &String, root: &Value) -> Option<Value> {
        //println!("transform text: {}", text);
        //println!("transform root: {}", root);

        let expr = self.parse_expr(text)?;
        let json = match expr.input.as_str() {
            "" => match root {
                Value::Null => None,
                x => Some(x.clone())
            }
            _ => match self.apply_input(&expr.input, root) {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("Error: failed to apply transform; reason: {} ", e.to_string());
                    None
                }
            }
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
                    eprintln!("Error: failed to apply JsonPath expression '{}'", expr.jpath);
                    None
                }
            }?;
        }
        for transform in expr.transforms {
            if let Some(builtin) = self.builtins.get(&transform.0) {
                match builtin(self, value, &transform.1) {
                    Some(new_value) => value = new_value,
                    None => {
                        eprintln!("Error: failed to apply builtin transform '{}'", transform.0);
                        return None
                    }
                }
            }
            else {
                match self.apply_input(&transform.0, &value) {
                    Ok(new_value) => value = new_value,
                    Err(e) => {
                        eprintln!("Error: failed to apply input transform '{}'; reason: {}", transform.0, e.to_string());
                        return None
                    }
                }
            }
        }
        Some(value)
    }

    fn transform_value(&mut self, v: &Value, input: &Value) -> Value {
        match v {
            Value::String(string) => {
                if let Some(value) = self.transform_string(string, input) {
                    //println!("transform result: {}", value);
                    value
                }
                else {
                    //println!("transform result: {}", v);
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
    let mut spec = Jsonatr::new(&input)?;
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
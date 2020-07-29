use crate::helpers::*;
use serde::Deserialize;
use serde_json::Value;
use std::process::{Command, Stdio};
use regex::Regex;
use simple_error::*;
use std::io::{Write, Read};

#[derive(Debug, Deserialize, PartialEq, Clone)]
enum InputKind {
    INLINE, // inline JSON
    FILE,   // external JSON file
    COMMAND // external command; its output should either be a valid JSON, or otherwise is converted to a JSON string
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Input {
    name: String,
    kind: InputKind,
    #[serde(rename = "let")]
    lets: Option<Value>,
    source: Value,
    #[serde(default="Input::pass_stdin")]
    stdin: bool,
    #[serde(default)]
    args: Vec<String>
}

impl Input {
    pub fn pass_stdin() -> bool { true }
}

struct Expr {
    input: String,
    jpath: String,
    transforms: Vec<(String,Vec<String>)>
}

lazy_static! {
    static ref INPUT_RE: Regex = Regex::new(r"^\$([[:word:]]*)").unwrap();
    static ref TRANSFORM_RE: Regex = Regex::new(r"[ \t]*\|[ \t]*([[:word:]]+)[ \t]*(?:\([ \t]*([^)]*?)[ \t]*\))?[ \t]*$").unwrap();
    static ref SEP_RE: Regex = Regex::new(r"[ \t]*,[ \t]*").unwrap();
}

type Locals = Vec<std::collections::HashMap<String, Value>>;
type Builtin = fn(&mut Transformer, Value, &Vec<String>) -> Option<Value>;
type Builtins = std::collections::HashMap<String, Builtin>;

#[derive(Deserialize)]
pub struct Transformer {
    #[serde(rename = "use")]
    uses: Option<Vec<String>>,

    input: Option<Vec<Input>>,
    output: Option<Value>,

    #[serde(skip)]
    inputs: std::collections::HashMap<String, Input>,

    #[serde(skip)]
    locals: Locals,

    #[serde(skip)]
    builtins: Builtins
}

impl Transformer {
    pub fn empty() -> Transformer {
        let mut spec = Transformer {
            uses: None,
            input: None,
            output: None,
            inputs: Default::default(),
            locals: vec![],
            builtins: Default::default()
        };
        spec.add_builtins();
        spec
    }

    pub fn new(spec: &str) -> Result<Transformer, SimpleError> {
        let mut spec: Transformer = try_with!(serde_json::from_str(spec),"failed to parse JSON");
        spec.add_builtins();
        if let Some(uses) = spec.uses.clone() {
            for path in uses {
                spec.add_use(path)?;
            }
        }
        if let Some(inputs) = spec.input.clone() {
            for input in inputs {
                spec.add_input(input)?;
            }
        }
        Ok(spec)
    }

    pub fn merge(&mut self, other: &Transformer) -> Result<(), SimpleError> {
        if other.output.is_some() {
            self.add_output(other.output.as_ref().unwrap().clone())?
        }
        for input in other.inputs.values() {
            self.add_input(input.clone())?;
        }
        Ok(())
    }

    pub fn add_use(&mut self, path: String) -> Result<(), SimpleError> {
        let file = read_file(&path)?;
        let other = Transformer::new(&file)?;
        self.merge(&other)?;
        Ok(())
    }

    pub fn add_input(&mut self, input: Input) -> Result<(), SimpleError> {
        if self.builtins.contains_key(&input.name) {
            bail!("can't define input '{}' because of the builtin function with the same name", input.name)
        }
        if let Some(input2) = self.inputs.get(&input.name) {
            if input != *input2 {
                bail!("found conflicting definition of input '{}'", input.name)
            }
        }
        if let Some(l) = &input.lets {
            if l.as_object().is_none() {
                bail!("wrong 'let' clause of input '{}': should be an object", input.name)
            }
        }
        self.inputs.insert(input.name.clone(), input);
        Ok(())
    }

    pub fn add_output(&mut self, output: Value) -> Result<(), SimpleError> {
        if self.output.is_some() {
            bail!("double definition of output")
        }
        self.output = Some(output);
        Ok(())
    }

    fn add_builtins(&mut self)  {
        self.builtins.insert("unwrap".to_string(), Transformer::builtin_unwrap);
        self.builtins.insert("map".to_string(), Transformer::builtin_map);
        self.builtins.insert("ifelse".to_string(), Transformer::builtin_ifelse);
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
                        match self.apply_input_by_name(&args[0], &x) {
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

    // checks the value for non-emptiness/non-zeroness,
    // and assumes that there are two arguments: if_branch and else_branch transformers
    fn builtin_ifelse(&mut self, v: Value, args: &Vec<String>) -> Option<Value> {
        if args.len() != 2 {
            return None
        }
        let cond = match v.clone() {
            Value::Null => false,
            Value::Bool(x) => x,
            Value::Number(x) => {
                if let Some(n) = x.as_f64() { n != 0f64 }
                else if let Some(n) = x.as_i64() { n != 0i64 }
                else if let Some(n) = x.as_u64() { n != 0u64 }
                else { return None }
            },
            Value::Array(x) => !x.is_empty(),
            Value::String(x) => !x.is_empty(),
            Value::Object(x) => !x.is_empty()
        };
        let index = if cond { 0 } else { 1 };
        match self.apply_input_by_name(&args[index], &v) {
            Ok(res) => Some(res),
            Err(e) => {
                eprintln!("Error: failed to apply input transform '{}'; reason: {}", args[index], e.to_string());
                None
            }
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

    fn apply_input(&mut self, input: &Input, root: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let result: Value;
        match input.kind {
            InputKind::INLINE => {
                result = self.transform_value(&input.source, root);
            },
            InputKind::FILE => {
                if let Some(path) = input.source.as_str() {
                    let file = std::fs::read_to_string(path)?;
                    let value = serde_json::from_str(&file)?;
                    result = self.transform_value(&value, &root);
                }
                else {
                    bail!("non-string provided as source for input '{}'", input.name)
                }
            }
            InputKind::COMMAND => {
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
                            if input.stdin {
                                match &process.stdin.as_mut().unwrap().write(serde_json::to_string(root).unwrap().as_bytes()) {
                                    Err(_) => bail!("couldn't write to command stdin for input '{}'", input.name),
                                    Ok(_) => (),
                                }
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
        Ok(result)
    }

    fn apply_input_by_name(&mut self, name: &String, root: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        // first try to find the reference in some local scope
        for scope in self.locals.iter().rev() {
            if scope.contains_key(name) {
                return Ok(scope.get(name).unwrap().clone());
            }
        }
        // if none is found, it should be present in the inputs
        let input = require_with!(self.inputs.get(name), "found reference to unknown input '{}'", name).clone();
        let lets: serde_json::Map<String, Value> =
            match input.lets.clone() {
                None => serde_json::Map::new(),
                Some(lets) => require_with!(lets.as_object(),"let clause of input '{}' is not an object", name).clone()
            };
        let mut locals = std::collections::HashMap::new();
        for (k, v) in lets {
            locals.insert(k.clone(), self.transform_value(&v, root));
        }
        self.locals.push(locals);
        let result = self.apply_input(&input, root);
        self.locals.pop();
        result
    }

    pub fn transform(&mut self, input: &Value) -> Result<String, SimpleError> {
        let output = require_with!(self.output.clone(), "no output specified");
        let transformed_output = self.transform_value(&output, input);
        let result = try_with!(serde_json::to_string_pretty(&transformed_output), "failed to produce output");
        Ok(result)
    }

    fn transform_string(&mut self, text: &String, root: &Value) -> Option<Value> {
        let expr = self.parse_expr(text)?;
        let json = match expr.input.as_str() {
            "" => match root {
                Value::Null => None,
                x => Some(x.clone())
            }
            _ => match self.apply_input_by_name(&expr.input, root) {
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
                match self.apply_input_by_name(&transform.0, &value) {
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
                    value
                }
                else {
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

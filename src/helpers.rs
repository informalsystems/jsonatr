use serde_json::Value;
use std::io::{self, Read};
use simple_error::*;

pub fn read_file(path: &str) -> Result<String, SimpleError> {
    let file = try_with!(std::fs::read_to_string(path), "failed to read file");
    Ok(file)
}

pub fn parse_string(string: &str) -> Result<Value, SimpleError> {
    let value: Value = try_with!(serde_json::from_str(&string), "failed to parse JSON");
    Ok(value)
}

pub fn parse_file(path: &str) -> Result<Value, SimpleError> {
    let file = read_file(path)?;
    let value = parse_string(&file)?;
    Ok(value)
}

pub fn parse_stdin() -> Result<Value, SimpleError> {
    let mut buffer = String::new();
    try_with!(io::stdin().read_to_string(&mut buffer), "failed to read from STDIN");
    let value = parse_string(&buffer)?;
    Ok(value)
}

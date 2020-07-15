use jsonatr::transformer::*;
use std::process::{Command};
use serde_json::Value;

fn test_expect(file: &str, expect: &str) {
    let input = std::fs::read_to_string(file).unwrap();
    let mut spec = Transformer::new(&input).unwrap();
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

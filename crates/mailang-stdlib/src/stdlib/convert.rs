//! Type conversion built-in functions

use mailang_bytecode::Value;
use super::value_to_string;

pub fn builtin_to_string(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(value_to_string(&args[0])))
}

pub fn builtin_parse_int(args: &[Value]) -> Result<Value, String> {
    match &args[0] {
        Value::Str(s) => s.parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
        Value::Float(n) => Ok(Value::Int(*n as i64)),
        Value::Int(n) => Ok(Value::Int(*n)),
        _ => Err("parse_int expects a string or number".to_string()),
    }
}

pub fn builtin_parse_float(args: &[Value]) -> Result<Value, String> {
    match &args[0] {
        Value::Str(s) => s.parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
        Value::Int(n) => Ok(Value::Float(*n as f64)),
        Value::Float(n) => Ok(Value::Float(*n)),
        _ => Err("parse_float expects a string or number".to_string()),
    }
}

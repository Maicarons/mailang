//! String built-in functions

use mailang_bytecode::Value;

pub fn builtin_len(args: &[Value]) -> Result<Value, String> {
    match &args[0] {
        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
        Value::Array(a) => Ok(Value::Int(a.len() as i64)),
        Value::Map(m) => Ok(Value::Int(m.len() as i64)),
        Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
        _ => Err("len expects a string, array, map, or tuple".to_string()),
    }
}

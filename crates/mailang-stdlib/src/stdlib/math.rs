//! Math built-in functions

use mailang_bytecode::Value;

fn to_f64(v: &Value) -> Result<f64, String> {
    match v {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err("Expected a number".to_string()),
    }
}

pub fn builtin_sqrt(args: &[Value]) -> Result<Value, String> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Float((*n as f64).sqrt())),
        Value::Float(n) => Ok(Value::Float(n.sqrt())),
        _ => Err("sqrt expects a number".to_string()),
    }
}

pub fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(n) => Ok(Value::Float(n.abs())),
        _ => Err("abs expects a number".to_string()),
    }
}

pub fn builtin_sin(args: &[Value]) -> Result<Value, String> {
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.sin()))
}

pub fn builtin_cos(args: &[Value]) -> Result<Value, String> {
    let n = to_f64(&args[0])?;
    Ok(Value::Float(n.cos()))
}

pub fn builtin_floor(args: &[Value]) -> Result<Value, String> {
    let n = to_f64(&args[0])?;
    Ok(Value::Int(n.floor() as i64))
}

pub fn builtin_ceil(args: &[Value]) -> Result<Value, String> {
    let n = to_f64(&args[0])?;
    Ok(Value::Int(n.ceil() as i64))
}

pub fn builtin_round(args: &[Value]) -> Result<Value, String> {
    let n = to_f64(&args[0])?;
    Ok(Value::Int(n.round() as i64))
}

pub fn builtin_min(args: &[Value]) -> Result<Value, String> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        _ => Err("min expects numbers".to_string()),
    }
}

pub fn builtin_max(args: &[Value]) -> Result<Value, String> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        _ => Err("max expects numbers".to_string()),
    }
}

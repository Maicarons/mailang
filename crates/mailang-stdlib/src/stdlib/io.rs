//! I/O built-in functions

use super::value_to_string;
use mailang_bytecode::Value;
use std::io::{self, Write};

#[cfg(feature = "capture-output")]
use super::capture;

pub fn builtin_println(args: &[Value]) -> Result<Value, String> {
    let parts: Vec<String> = args.iter().map(value_to_string).collect();
    let line = parts.join(" ");

    #[cfg(feature = "capture-output")]
    {
        if capture::is_captured() {
            capture::push_line(line);
            return Ok(Value::Null);
        }
    }

    println!("{}", line);
    Ok(Value::Null)
}

pub fn builtin_print(args: &[Value]) -> Result<Value, String> {
    let parts: Vec<String> = args.iter().map(value_to_string).collect();
    let text = parts.join(" ");

    #[cfg(feature = "capture-output")]
    {
        if capture::is_captured() {
            capture::push_text(text);
            return Ok(Value::Null);
        }
    }

    print!("{}", text);
    io::stdout().flush().map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn builtin_input(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        print!("{}", value_to_string(&args[0]));
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    Ok(Value::Str(input.trim_end().into()))
}

//! MaìLang standard library
//!
//! This module provides built-in functions for MaìLang.

mod io;
mod math;
mod string;
mod time;
mod convert;

pub use io::*;
pub use math::*;
pub use string::*;
pub use time::*;
pub use convert::*;

use mailang_bytecode::Value;

// Output capture for WASM builds
#[cfg(feature = "capture-output")]
mod capture {
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        static ref CAPTURED_OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
        static ref IS_CAPTURED: Mutex<bool> = Mutex::new(false);
    }

    pub fn set_captured(captured: bool) -> bool {
        let mut is_captured = IS_CAPTURED.lock().unwrap();
        let prev = *is_captured;
        *is_captured = captured;
        if captured {
            CAPTURED_OUTPUT.lock().unwrap().clear();
        }
        prev
    }

    pub fn get_output() -> Vec<String> {
        CAPTURED_OUTPUT.lock().unwrap().clone()
    }

    pub fn is_captured() -> bool {
        *IS_CAPTURED.lock().unwrap()
    }

    pub fn push_line(line: String) {
        CAPTURED_OUTPUT.lock().unwrap().push(line);
    }

    pub fn push_text(text: String) {
        let mut output = CAPTURED_OUTPUT.lock().unwrap();
        if let Some(last) = output.last_mut() {
            last.push_str(&text);
        } else {
            output.push(text);
        }
    }
}

/// Enable or disable output capture mode (only available with capture-output feature).
#[cfg(feature = "capture-output")]
pub fn set_println_captured(captured: bool) -> bool {
    capture::set_captured(captured)
}

/// Get the captured output lines (only available with capture-output feature).
#[cfg(feature = "capture-output")]
pub fn get_captured_output() -> Vec<String> {
    capture::get_output()
}

pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Str(s) => s.to_string(),
        Value::Char(c) => c.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.borrow().iter().map(value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Map(m) => {
            let items: Vec<String> = m
                .borrow()
                .iter()
                .map(|(k, v)| format!("{}: {}", value_to_string(k), value_to_string(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.borrow().iter().map(value_to_string).collect();
            format!("({})", items.join(", "))
        }
        Value::Function(f) => format!("<fn {}>", f.name),
        Value::Closure(c) => format!("<closure {}>", c.function_index),
        Value::Class(cls) => format!("<class {}>", cls.name),
        Value::Instance { class_index, .. } => format!("<instance {}>", class_index),
        Value::Ok(v) => format!("Ok({})", value_to_string(v)),
        Value::Err(v) => format!("Err({})", value_to_string(v)),
        Value::Some(v) => format!("Some({})", value_to_string(v)),
        Value::Builtin { name, .. } => format!("<builtin {}>", name),
    }
}

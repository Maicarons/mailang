//! Process / environment builtins.

use mailang_bytecode::Value;

/// `env(name)` → str | null
pub fn builtin_env(args: &[Value]) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Err("env(name): name must be string".into()),
    };
    match std::env::var(&name) {
        Ok(v) => Ok(Value::Str(v.into())),
        Err(_) => Ok(Value::Null),
    }
}

/// `process_exit(code)` — terminate the process with `code` (never returns).
pub fn builtin_process_exit(args: &[Value]) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::Int(n)) => *n as i32,
        _ => return Err("process_exit(code): code must be int".into()),
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_missing_is_null() {
        let v = builtin_env(&[Value::Str("MAILANG_DEFINITELY_UNSET_VAR_XYZ".into())]).unwrap();
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn env_reads_present_var() {
        std::env::set_var("MAILANG_TEST_ENV", "hello");
        let v = builtin_env(&[Value::Str("MAILANG_TEST_ENV".into())]).unwrap();
        assert_eq!(v, Value::Str("hello".into()));
    }

    #[test]
    fn env_rejects_non_string() {
        assert!(builtin_env(&[Value::Int(1)]).is_err());
    }
}

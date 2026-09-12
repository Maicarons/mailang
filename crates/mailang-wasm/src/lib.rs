use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmInterpreter {
    inner: mailang_core::MailangInterpreter,
}

#[wasm_bindgen]
impl WasmInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: mailang_core::MailangInterpreter::new(),
        }
    }

    /// Evaluate MaìLang code and return the output.
    /// Returns captured println lines, or the expression result if none.
    pub fn eval(&mut self, code: &str) -> String {
        mailang_stdlib::set_println_captured(true);

        let result = match self.inner.eval(code) {
            Ok(value_str) => {
                let captured = mailang_stdlib::get_captured_output();
                if captured.is_empty() {
                    value_str
                } else {
                    captured.join("\n")
                }
            }
            Err(e) => format!("Error: {}", e),
        };

        mailang_stdlib::set_println_captured(false);
        result
    }

    /// Evaluate code and return a structured result.
    /// Returns a JSON string: {"ok": true, "output": "..."} or {"ok": false, "error": "..."}
    pub fn eval_json(&mut self, code: &str) -> String {
        // Enable output capture
        mailang_stdlib::set_println_captured(true);

        let result = match self.inner.eval(code) {
            Ok(_) => {
                // Get captured output from stdlib
                let output = mailang_stdlib::get_captured_output();
                let output_str = if output.is_empty() {
                    String::new()
                } else {
                    output.join("\n")
                };
                format!(r#"{{"ok":true,"output":"{}"}}"#, escape_json(&output_str))
            }
            Err(e) => {
                format!(r#"{{"ok":false,"error":"{}"}}"#, escape_json(&e))
            }
        };

        // Disable output capture
        mailang_stdlib::set_println_captured(false);

        result
    }

    /// Reset the interpreter state (clear globals, etc.)
    pub fn reset(&mut self) {
        self.inner = mailang_core::MailangInterpreter::new();
    }

    /// Get the version of the MaìLang interpreter.
    pub fn version() -> String {
        "0.1.0".to_string()
    }

    /// Get supported features as a comma-separated string.
    pub fn features() -> String {
        "variables,functions,loops,arrays,lambdas,builtins".to_string()
    }
}

impl Default for WasmInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

//! Minimal JSON encode/decode (`json_parse` / `json_stringify`).
//!
//! Hand-rolled parser — no external crates. Supports objects, arrays, strings
//! (with common escapes and `\uXXXX`), numbers, `true`/`false`/`null`.

use core::cell::RefCell;
use mailang_bytecode::Value;
use std::rc::Rc;

/// `json_parse(s)` → map/array/str/num/bool/null
pub fn builtin_json_parse(args: &[Value]) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Err("json_parse(s): s must be string".into()),
    };
    let mut p = Parser::new(&s);
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("json_parse: trailing data at {}", p.pos));
    }
    Ok(v)
}

/// `json_stringify(v)` → str (compact JSON)
pub fn builtin_json_stringify(args: &[Value]) -> Result<Value, String> {
    let v = args.first().ok_or("json_stringify(v): missing value")?;
    let mut out = String::new();
    write_json(v, &mut out)?;
    Ok(Value::Str(out.into()))
}

fn write_json(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(n) => {
            if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&n.to_string());
            }
        }
        Value::Str(s) => write_json_string(s, out),
        Value::Char(c) => write_json_string(&c.to_string(), out),
        Value::Array(a) => {
            out.push('[');
            for (i, item) in a.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out)?;
            }
            out.push(']');
        }
        Value::Tuple(a) => {
            out.push('[');
            for (i, item) in a.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out)?;
            }
            out.push(']');
        }
        Value::Map(m) => {
            out.push('{');
            for (i, (k, val)) in m.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match k {
                    Value::Str(s) => write_json_string(s, out),
                    Value::Char(c) => write_json_string(&c.to_string(), out),
                    Value::Int(n) => write_json_string(&n.to_string(), out),
                    Value::Bool(b) => write_json_string(&b.to_string(), out),
                    other => {
                        return Err(format!("json_stringify: unsupported map key {:?}", other))
                    }
                }
                out.push(':');
                write_json(val, out)?;
            }
            out.push('}');
        }
        Value::Ok(inner) | Value::Some(inner) => write_json(inner, out)?,
        other => return Err(format!("json_stringify: unsupported type {:?}", other)),
    }
    Ok(())
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), String> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "json_parse: expected '{}' at {}",
                b as char, self.pos
            ))
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::Str(self.parse_string()?.into())),
            Some(b't') => self.parse_lit("true", Value::Bool(true)),
            Some(b'f') => self.parse_lit("false", Value::Bool(false)),
            Some(b'n') => self.parse_lit("null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(format!(
                "json_parse: unexpected {:?} at {}",
                other, self.pos
            )),
        }
    }

    fn parse_lit(&mut self, lit: &str, v: Value) -> Result<Value, String> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(v)
        } else {
            Err(format!("json_parse: expected '{}' at {}", lit, self.pos))
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut entries: Vec<(Value, Value)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Map(Rc::new(RefCell::new(entries))));
        }
        loop {
            self.skip_ws();
            let key = Value::Str(self.parse_string()?.into());
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let val = self.parse_value()?;
            entries.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                other => {
                    return Err(format!(
                        "json_parse: expected ',' or '}}' at {} (got {:?})",
                        self.pos, other
                    ))
                }
            }
        }
        Ok(Value::Map(Rc::new(RefCell::new(entries))))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items: Vec<Value> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(Rc::new(RefCell::new(items))));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                other => {
                    return Err(format!(
                        "json_parse: expected ',' or ']' at {} (got {:?})",
                        self.pos, other
                    ))
                }
            }
        }
        Ok(Value::Array(Rc::new(RefCell::new(items))))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            let c = self.peek().ok_or("json_parse: unterminated string")?;
            self.pos += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or("json_parse: bad escape")?;
                    self.pos += 1;
                    match e {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'b' => s.push('\u{08}'),
                        b'f' => s.push('\u{0c}'),
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.pos..self.pos + 4)
                                .ok_or("json_parse: short \\u")?;
                            self.pos += 4;
                            let code = u16::from_str_radix(
                                std::str::from_utf8(hex).map_err(|e| e.to_string())?,
                                16,
                            )
                            .map_err(|e| e.to_string())?;
                            // Basic BMP only (no surrogate pairs).
                            s.push(char::from_u32(code as u32).unwrap_or('\u{fffd}'));
                        }
                        other => return Err(format!("json_parse: bad escape \\{}", other as char)),
                    }
                }
                // Multi-byte UTF-8: copy the lead byte and its continuation bytes.
                c if c >= 0x80 => {
                    let start = self.pos - 1;
                    let extra = match c {
                        0xC0..=0xDF => 1,
                        0xE0..=0xEF => 2,
                        _ => 3,
                    };
                    self.pos = start + 1 + extra;
                    let chunk = self
                        .bytes
                        .get(start..self.pos)
                        .ok_or("json_parse: bad utf-8")?;
                    s.push_str(std::str::from_utf8(chunk).map_err(|e| e.to_string())?);
                }
                c => s.push(c as char),
            }
        }
        Ok(s)
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        if text.is_empty() || text == "-" {
            return Err(format!("json_parse: bad number at {}", start));
        }
        if !is_float {
            if let Ok(n) = text.parse::<i64>() {
                return Ok(Value::Int(n));
            }
        }
        text.parse::<f64>()
            .map(Value::Float)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        Value::Str(x.into())
    }

    #[test]
    fn parse_scalars() {
        assert_eq!(builtin_json_parse(&[s("null")]).unwrap(), Value::Null);
        assert_eq!(builtin_json_parse(&[s("true")]).unwrap(), Value::Bool(true));
        assert_eq!(
            builtin_json_parse(&[s("false")]).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(builtin_json_parse(&[s("42")]).unwrap(), Value::Int(42));
        assert_eq!(builtin_json_parse(&[s("-7")]).unwrap(), Value::Int(-7));
        assert_eq!(builtin_json_parse(&[s("3.5")]).unwrap(), Value::Float(3.5));
        assert_eq!(builtin_json_parse(&[s("\"hi\"")]).unwrap(), s("hi"));
    }

    #[test]
    fn parse_nested_and_escapes() {
        let v = builtin_json_parse(&[s(r#"{"a":[1,2,{"b":"x\ny"}],"c":null}"#)]).unwrap();
        let out = builtin_json_stringify(&[v.clone()]).unwrap();
        assert_eq!(out, s(r#"{"a":[1,2,{"b":"x\ny"}],"c":null}"#));
    }

    #[test]
    fn stringify_roundtrip() {
        let src = r#"[{"k":1,"ok":true},{"n":-2.5,"s":"q\"z"}]"#;
        let v = builtin_json_parse(&[s(src)]).unwrap();
        let out = builtin_json_stringify(&[v]).unwrap();
        assert_eq!(out, s(src));
    }

    #[test]
    fn roundtrip_map_from_rust() {
        let m = Value::Map(Rc::new(RefCell::new(vec![
            (s("x"), Value::Int(1)),
            (
                s("y"),
                Value::Array(Rc::new(RefCell::new(vec![Value::Bool(false), Value::Null]))),
            ),
        ])));
        let text = builtin_json_stringify(&[m]).unwrap();
        assert_eq!(text, s(r#"{"x":1,"y":[false,null]}"#));
        let back = builtin_json_parse(&[text]).unwrap();
        let again = builtin_json_stringify(&[back]).unwrap();
        assert_eq!(again, s(r#"{"x":1,"y":[false,null]}"#));
    }

    #[test]
    fn parse_rejects_trailing() {
        assert!(builtin_json_parse(&[s("1 2")]).is_err());
    }
}

//! Binary bytecode container format (`.mailangbc`).
//!
//! Layout (all multi-byte integers little-endian):
//!
//! ```text
//! magic:   b"MAILBC01"          (8 bytes)
//! version: u16                  (currently 1)
//! flags:   u16                  (reserved, 0)
//! main:    u32                  (main chunk index)
//! n_globals: u32
//!   for each global: u32 len + utf8 bytes
//! n_chunks: u32
//!   for each chunk:
//!     name: u32 len + utf8 bytes
//!     n_const: u32
//!       for each constant: Value
//!     n_instr: u32
//!       for each instruction: opcode u8, has_operand u8, operand u32 (if has_operand), line u32
//! ```
//!
//! Value tags (u8):
//! 0 Null, 1 Bool, 2 Int, 3 Float, 4 Str, 5 Char,
//! 6 Array, 7 Map, 8 Tuple, 9 Function, 10 Closure,
//! 11 Class, 12 Instance, 13 Ok, 14 Err, 15 Some, 16 Builtin

use crate::{Bytecode, Chunk, ClassObj, ClosureObj, FunctionObj, Instruction, Opcode, Value};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

pub const FORMAT_MAGIC: &[u8; 8] = b"MAILBC01";
pub const FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeFormatError {
    Truncated,
    BadMagic,
    UnsupportedVersion(u16),
    InvalidOpcode(u8),
    InvalidTag(u8),
    InvalidUtf8,
    TrailingData,
}

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Null => self.u8(0),
            Value::Bool(b) => {
                self.u8(1);
                self.u8(*b as u8);
            }
            Value::Int(n) => {
                self.u8(2);
                self.i64(*n);
            }
            Value::Float(n) => {
                self.u8(3);
                self.f64(*n);
            }
            Value::Str(s) => {
                self.u8(4);
                self.str(s);
            }
            Value::Char(c) => {
                self.u8(5);
                self.u32(*c as u32);
            }
            Value::Array(items) => {
                self.u8(6);
                let items = items.borrow();
                self.u32(items.len() as u32);
                for item in items.iter() {
                    self.value(item);
                }
            }
            Value::Map(entries) => {
                self.u8(7);
                let entries = entries.borrow();
                self.u32(entries.len() as u32);
                for (k, val) in entries.iter() {
                    self.value(k);
                    self.value(val);
                }
            }
            Value::Tuple(items) => {
                self.u8(8);
                let items = items.borrow();
                self.u32(items.len() as u32);
                for item in items.iter() {
                    self.value(item);
                }
            }
            Value::Function(f) => {
                self.u8(9);
                self.str(&f.name);
                self.u32(f.arity as u32);
                self.u32(f.chunk_index as u32);
            }
            Value::Closure(c) => {
                self.u8(10);
                self.u32(c.function_index as u32);
                self.u32(c.arity as u32);
                self.u32(c.upvalues.len() as u32);
                for u in &c.upvalues {
                    self.u32(*u as u32);
                }
            }
            Value::Class(cls) => {
                self.u8(11);
                self.str(&cls.name);
                self.u32(cls.methods.len() as u32);
                for (mname, ci) in cls.methods.iter() {
                    self.str(mname);
                    self.u32(*ci as u32);
                }
                match &cls.superclass {
                    Some(s) => {
                        self.u8(1);
                        self.str(s);
                    }
                    None => self.u8(0),
                }
                self.u32(cls.properties.len() as u32);
                for (pname, pval) in cls.properties.iter() {
                    self.str(pname);
                    self.value(pval);
                }
            }
            Value::Instance {
                class_index,
                fields,
            } => {
                self.u8(12);
                self.u32(*class_index as u32);
                let fields = fields.borrow();
                self.u32(fields.len() as u32);
                for (fname, fval) in fields.iter() {
                    self.str(fname);
                    self.value(fval);
                }
            }
            Value::Ok(inner) => {
                self.u8(13);
                self.value(inner);
            }
            Value::Err(inner) => {
                self.u8(14);
                self.value(inner);
            }
            Value::Some(inner) => {
                self.u8(15);
                self.value(inner);
            }
            Value::Builtin { name, arity } => {
                self.u8(16);
                self.str(name);
                self.u32(*arity as u32);
            }
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], BytecodeFormatError> {
        if self.remaining() < n {
            return Err(BytecodeFormatError::Truncated);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, BytecodeFormatError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BytecodeFormatError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, BytecodeFormatError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, BytecodeFormatError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    fn i64(&mut self) -> Result<i64, BytecodeFormatError> {
        Ok(self.u64()? as i64)
    }

    fn f64(&mut self) -> Result<f64, BytecodeFormatError> {
        Ok(f64::from_bits(self.u64()?))
    }

    fn str(&mut self) -> Result<String, BytecodeFormatError> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        core::str::from_utf8(bytes)
            .map(|s| s.to_string())
            .map_err(|_| BytecodeFormatError::InvalidUtf8)
    }

    fn value(&mut self) -> Result<Value, BytecodeFormatError> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(Value::Null),
            1 => Ok(Value::Bool(self.u8()? != 0)),
            2 => Ok(Value::Int(self.i64()?)),
            3 => Ok(Value::Float(self.f64()?)),
            4 => {
                let s = self.str()?;
                Ok(Value::Str(Rc::from(s.as_str())))
            }
            5 => {
                let cp = self.u32()?;
                let c = char::from_u32(cp).ok_or(BytecodeFormatError::InvalidTag(tag))?;
                Ok(Value::Char(c))
            }
            6 => {
                let n = self.u32()? as usize;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.value()?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(items))))
            }
            7 => {
                let n = self.u32()? as usize;
                let mut entries = Vec::with_capacity(n);
                for _ in 0..n {
                    let k = self.value()?;
                    let v = self.value()?;
                    entries.push((k, v));
                }
                Ok(Value::Map(Rc::new(RefCell::new(entries))))
            }
            8 => {
                let n = self.u32()? as usize;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.value()?);
                }
                Ok(Value::Tuple(Rc::new(RefCell::new(items))))
            }
            9 => {
                let name = self.str()?;
                let arity = self.u32()? as usize;
                let chunk_index = self.u32()? as usize;
                Ok(Value::Function(Rc::new(FunctionObj {
                    name: Rc::from(name.as_str()),
                    arity,
                    chunk_index,
                })))
            }
            10 => {
                let function_index = self.u32()? as usize;
                let arity = self.u32()? as usize;
                let n = self.u32()? as usize;
                let mut upvalues = Vec::with_capacity(n);
                for _ in 0..n {
                    upvalues.push(self.u32()? as usize);
                }
                Ok(Value::Closure(Rc::new(ClosureObj {
                    function_index,
                    arity,
                    upvalues,
                })))
            }
            11 => {
                let name = self.str()?;
                let n_methods = self.u32()? as usize;
                let mut methods = Vec::with_capacity(n_methods);
                for _ in 0..n_methods {
                    let mname = self.str()?;
                    let ci = self.u32()? as usize;
                    methods.push((mname, ci));
                }
                let has_super = self.u8()?;
                let superclass = if has_super == 1 {
                    Some(Rc::from(self.str()?.as_str()))
                } else {
                    None
                };
                let n_props = self.u32()? as usize;
                let mut properties = Vec::with_capacity(n_props);
                for _ in 0..n_props {
                    let pname = self.str()?;
                    let pval = self.value()?;
                    properties.push((pname, pval));
                }
                Ok(Value::Class(Rc::new(ClassObj {
                    name: Rc::from(name.as_str()),
                    methods: Rc::new(methods),
                    superclass,
                    properties: Rc::new(properties),
                })))
            }
            12 => {
                let class_index = self.u32()? as usize;
                let n = self.u32()? as usize;
                let mut fields = Vec::with_capacity(n);
                for _ in 0..n {
                    let fname = self.str()?;
                    let fval = self.value()?;
                    fields.push((fname, fval));
                }
                Ok(Value::Instance {
                    class_index,
                    fields: Rc::new(RefCell::new(fields)),
                })
            }
            13 => Ok(Value::Ok(Box::new(self.value()?))),
            14 => Ok(Value::Err(Box::new(self.value()?))),
            15 => Ok(Value::Some(Box::new(self.value()?))),
            16 => {
                let name = self.str()?;
                let arity = self.u32()? as usize;
                Ok(Value::Builtin {
                    name: Rc::from(name.as_str()),
                    arity,
                })
            }
            other => Err(BytecodeFormatError::InvalidTag(other)),
        }
    }
}

/// Encode bytecode into the `.mailangbc` container.
pub fn encode(bc: &Bytecode) -> Vec<u8> {
    let mut w = Writer::new();
    w.buf.extend_from_slice(FORMAT_MAGIC);
    w.u16(FORMAT_VERSION);
    w.u16(0); // flags
    w.u32(bc.main_chunk as u32);

    w.u32(bc.global_names.len() as u32);
    for name in &bc.global_names {
        w.str(name);
    }

    w.u32(bc.chunks.len() as u32);
    for chunk in &bc.chunks {
        w.str(&chunk.name);
        w.u32(chunk.constants.len() as u32);
        for c in &chunk.constants {
            w.value(c);
        }
        w.u32(chunk.instructions.len() as u32);
        for ins in &chunk.instructions {
            w.u8(ins.opcode.as_u8());
            match ins.operand {
                Some(op) => {
                    w.u8(1);
                    w.u32(op);
                }
                None => w.u8(0),
            }
            w.u32(ins.line);
        }
    }
    w.buf
}

/// Decode a `.mailangbc` container into bytecode.
pub fn decode(data: &[u8]) -> Result<Bytecode, BytecodeFormatError> {
    let mut r = Reader::new(data);
    let magic = r.take(8)?;
    if magic != FORMAT_MAGIC {
        return Err(BytecodeFormatError::BadMagic);
    }
    let version = r.u16()?;
    if version != FORMAT_VERSION {
        return Err(BytecodeFormatError::UnsupportedVersion(version));
    }
    let _flags = r.u16()?;
    let main_chunk = r.u32()? as usize;

    let n_globals = r.u32()? as usize;
    let mut global_names = Vec::with_capacity(n_globals);
    for _ in 0..n_globals {
        global_names.push(r.str()?);
    }

    let n_chunks = r.u32()? as usize;
    let mut chunks = Vec::with_capacity(n_chunks);
    for _ in 0..n_chunks {
        let name = r.str()?;
        let n_const = r.u32()? as usize;
        let mut constants = Vec::with_capacity(n_const);
        for _ in 0..n_const {
            constants.push(r.value()?);
        }
        let n_instr = r.u32()? as usize;
        let mut instructions = Vec::with_capacity(n_instr);
        for _ in 0..n_instr {
            let op_u8 = r.u8()?;
            let opcode = Opcode::from_u8(op_u8).ok_or(BytecodeFormatError::InvalidOpcode(op_u8))?;
            let has_operand = r.u8()?;
            let operand = if has_operand == 1 {
                Some(r.u32()?)
            } else {
                None
            };
            let line = r.u32()?;
            instructions.push(Instruction {
                opcode,
                operand,
                line,
            });
        }
        chunks.push(Chunk {
            instructions,
            constants,
            name,
        });
    }

    if r.remaining() != 0 {
        return Err(BytecodeFormatError::TrailingData);
    }

    Ok(Bytecode {
        chunks,
        main_chunk,
        global_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bytecode;

    #[test]
    fn roundtrip_empty() {
        let bc = Bytecode::new();
        let bytes = encode(&bc);
        let back = decode(&bytes).unwrap();
        assert_eq!(bc, back);
    }

    #[test]
    fn roundtrip_simple_program() {
        let mut bc = Bytecode::new();
        let slot = bc.intern_global("x");
        let main = &mut bc.chunks[0];
        main.emit(Opcode::Push, Some(0), 1);
        main.emit(Opcode::StoreGlobal, Some(slot), 1);
        main.emit(Opcode::Halt, None, 2);
        main.add_constant(Value::Int(42));

        let bytes = encode(&bc);
        let back = decode(&bytes).unwrap();
        assert_eq!(bc, back);
    }

    #[test]
    fn reject_bad_magic() {
        assert_eq!(decode(b"NOTMAGIC0000"), Err(BytecodeFormatError::BadMagic));
    }
}

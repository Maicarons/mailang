#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

#[cfg(all(feature = "serde", feature = "std"))]
use serde::{Deserialize, Serialize};

pub mod format;
pub use format::{decode, encode, BytecodeFormatError, FORMAT_MAGIC, FORMAT_VERSION};

/// Opcode is `Copy` and serialized as a single byte in the binary format.
/// Keep variants in a stable order; append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum Opcode {
    // Stack operations
    Push = 0,
    Pop,
    Dup,

    // Load/Store
    LoadLocal,
    StoreLocal,
    LoadGlobal,
    StoreGlobal,
    LoadUpvalue,
    StoreUpvalue,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Neg,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,
    Not,

    // Control flow
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Call,
    TailCall,
    Return,

    // Object operations
    GetProperty,
    SetProperty,
    Invoke,

    // Array/Map
    BuildArray,
    BuildMap,
    IndexGet,
    IndexSet,

    // Class/Trait
    CreateClass,
    CreateInstance,
    GetMethod,

    // Pattern matching
    MatchPattern,

    // Closure creation
    MakeClosure,

    // Error handling
    Throw,
    TryBegin,
    TryEnd,
    WrapOk,
    WrapErr,
    WrapSome,
    UnwrapOk,
    UnwrapErr,
    UnwrapSome,

    // Special
    Nop,
    Halt,
}

impl Opcode {
    /// Stable discriminant used by the binary bytecode format.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Opcode::Push,
            1 => Opcode::Pop,
            2 => Opcode::Dup,
            3 => Opcode::LoadLocal,
            4 => Opcode::StoreLocal,
            5 => Opcode::LoadGlobal,
            6 => Opcode::StoreGlobal,
            7 => Opcode::LoadUpvalue,
            8 => Opcode::StoreUpvalue,
            9 => Opcode::Add,
            10 => Opcode::Sub,
            11 => Opcode::Mul,
            12 => Opcode::Div,
            13 => Opcode::Mod,
            14 => Opcode::Pow,
            15 => Opcode::Neg,
            16 => Opcode::BitAnd,
            17 => Opcode::BitOr,
            18 => Opcode::BitXor,
            19 => Opcode::BitNot,
            20 => Opcode::Shl,
            21 => Opcode::Shr,
            22 => Opcode::Eq,
            23 => Opcode::Ne,
            24 => Opcode::Lt,
            25 => Opcode::Le,
            26 => Opcode::Gt,
            27 => Opcode::Ge,
            28 => Opcode::And,
            29 => Opcode::Or,
            30 => Opcode::Not,
            31 => Opcode::Jump,
            32 => Opcode::JumpIfFalse,
            33 => Opcode::JumpIfTrue,
            34 => Opcode::Call,
            35 => Opcode::TailCall,
            36 => Opcode::Return,
            37 => Opcode::GetProperty,
            38 => Opcode::SetProperty,
            39 => Opcode::Invoke,
            40 => Opcode::BuildArray,
            41 => Opcode::BuildMap,
            42 => Opcode::IndexGet,
            43 => Opcode::IndexSet,
            44 => Opcode::CreateClass,
            45 => Opcode::CreateInstance,
            46 => Opcode::GetMethod,
            47 => Opcode::MatchPattern,
            48 => Opcode::MakeClosure,
            49 => Opcode::Throw,
            50 => Opcode::TryBegin,
            51 => Opcode::TryEnd,
            52 => Opcode::WrapOk,
            53 => Opcode::WrapErr,
            54 => Opcode::WrapSome,
            55 => Opcode::UnwrapOk,
            56 => Opcode::UnwrapErr,
            57 => Opcode::UnwrapSome,
            58 => Opcode::Nop,
            59 => Opcode::Halt,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Char(char),
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Tuple(Rc<RefCell<Vec<Value>>>),
    Function {
        name: Rc<str>,
        arity: usize,
        chunk_index: usize,
    },
    Closure {
        function_index: usize,
        arity: usize,
        upvalues: Vec<usize>,
    },
    Class {
        name: Rc<str>,
        methods: Rc<Vec<(String, usize)>>,
        superclass: Option<String>,
        properties: Rc<Vec<(String, Value)>>,
    },
    Instance {
        class_index: usize,
        fields: Rc<RefCell<Vec<(String, Value)>>>,
    },
    Ok(Box<Value>),
    Err(Box<Value>),
    Some(Box<Value>),
    Builtin {
        name: Rc<str>,
        arity: usize,
    },
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => *a.borrow() == *b.borrow(),
            (Value::Map(a), Value::Map(b)) => *a.borrow() == *b.borrow(),
            (Value::Tuple(a), Value::Tuple(b)) => *a.borrow() == *b.borrow(),
            (
                Value::Function {
                    name: n1,
                    arity: a1,
                    chunk_index: c1,
                },
                Value::Function {
                    name: n2,
                    arity: a2,
                    chunk_index: c2,
                },
            ) => n1 == n2 && a1 == a2 && c1 == c2,
            (
                Value::Closure {
                    function_index: f1,
                    arity: a1,
                    upvalues: u1,
                },
                Value::Closure {
                    function_index: f2,
                    arity: a2,
                    upvalues: u2,
                },
            ) => f1 == f2 && a1 == a2 && u1 == u2,
            (
                Value::Class {
                    name: n1,
                    methods: m1,
                    superclass: s1,
                    properties: p1,
                },
                Value::Class {
                    name: n2,
                    methods: m2,
                    superclass: s2,
                    properties: p2,
                },
            ) => n1 == n2 && m1 == m2 && s1 == s2 && p1 == p2,
            (
                Value::Instance {
                    class_index: c1,
                    fields: f1,
                },
                Value::Instance {
                    class_index: c2,
                    fields: f2,
                },
            ) => c1 == c2 && *f1.borrow() == *f2.borrow(),
            (Value::Ok(a), Value::Ok(b)) => a == b,
            (Value::Err(a), Value::Err(b)) => a == b,
            (Value::Some(a), Value::Some(b)) => a == b,
            (
                Value::Builtin {
                    name: n1,
                    arity: a1,
                },
                Value::Builtin {
                    name: n2,
                    arity: a2,
                },
            ) => n1 == n2 && a1 == a2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Option<u32>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub name: String,
}

impl Chunk {
    pub fn new(name: String) -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            name,
        }
    }

    pub fn emit(&mut self, opcode: Opcode, operand: Option<u32>, line: usize) {
        self.instructions.push(Instruction {
            opcode,
            operand,
            line,
        });
    }

    pub fn add_constant(&mut self, value: Value) -> u32 {
        let index = self.constants.len() as u32;
        self.constants.push(value);
        index
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct Bytecode {
    pub chunks: Vec<Chunk>,
    pub main_chunk: usize,
    /// Global variable names indexed by slot ID. Used for LoadGlobal/StoreGlobal.
    pub global_names: Vec<String>,
}

impl Bytecode {
    pub fn new() -> Self {
        let main_chunk = Chunk::new("main".to_string());
        Self {
            chunks: vec![main_chunk],
            main_chunk: 0,
            global_names: Vec::new(),
        }
    }

    /// Register a global name and return its slot index.
    pub fn intern_global(&mut self, name: &str) -> u32 {
        if let Some(pos) = self.global_names.iter().position(|n| n == name) {
            return pos as u32;
        }
        let idx = self.global_names.len() as u32;
        self.global_names.push(name.to_string());
        idx
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

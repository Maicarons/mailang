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
    // Fused int immediate ops (hot path: n-1, n<=1, ...)
    AddImm,
    SubImm,
    MulImm,
    EqImm,
    NeImm,
    LtImm,
    LeImm,
    GtImm,
    GeImm,

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
            28 => Opcode::AddImm,
            29 => Opcode::SubImm,
            30 => Opcode::MulImm,
            31 => Opcode::EqImm,
            32 => Opcode::NeImm,
            33 => Opcode::LtImm,
            34 => Opcode::LeImm,
            35 => Opcode::GtImm,
            36 => Opcode::GeImm,
            37 => Opcode::And,
            38 => Opcode::Or,
            39 => Opcode::Not,
            40 => Opcode::Jump,
            41 => Opcode::JumpIfFalse,
            42 => Opcode::JumpIfTrue,
            43 => Opcode::Call,
            44 => Opcode::TailCall,
            45 => Opcode::Return,
            46 => Opcode::GetProperty,
            47 => Opcode::SetProperty,
            48 => Opcode::Invoke,
            49 => Opcode::BuildArray,
            50 => Opcode::BuildMap,
            51 => Opcode::IndexGet,
            52 => Opcode::IndexSet,
            53 => Opcode::CreateClass,
            54 => Opcode::CreateInstance,
            55 => Opcode::GetMethod,
            56 => Opcode::MatchPattern,
            57 => Opcode::MakeClosure,
            58 => Opcode::Throw,
            59 => Opcode::TryBegin,
            60 => Opcode::TryEnd,
            61 => Opcode::WrapOk,
            62 => Opcode::WrapErr,
            63 => Opcode::WrapSome,
            64 => Opcode::UnwrapOk,
            65 => Opcode::UnwrapErr,
            66 => Opcode::UnwrapSome,
            67 => Opcode::Nop,
            68 => Opcode::Halt,
            _ => return None,
        })
    }
}

/// Fat payloads are boxed so `Value` stays small on the operand stack.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct FunctionObj {
    pub name: Rc<str>,
    pub arity: usize,
    pub chunk_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct ClosureObj {
    pub function_index: usize,
    pub arity: usize,
    pub upvalues: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct ClassObj {
    pub name: Rc<str>,
    pub methods: Rc<Vec<(String, usize)>>,
    pub superclass: Option<Rc<str>>,
    pub properties: Rc<Vec<(String, Value)>>,
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
    Function(Rc<FunctionObj>),
    Closure(Rc<ClosureObj>),
    Class(Rc<ClassObj>),
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
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Closure(a), Value::Closure(b)) => a == b,
            (Value::Class(a), Value::Class(b)) => a == b,
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

/// Compact, `Copy` instruction — fetched by value on the hot path.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(all(feature = "serde", feature = "std"), derive(Serialize, Deserialize))]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Option<u32>,
    pub line: u32,
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

    pub fn emit(&mut self, opcode: Opcode, operand: Option<u32>, line: u32) {
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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Opcode {
    // Stack operations
    Push,
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

    // Error handling
    Throw,
    TryBegin,
    TryEnd,

    // Special
    Nop,
    Halt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Tuple(Vec<Value>),
    Function {
        name: String,
        arity: usize,
        chunk_index: usize,
    },
    Closure {
        function_index: usize,
        upvalues: Vec<usize>,
    },
    Class {
        name: String,
        methods: Vec<(String, usize)>,
    },
    Instance {
        class_index: usize,
        fields: Vec<(String, Value)>,
    },
    Ok(Box<Value>),
    Err(Box<Value>),
    Some(Box<Value>),
    Builtin { name: String, arity: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Option<u32>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bytecode {
    pub chunks: Vec<Chunk>,
    pub main_chunk: usize,
}

impl Bytecode {
    pub fn new() -> Self {
        let main_chunk = Chunk::new("main".to_string());
        Self {
            chunks: vec![main_chunk],
            main_chunk: 0,
        }
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

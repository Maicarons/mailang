use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use mailang_bytecode::{Bytecode, ClosureObj, Opcode, Value};
use crate::error::VmError;

type BuiltinFn = fn(&[Value]) -> Result<Value, String>;
/// Host-registered callback (FFI / embedders). `Rc` so it can be re-applied
/// when a new VM is created after each `eval`.
pub type HostFn = Rc<dyn Fn(&[Value]) -> Result<Value, String>>;

#[derive(Debug, Clone)]
struct CallFrame {
    chunk_index: usize,
    ip: usize,
    stack_base: usize,
    upvalues: Vec<usize>,
    /// True for CallDirect: no function slot below `stack_base`.
    direct: bool,
}

#[derive(Debug, Clone)]
struct RegisteredClass {
    name: String,
    superclass: Option<String>,
    methods: Vec<(String, usize, usize)>,
    properties: Vec<(String, Value)>,
}

pub struct Vm {
    bytecode: Bytecode,
    stack: Vec<Value>,
    /// Global variables indexed by interned slot (no name lookup on hot path).
    globals: Vec<Value>,
    builtins: HashMap<String, BuiltinFn>,
    host_fns: HashMap<String, HostFn>,
    call_stack: Vec<CallFrame>,
    ip: usize,
    chunk_index: usize,
    class_table: Vec<RegisteredClass>,
    upvalue_store: Vec<Value>,
}

impl Vm {
    pub fn new(bytecode: Bytecode) -> Self {
        let mut builtins: HashMap<String, BuiltinFn> = HashMap::new();
        builtins.insert("println".to_string(), |args| mailang_stdlib::builtin_println(args));
        builtins.insert("print".to_string(), |args| mailang_stdlib::builtin_print(args));
        builtins.insert("input".to_string(), |args| mailang_stdlib::builtin_input(args));
        builtins.insert("sqrt".to_string(), |args| mailang_stdlib::builtin_sqrt(args));
        builtins.insert("abs".to_string(), |args| mailang_stdlib::builtin_abs(args));
        builtins.insert("sin".to_string(), |args| mailang_stdlib::builtin_sin(args));
        builtins.insert("cos".to_string(), |args| mailang_stdlib::builtin_cos(args));
        builtins.insert("floor".to_string(), |args| mailang_stdlib::builtin_floor(args));
        builtins.insert("ceil".to_string(), |args| mailang_stdlib::builtin_ceil(args));
        builtins.insert("round".to_string(), |args| mailang_stdlib::builtin_round(args));
        builtins.insert("min".to_string(), |args| mailang_stdlib::builtin_min(args));
        builtins.insert("max".to_string(), |args| mailang_stdlib::builtin_max(args));
        builtins.insert("len".to_string(), |args| mailang_stdlib::builtin_len(args));
        builtins.insert("to_string".to_string(), |args| mailang_stdlib::builtin_to_string(args));
        builtins.insert("parse_int".to_string(), |args| mailang_stdlib::builtin_parse_int(args));
        builtins.insert("parse_float".to_string(), |args| mailang_stdlib::builtin_parse_float(args));
        builtins.insert("time_now".to_string(), |args| mailang_stdlib::builtin_time_now(args));
        builtins.insert("time_now_secs".to_string(), |args| mailang_stdlib::builtin_time_now_secs(args));
        builtins.insert("time_year".to_string(), |args| mailang_stdlib::builtin_time_year(args));
        builtins.insert("time_month".to_string(), |args| mailang_stdlib::builtin_time_month(args));
        builtins.insert("time_day".to_string(), |args| mailang_stdlib::builtin_time_day(args));
        builtins.insert("time_hour".to_string(), |args| mailang_stdlib::builtin_time_hour(args));
        builtins.insert("time_minute".to_string(), |args| mailang_stdlib::builtin_time_minute(args));
        builtins.insert("time_second".to_string(), |args| mailang_stdlib::builtin_time_second(args));
        builtins.insert("time_date".to_string(), |args| mailang_stdlib::builtin_time_date(args));
        builtins.insert("time_datetime".to_string(), |args| mailang_stdlib::builtin_time_datetime(args));
        builtins.insert("time_elapsed".to_string(), |args| mailang_stdlib::builtin_time_elapsed(args));
        builtins.insert("time_sleep".to_string(), |args| mailang_stdlib::builtin_time_sleep(args));
        // Simulated IoT HAL
        builtins.insert("gpio_write".to_string(), |args| mailang_stdlib::hal::builtin_gpio_write(args));
        builtins.insert("gpio_read".to_string(), |args| mailang_stdlib::hal::builtin_gpio_read(args));
        builtins.insert("delay_ms".to_string(), |args| mailang_stdlib::hal::builtin_delay_ms(args));
        builtins.insert("adc_read".to_string(), |args| mailang_stdlib::hal::builtin_adc_read(args));

        let mut globals = vec![Value::Null; bytecode.global_names.len()];
        for (slot, name) in bytecode.global_names.iter().enumerate() {
            if builtins.contains_key(name.as_str()) {
                globals[slot] = Value::Builtin {
                    name: name.as_str().into(),
                    arity: 0,
                };
            }
        }

        Self {
            bytecode,
            stack: Vec::with_capacity(256),
            globals,
            builtins,
            host_fns: HashMap::new(),
            call_stack: Vec::new(),
            ip: 0,
            chunk_index: 0,
            class_table: Vec::new(),
            upvalue_store: Vec::new(),
        }
    }

    /// Register a host callback callable from MaìLang as `name(...)`.
    pub fn register_host_fn(&mut self, name: impl Into<String>, f: HostFn) {
        let name = name.into();
        self.host_fns.insert(name.clone(), f);
        // Also ensure a global slot exists and holds a Builtin callable.
        let slot = self.bytecode.intern_global(&name) as usize;
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Null);
        }
        self.globals[slot] = Value::Builtin {
            name: name.as_str().into(),
            arity: 0,
        };
    }

    fn global_slot(&mut self, name: &str) -> usize {
        if let Some(pos) = self.bytecode.global_names.iter().position(|n| n == name) {
            return pos;
        }
        let slot = self.bytecode.intern_global(name) as usize;
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Null);
        }
        slot
    }

    pub fn get_global(&mut self, name: &str) -> Value {
        let slot = self.global_slot(name);
        self.globals.get(slot).cloned().unwrap_or(Value::Null)
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        let slot = self.global_slot(name);
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Null);
        }
        self.globals[slot] = value;
    }

    /// Break Rc cycles reachable from the operand stack and globals.
    /// Returns the number of back-edges cut.
    pub fn collect_cycles(&mut self) -> usize {
        let mut roots: Vec<Value> = self.stack.clone();
        roots.extend(self.globals.iter().cloned());
        let broken = mailang_gc::collect_cycles(&mut roots);
        if broken > 0 {
            // Write mutated values back.
            let n_stack = self.stack.len();
            for (i, v) in roots.iter().take(n_stack).enumerate() {
                self.stack[i] = v.clone();
            }
            for (i, v) in roots.iter().skip(n_stack).enumerate() {
                if i < self.globals.len() {
                    self.globals[i] = v.clone();
                }
            }
        }
        broken
    }

    pub fn run(&mut self) -> Result<Value, VmError> {
        loop {
            let instructions = &self.bytecode.chunks[self.chunk_index].instructions;
            if self.ip >= instructions.len() {
                return Err(VmError::Internal("IP out of bounds".to_string()));
            }
            let instruction = instructions[self.ip];
            self.ip += 1;
            let opcode = instruction.opcode;
            let operand = instruction.operand;

            match opcode {
                Opcode::Push => {
                    let index = operand.ok_or_else(|| VmError::Internal("Push missing operand".to_string()))? as usize;
                    let value = self.bytecode.chunks[self.chunk_index]
                        .constants.get(index).cloned()
                        .ok_or_else(|| VmError::Internal("Invalid constant index".to_string()))?;
                    self.push(value)?;
                }
                Opcode::Pop => { self.pop()?; }
                Opcode::Dup => {
                    let value = self.peek()?.clone();
                    self.push(value)?;
                }
                Opcode::LoadLocal => {
                    let index = operand.unwrap_or(0) as usize;
                    let base = self
                        .call_stack
                        .last()
                        .map(|f| f.stack_base)
                        .unwrap_or(0);
                    // Fast path for Copy scalars avoids a full Value clone.
                    match self.stack.get(base + index) {
                        Some(Value::Int(n)) => self.stack.push(Value::Int(*n)),
                        Some(Value::Bool(b)) => self.stack.push(Value::Bool(*b)),
                        Some(Value::Float(f)) => self.stack.push(Value::Float(*f)),
                        Some(Value::Char(c)) => self.stack.push(Value::Char(*c)),
                        Some(Value::Null) => self.stack.push(Value::Null),
                        Some(v) => {
                            let v = v.clone();
                            self.stack.push(v);
                        }
                        None => return Err(VmError::Internal("Invalid local index".to_string())),
                    }
                }
                Opcode::StoreLocal => {
                    let index = operand.unwrap_or(0) as usize;
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let base = self
                        .call_stack
                        .last()
                        .map(|f| f.stack_base)
                        .unwrap_or(0);
                    if base + index >= self.stack.len() {
                        self.stack.resize(base + index + 1, Value::Null);
                    }
                    self.stack[base + index] = value;
                }
                Opcode::LoadGlobal => {
                    let slot = operand.unwrap_or(0) as usize;
                    match self.globals.get(slot) {
                        Some(Value::Int(n)) => self.stack.push(Value::Int(*n)),
                        Some(Value::Bool(b)) => self.stack.push(Value::Bool(*b)),
                        Some(Value::Float(f)) => self.stack.push(Value::Float(*f)),
                        Some(v) => {
                            let v = v.clone();
                            self.stack.push(v);
                        }
                        None => self.stack.push(Value::Null),
                    }
                }
                Opcode::StoreGlobal => {
                    let slot = operand.unwrap_or(0) as usize;
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if slot >= self.globals.len() {
                        self.globals.resize(slot + 1, Value::Null);
                    }
                    self.globals[slot] = value;
                }
                Opcode::LoadUpvalue => {
                    let index = operand.unwrap_or(0) as usize;
                    let store_idx = self
                        .call_stack
                        .last()
                        .and_then(|f| f.upvalues.get(index).copied())
                        .ok_or_else(|| VmError::Internal("Invalid upvalue index".to_string()))?;
                    let value = self.upvalue_store.get(store_idx).cloned()
                        .ok_or_else(|| VmError::Internal("Invalid upvalue store index".to_string()))?;
                    self.stack.push(value);
                }
                Opcode::StoreUpvalue => {
                    let index = operand.unwrap_or(0) as usize;
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let store_idx = self
                        .call_stack
                        .last()
                        .and_then(|f| f.upvalues.get(index).copied())
                        .ok_or_else(|| VmError::Internal("Invalid upvalue index".to_string()))?;
                    if store_idx >= self.upvalue_store.len() {
                        return Err(VmError::Internal("Invalid upvalue store index".to_string()));
                    }
                    self.upvalue_store[store_idx] = value;
                }
                Opcode::Add => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    // Hot path: integer add
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Int(a.wrapping_add(*b)));
                    } else {
                        self.stack.push(self.add_values(left, right)?);
                    }
                }
                Opcode::Sub => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Int(a.wrapping_sub(*b)));
                    } else {
                        self.stack.push(self.sub_values(left, right)?);
                    }
                }
                Opcode::Mul => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Int(a.wrapping_mul(*b)));
                    } else {
                        self.push(self.mul_values(left, right)?)?;
                    }
                }
                Opcode::Div => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.div_values(left, right)?)?;
                }
                Opcode::Mod => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.mod_values(left, right)?)?;
                }
                Opcode::Pow => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.pow_values(left, right)?)?;
                }
                Opcode::Neg => {
                    let value = self.pop()?;
                    match value {
                        Value::Int(n) => self.push(Value::Int(-n))?,
                        Value::Float(n) => self.push(Value::Float(-n))?,
                        _ => return Err(VmError::TypeError("Cannot negate non-number".to_string())),
                    }
                }
                Opcode::BitAnd => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.bitand_values(left, right)?)?;
                }
                Opcode::BitOr => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.bitor_values(left, right)?)?;
                }
                Opcode::BitXor => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.push(self.bitxor_values(left, right)?)?;
                }
                Opcode::BitNot => {
                    let value = self.pop()?;
                    match value {
                        Value::Int(n) => self.push(Value::Int(!n))?,
                        _ => return Err(VmError::TypeError("Cannot bitwise-not non-integer".to_string())),
                    }
                }
                Opcode::Shl => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => self.push(Value::Int(a << b))?,
                        _ => return Err(VmError::TypeError("Shift requires integers".to_string())),
                    }
                }
                Opcode::Shr => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => self.push(Value::Int(a >> b))?,
                        _ => return Err(VmError::TypeError("Shift requires integers".to_string())),
                    }
                }
                Opcode::Eq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a == b));
                    } else if let (Value::Bool(a), Value::Bool(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a == b));
                    } else {
                        self.stack.push(Value::Bool(self.values_equal(&left, &right)));
                    }
                }
                Opcode::Ne => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a != b));
                    } else {
                        self.stack.push(Value::Bool(!self.values_equal(&left, &right)));
                    }
                }
                Opcode::Lt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a < b));
                    } else {
                        self.stack.push(Value::Bool(self.compare_values(&left, &right)? < 0));
                    }
                }
                Opcode::Le => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a <= b));
                    } else {
                        self.stack.push(Value::Bool(self.compare_values(&left, &right)? <= 0));
                    }
                }
                Opcode::Gt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a > b));
                    } else {
                        self.stack.push(Value::Bool(self.compare_values(&left, &right)? > 0));
                    }
                }
                Opcode::Ge => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                        self.stack.push(Value::Bool(a >= b));
                    } else {
                        self.stack.push(Value::Bool(self.compare_values(&left, &right)? >= 0));
                    }
                }
                Opcode::AddImm | Opcode::SubImm | Opcode::MulImm => {
                    let imm = operand.unwrap_or(0) as i32 as i64;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match left {
                        Value::Int(a) => {
                            let r = match opcode {
                                Opcode::AddImm => a.wrapping_add(imm),
                                Opcode::SubImm => a.wrapping_sub(imm),
                                _ => a.wrapping_mul(imm),
                            };
                            self.stack.push(Value::Int(r));
                        }
                        other => {
                            let lit = Value::Int(imm);
                            let v = match opcode {
                                Opcode::AddImm => self.add_values(other, lit)?,
                                Opcode::SubImm => self.sub_values(other, lit)?,
                                _ => self.mul_values(other, lit)?,
                            };
                            self.stack.push(v);
                        }
                    }
                }
                Opcode::EqImm | Opcode::NeImm | Opcode::LtImm | Opcode::LeImm
                | Opcode::GtImm | Opcode::GeImm => {
                    let imm = operand.unwrap_or(0) as i32 as i64;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match left {
                        Value::Int(a) => {
                            let b = match opcode {
                                Opcode::EqImm => a == imm,
                                Opcode::NeImm => a != imm,
                                Opcode::LtImm => a < imm,
                                Opcode::LeImm => a <= imm,
                                Opcode::GtImm => a > imm,
                                _ => a >= imm,
                            };
                            self.stack.push(Value::Bool(b));
                        }
                        other => {
                            let lit = Value::Int(imm);
                            let c = self.compare_values(&other, &lit)?;
                            let b = match opcode {
                                Opcode::EqImm => c == 0,
                                Opcode::NeImm => c != 0,
                                Opcode::LtImm => c < 0,
                                Opcode::LeImm => c <= 0,
                                Opcode::GtImm => c > 0,
                                _ => c >= 0,
                            };
                            self.stack.push(Value::Bool(b));
                        }
                    }
                }
                Opcode::And => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    let result = self.is_truthy(&left) && self.is_truthy(&right);
                    self.push(Value::Bool(result))?;
                }
                Opcode::Or => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    let result = self.is_truthy(&left) || self.is_truthy(&right);
                    self.push(Value::Bool(result))?;
                }
                Opcode::Not => {
                    let value = self.pop()?;
                    self.push(Value::Bool(!self.is_truthy(&value)))?;
                }
                Opcode::Jump => {
                    let target = operand.ok_or_else(|| VmError::Internal("Jump missing target".to_string()))? as usize;
                    self.ip = target;
                }
                Opcode::JumpIfFalse => {
                    let target = operand.unwrap_or(0) as usize;
                    let jump = match self.stack.last() {
                        Some(Value::Bool(false)) => true,
                        Some(Value::Null) => true,
                        Some(Value::Int(0)) => true,
                        Some(v) => !self.is_truthy(v),
                        None => return Err(VmError::StackUnderflow),
                    };
                    if jump {
                        self.ip = target;
                    }
                }
                Opcode::JumpIfTrue => {
                    let target = operand.ok_or_else(|| VmError::Internal("JumpIfTrue missing target".to_string()))? as usize;
                    let condition = self.peek()?;
                    if self.is_truthy(condition) {
                        self.ip = target;
                    }
                }
                Opcode::Call => {
                    let arg_count = operand.unwrap_or(0) as usize;
                    let func_index = self.stack.len().checked_sub(arg_count + 1)
                        .ok_or_else(|| VmError::Internal(format!(
                            "Stack underflow in Call: stack_len={}, arg_count={}",
                            self.stack.len(), arg_count
                        )))?;

                    // Fast path: plain function without cloning the Value.
                    if let Value::Function(f) = &self.stack[func_index] {
                        let arity = f.arity;
                        let chunk_index = f.chunk_index;
                        if arity != arg_count {
                            return Err(VmError::WrongArgumentCount { expected: arity, found: arg_count });
                        }
                        self.call_stack.push(CallFrame {
                            chunk_index: self.chunk_index,
                            ip: self.ip,
                            stack_base: func_index + 1,
                            upvalues: Vec::new(),
                            direct: false,
                        });
                        self.chunk_index = chunk_index;
                        self.ip = 0;
                        continue;
                    }

                    let func = self.stack[func_index].clone();

                    match func {
                        Value::Function(f) => {
                            if f.arity != arg_count {
                                return Err(VmError::WrongArgumentCount { expected: f.arity, found: arg_count });
                            }
                            let frame = CallFrame {
                                chunk_index: self.chunk_index,
                                ip: self.ip,
                                stack_base: func_index + 1,
                                upvalues: Vec::new(),
                                direct: false,
                            };
                            self.call_stack.push(frame);
                            self.chunk_index = f.chunk_index;
                            self.ip = 0;
                        }
                        Value::Closure(c) => {
                            if c.arity != arg_count {
                                return Err(VmError::WrongArgumentCount { expected: c.arity, found: arg_count });
                            }
                            let frame = CallFrame {
                                chunk_index: self.chunk_index,
                                ip: self.ip,
                                stack_base: func_index + 1,
                                upvalues: c.upvalues.clone(),
                                direct: false,
                            };
                            self.call_stack.push(frame);
                            self.chunk_index = c.function_index;
                            self.ip = 0;
                        }
                        Value::Class(cls) => {
                            let class_idx = self.class_table.len();
                            self.class_table.push(RegisteredClass {
                                name: cls.name.to_string(),
                                superclass: cls.superclass.as_ref().map(|s| s.to_string()),
                                methods: cls.methods.iter().map(|(n, ci)| (n.clone(), *ci, 0usize)).collect(),
                                properties: cls.properties.as_ref().clone(),
                            });

                            let mut fields: Vec<(String, Value)> = Vec::new();
                            for (pname, pdefault) in cls.properties.iter() {
                                fields.push((pname.clone(), pdefault.clone()));
                            }
                            let instance = Value::Instance {
                                class_index: class_idx,
                                fields: Rc::new(RefCell::new(fields)),
                            };
                            self.stack[func_index] = instance.clone();

                            let init_chunk = cls.methods.iter()
                                .find(|(n, _)| n == "init")
                                .map(|(_, ci)| *ci);

                            if let Some(chunk_index) = init_chunk {
                                self.stack.insert(func_index, instance.clone());
                                let frame = CallFrame {
                                    chunk_index: self.chunk_index,
                                    ip: self.ip,
                                    stack_base: func_index + 1,
                                    upvalues: Vec::new(),
                                    direct: false,
                                };
                                self.call_stack.push(frame);
                                self.chunk_index = chunk_index;
                                self.ip = 0;
                            } else {
                                self.stack.truncate(func_index + 1);
                            }
                        }
                        Value::Builtin { name, .. } => {
                            let args: Vec<Value> = self.stack[func_index + 1..].to_vec();
                            self.stack.truncate(func_index);
                            let call_result = if let Some(host) = self.host_fns.get(name.as_ref()) {
                                host(&args)
                            } else if let Some(builtin_fn) = self.builtins.get(name.as_ref()) {
                                builtin_fn(&args)
                            } else {
                                return Err(VmError::UndefinedFunction(name.to_string()));
                            };
                            match call_result {
                                Ok(result) => self.push(result)?,
                                Err(e) => return Err(VmError::RuntimeError(e)),
                            }
                        }
                        _ => {
                            return Err(VmError::TypeError("Cannot call non-function".to_string()));
                        }
                    }
                }
                Opcode::WrapOk => {
                    let value = self.pop()?;
                    self.push(Value::Ok(Box::new(value)))?;
                }
                Opcode::WrapErr => {
                    let value = self.pop()?;
                    self.push(Value::Err(Box::new(value)))?;
                }
                Opcode::WrapSome => {
                    let value = self.pop()?;
                    self.push(Value::Some(Box::new(value)))?;
                }
                Opcode::UnwrapOk => {
                    let value = self.pop()?;
                    match value {
                        Value::Ok(inner) => {
                            self.push(*inner)?;
                            self.push(Value::Bool(true))?;
                        }
                        _ => {
                            self.push(Value::Bool(false))?;
                        }
                    }
                }
                Opcode::UnwrapErr => {
                    let value = self.pop()?;
                    match value {
                        Value::Err(inner) => {
                            self.push(*inner)?;
                            self.push(Value::Bool(true))?;
                        }
                        _ => {
                            self.push(Value::Bool(false))?;
                        }
                    }
                }
                Opcode::UnwrapSome => {
                    let value = self.pop()?;
                    match value {
                        Value::Some(inner) => {
                            self.push(*inner)?;
                            self.push(Value::Bool(true))?;
                        }
                        _ => {
                            self.push(Value::Bool(false))?;
                        }
                    }
                }
                Opcode::TailCall => {
                    let arg_count = operand.unwrap_or(0) as usize;
                    let func_index = self.stack.len().checked_sub(arg_count + 1)
                        .ok_or_else(|| VmError::Internal(format!(
                            "Stack underflow in TailCall: stack_len={}, arg_count={}",
                            self.stack.len(), arg_count
                        )))?;
                    let func = self.stack[func_index].clone();

                    match func {
                        Value::Function(f) => {
                            if f.arity != arg_count {
                                return Err(VmError::WrongArgumentCount { expected: f.arity, found: arg_count });
                            }
                            // Move the new callee+args over the current frame's slot
                            // so the abandoned locals are discarded and Return goes
                            // straight to the original caller.
                            if let Some(frame) = self.call_stack.last() {
                                let old_base = frame.stack_base.saturating_sub(1);
                                let new_len = arg_count + 1;
                                if old_base != func_index {
                                    for i in 0..new_len {
                                        self.stack[old_base + i] = self.stack[func_index + i].clone();
                                    }
                                    self.stack.truncate(old_base + new_len);
                                }
                                if let Some(frame) = self.call_stack.last_mut() {
                                    frame.stack_base = old_base + 1;
                                    frame.upvalues.clear();
                                }
                            }
                            self.chunk_index = f.chunk_index;
                            self.ip = 0;
                        }
                        Value::Closure(c) => {
                            if c.arity != arg_count {
                                return Err(VmError::WrongArgumentCount { expected: c.arity, found: arg_count });
                            }
                            if let Some(frame) = self.call_stack.last() {
                                let old_base = frame.stack_base.saturating_sub(1);
                                let new_len = arg_count + 1;
                                if old_base != func_index {
                                    for i in 0..new_len {
                                        self.stack[old_base + i] = self.stack[func_index + i].clone();
                                    }
                                    self.stack.truncate(old_base + new_len);
                                }
                                if let Some(frame) = self.call_stack.last_mut() {
                                    frame.stack_base = old_base + 1;
                                    frame.upvalues = c.upvalues.clone();
                                }
                            }
                            self.chunk_index = c.function_index;
                            self.ip = 0;
                        }
                        Value::Builtin { name, .. } => {
                            let args: Vec<Value> = self.stack[func_index + 1..].to_vec();
                            self.stack.truncate(func_index);
                            let result = if let Some(builtin_fn) = self.builtins.get(name.as_ref()) {
                                match builtin_fn(&args) {
                                    Ok(result) => result,
                                    Err(e) => return Err(VmError::RuntimeError(e)),
                                }
                            } else {
                                return Err(VmError::UndefinedFunction(name.to_string()));
                            };
                            if let Some(frame) = self.call_stack.pop() {
                                let base = frame.stack_base.saturating_sub(1);
                                self.stack.truncate(base);
                                self.chunk_index = frame.chunk_index;
                                self.ip = frame.ip;
                                self.push(result)?;
                            } else {
                                return Ok(result);
                            }
                        }
                        // Constructors / unknown callables: perform a normal Call.
                        Value::Class(cls) => {
                            let class_idx = self.class_table.len();
                            self.class_table.push(RegisteredClass {
                                name: cls.name.to_string(),
                                superclass: cls.superclass.as_ref().map(|s| s.to_string()),
                                methods: cls.methods.iter().map(|(n, ci)| (n.clone(), *ci, 0usize)).collect(),
                                properties: cls.properties.as_ref().clone(),
                            });

                            let mut fields: Vec<(String, Value)> = Vec::new();
                            for (pname, pdefault) in cls.properties.iter() {
                                fields.push((pname.clone(), pdefault.clone()));
                            }
                            let instance = Value::Instance {
                                class_index: class_idx,
                                fields: Rc::new(RefCell::new(fields)),
                            };
                            self.stack[func_index] = instance.clone();

                            let init_chunk = cls.methods.iter()
                                .find(|(n, _)| n == "init")
                                .map(|(_, ci)| *ci);

                            if let Some(chunk_index) = init_chunk {
                                self.stack.insert(func_index, instance.clone());
                                let frame = CallFrame {
                                    chunk_index: self.chunk_index,
                                    ip: self.ip,
                                    stack_base: func_index + 1,
                                    upvalues: Vec::new(),
                                    direct: false,
                                };
                                self.call_stack.push(frame);
                                self.chunk_index = chunk_index;
                                self.ip = 0;
                            } else {
                                self.stack.truncate(func_index + 1);
                            }
                        }
                        _ => {
                            return Err(VmError::TypeError("TailCall not supported for this callable".to_string()));
                        }
                    }
                }
                Opcode::CallDirect => {
                    let packed = operand.unwrap_or(0);
                    let chunk_index = (packed >> 16) as usize;
                    let arg_count = (packed & 0xFFFF) as usize;
                    // Stack: [args...] �?stack_base is the first argument.
                    let stack_base = self.stack.len().checked_sub(arg_count)
                        .ok_or_else(|| VmError::StackUnderflow)?;
                    self.call_stack.push(CallFrame {
                        chunk_index: self.chunk_index,
                        ip: self.ip,
                        stack_base,
                        upvalues: Vec::new(),
                        direct: true,
                    });
                    self.chunk_index = chunk_index;
                    self.ip = 0;
                }
                Opcode::Return => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let Some(frame) = self.call_stack.pop() {
                        let base = if frame.direct {
                            frame.stack_base
                        } else {
                            frame.stack_base.saturating_sub(1)
                        };
                        self.stack.truncate(base);
                        self.chunk_index = frame.chunk_index;
                        self.ip = frame.ip;
                        self.stack.push(value);
                    } else {
                        return Ok(value);
                    }
                }
                Opcode::GetProperty => {
                    let prop_index = operand.ok_or_else(|| VmError::Internal("GetProperty missing operand".to_string()))? as usize;
                    let prop_name = match &self.bytecode.chunks[self.chunk_index].constants[prop_index] {
                        Value::Str(s) => s.to_string(),
                        _ => return Err(VmError::Internal("Expected string constant".to_string())),
                    };
                    let object = self.pop()?;
                    match &object {
                        Value::Map(entries) => {
                            let mut found = false;
                            for (k, v) in entries.borrow().iter() {
                                if let Value::Str(s) = k {
                                    if s.as_ref() == prop_name.as_str() {
                                        self.push(v.clone())?;
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if !found {
                                self.push(Value::Null)?;
                            }
                        }
                        Value::Instance { fields, .. } => {
                            let mut found = false;
                            for (name, value) in fields.borrow().iter() {
                                if name == &prop_name {
                                    self.push(value.clone())?;
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                self.push(Value::Null)?;
                            }
                        }
                        Value::Array(arr) => {
                            match prop_name.as_str() {
                                "len" => self.push(Value::Int(arr.borrow().len() as i64))?,
                                _ => return Err(VmError::UndefinedProperty(prop_name)),
                            }
                        }
                        Value::Str(s) => {
                            match prop_name.as_str() {
                                "len" => self.push(Value::Int(s.chars().count() as i64))?,
                                _ => return Err(VmError::UndefinedProperty(prop_name)),
                            }
                        }
                        _ => return Err(VmError::TypeError("Cannot access property of non-object".to_string())),
                    }
                }
                Opcode::SetProperty => {
                    let prop_index = operand.ok_or_else(|| VmError::Internal("SetProperty missing operand".to_string()))? as usize;
                    let prop_name = match &self.bytecode.chunks[self.chunk_index].constants[prop_index] {
                        Value::Str(s) => s.to_string(),
                        _ => return Err(VmError::Internal("Expected string constant".to_string())),
                    };
                    let value = self.pop()?;
                    let object = self.pop()?;
                    match &object {
                        Value::Instance { class_index, fields } => {
                            let mut found = false;
                            for (name, v) in fields.borrow_mut().iter_mut() {
                                if name == &prop_name {
                                    *v = value.clone();
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                fields.borrow_mut().push((prop_name, value.clone()));
                            }
                            self.push(Value::Instance { class_index: *class_index, fields: fields.clone() })?;
                        }
                        Value::Map(entries) => {
                            {
                                let mut map = entries.borrow_mut();
                                map.retain(|(k, _)| {
                                    if let Value::Str(s) = k {
                                        s.as_ref() != prop_name.as_str()
                                    } else {
                                        true
                                    }
                                });
                                map.push((Value::Str(prop_name.as_str().into()), value));
                            }
                            self.push(object)?;
                        }
                        _ => return Err(VmError::TypeError("Cannot set property of non-object".to_string())),
                    }
                }
                Opcode::Invoke => {
                    let packed = operand.ok_or_else(|| VmError::Internal("Invoke missing operand".to_string()))?;
                    let arg_count = (packed >> 16) as usize;
                    let method_const_idx = (packed & 0xFFFF) as usize;
                    let method_name = match &self.bytecode.chunks[self.chunk_index].constants[method_const_idx] {
                        Value::Str(s) => s.to_string(),
                        _ => return Err(VmError::Internal("Expected string constant for method name".to_string())),
                    };

                    let obj_index = self.stack.len().checked_sub(arg_count + 1)
                        .ok_or_else(|| VmError::StackUnderflow)?;
                    let object = self.stack[obj_index].clone();

                    match &object {
                        Value::Instance { class_index, .. } => {
                            let class = self.class_table.get(*class_index)
                                .ok_or_else(|| VmError::Internal("Invalid class index".to_string()))?;
                            let method = class.methods.iter()
                                .find(|(n, _, _)| n == &method_name)
                                .cloned();
                            match method {
                                Some((_, chunk_index, _)) => {
                                    self.stack.insert(obj_index, object.clone());
                                    let frame = CallFrame {
                                        chunk_index: self.chunk_index,
                                        ip: self.ip,
                                        stack_base: obj_index + 1,
                                        upvalues: Vec::new(),
                                        direct: false,
                                    };
                                    self.call_stack.push(frame);
                                    self.chunk_index = chunk_index;
                                    self.ip = 0;
                                }
                                None => {
                                    return Err(VmError::UndefinedFunction(method_name));
                                }
                            }
                        }
                        Value::Map(entries) => {
                            let mut found = false;
                            for (k, v) in entries.borrow().iter() {
                                if let Value::Str(s) = k {
                                    if s.as_ref() == method_name.as_str() {
                                        self.stack[obj_index] = v.clone();
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if !found {
                                return Err(VmError::UndefinedFunction(method_name));
                            }
                            let func = self.stack[obj_index].clone();
                            match func {
                                Value::Function(f) => {
                                    if f.arity != arg_count {
                                        return Err(VmError::WrongArgumentCount { expected: f.arity, found: arg_count });
                                    }
                                    let frame = CallFrame {
                                        chunk_index: self.chunk_index,
                                        ip: self.ip,
                                        stack_base: obj_index + 1,
                                        upvalues: Vec::new(),
                                        direct: false,
                                    };
                                    self.call_stack.push(frame);
                                    self.chunk_index = f.chunk_index;
                                    self.ip = 0;
                                }
                                Value::Builtin { name: bname, .. } => {
                                    let args: Vec<Value> = self.stack[obj_index + 1..].to_vec();
                                    self.stack.truncate(obj_index);
                                    if let Some(builtin_fn) = self.builtins.get(bname.as_ref()) {
                                        match builtin_fn(&args) {
                                            Ok(result) => self.push(result)?,
                                            Err(e) => return Err(VmError::RuntimeError(e)),
                                        }
                                    } else {
                                        return Err(VmError::UndefinedFunction(bname.to_string()));
                                    }
                                }
                                _ => {
                                    return Err(VmError::TypeError("Map value is not callable".to_string()));
                                }
                            }
                        }
                        _ => {
                            return Err(VmError::TypeError(format!("Cannot invoke method '{}' on non-object", method_name)));
                        }
                    }
                }
                Opcode::BuildArray => {
                    let count = operand.unwrap_or(0) as usize;
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop()?);
                    }
                    elements.reverse();
                    self.push(Value::Array(Rc::new(RefCell::new(elements))))?;
                }
                Opcode::BuildMap => {
                    let count = operand.unwrap_or(0) as usize;
                    let mut entries = Vec::with_capacity(count);
                    for _ in 0..count {
                        let value = self.pop()?;
                        let key = self.pop()?;
                        entries.push((key, value));
                    }
                    entries.reverse();
                    self.push(Value::Map(Rc::new(RefCell::new(entries))))?;
                }
                Opcode::IndexGet => {
                    let index = self.pop()?;
                    let object = self.pop()?;
                    match (&object, &index) {
                        (Value::Array(arr), Value::Int(i)) => {
                            let arr_ref = arr.borrow();
                            if *i < 0 || *i >= arr_ref.len() as i64 {
                                return Err(VmError::IndexOutOfBounds { index: *i, length: arr_ref.len() });
                            }
                            self.push(arr_ref[*i as usize].clone())?;
                        }
                        (Value::Map(entries), key) => {
                            let mut found = false;
                            for (k, v) in entries.borrow().iter() {
                                if self.values_equal(k, key) {
                                    self.push(v.clone())?;
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                self.push(Value::Null)?;
                            }
                        }
                        (Value::Str(s), Value::Int(i)) => {
                            let char_count = s.chars().count();
                            if *i < 0 || *i >= char_count as i64 {
                                return Err(VmError::IndexOutOfBounds { index: *i, length: char_count });
                            }
                            self.push(Value::Char(s.chars().nth(*i as usize).unwrap()))?;
                        }
                        _ => return Err(VmError::TypeError("Cannot index this type".to_string())),
                    }
                }
                Opcode::IndexSet => {
                    let value = self.pop()?;
                    let index = self.pop()?;
                    let mut object = self.pop()?;
                    match (&mut object, &index) {
                        (Value::Array(arr), Value::Int(i)) => {
                            {
                                let mut arr_mut = arr.borrow_mut();
                                if *i < 0 || *i >= arr_mut.len() as i64 {
                                    return Err(VmError::IndexOutOfBounds { index: *i, length: arr_mut.len() });
                                }
                                arr_mut[*i as usize] = value;
                            }
                        }
                        (Value::Map(entries), key) => {
                            {
                                let mut map = entries.borrow_mut();
                                map.retain(|(k, _)| !self.values_equal(k, key));
                                map.push((index, value));
                            }
                        }
                        _ => return Err(VmError::TypeError("Cannot index-assign this type".to_string())),
                    }
                    self.push(object)?;
                }
                Opcode::CreateClass => {
                    let class_const_idx = operand.ok_or_else(|| VmError::Internal("CreateClass missing operand".to_string()))? as usize;
                    let class = self.bytecode.chunks[self.chunk_index].constants[class_const_idx].clone();
                    if let Value::Class(cls) = &class {
                        let class_idx = self.class_table.len();
                        let methods_vec: Vec<(String, usize, usize)> = cls.methods.iter()
                            .map(|(n, ci)| (n.clone(), *ci, 0))
                            .collect();
                        self.class_table.push(RegisteredClass {
                            name: cls.name.to_string(),
                            superclass: cls.superclass.as_ref().map(|s| s.to_string()),
                            methods: methods_vec,
                            properties: cls.properties.as_ref().clone(),
                        });
                        self.push(class)?;
                    } else {
                        self.push(class)?;
                    }
                }
                Opcode::CreateInstance => { self.push(Value::Null)?; }
                Opcode::GetMethod => { self.push(Value::Null)?; }
                Opcode::MatchPattern => { self.push(Value::Bool(true))?; }
                Opcode::MakeClosure => {
                    let count = operand.unwrap_or(0) as usize;
                    let mut uv_indices = Vec::with_capacity(count);
                    for _ in 0..count {
                        let value = self.pop()?;
                        let idx = self.upvalue_store.len();
                        self.upvalue_store.push(value);
                        uv_indices.push(idx);
                    }
                    uv_indices.reverse();
                    let func = self.pop()?;
                    match func {
                        Value::Function(f) => {
                            self.push(Value::Closure(Rc::new(ClosureObj {
                                function_index: f.chunk_index,
                                arity: f.arity,
                                upvalues: uv_indices,
                            })))?;
                        }
                        _ => {
                            return Err(VmError::TypeError("MakeClosure expects a function".to_string()));
                        }
                    }
                }
                Opcode::Throw => {
                    let value = self.pop()?;
                    return Err(VmError::RuntimeError(format!("Thrown: {:?}", value)));
                }
                Opcode::TryBegin => {}
                Opcode::TryEnd => {}
                Opcode::Nop => {}
                Opcode::Halt => {
                    if self.stack.is_empty() {
                        return Ok(Value::Null);
                    }
                    return Ok(self.pop()?);
                }
            }
        }
    }

    fn push(&mut self, value: Value) -> Result<(), VmError> {
        if self.stack.len() >= 100_000 {
            return Err(VmError::StackOverflow);
        }
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VmError> {
        self.stack.last().ok_or(VmError::StackUnderflow)
    }

    fn current_frame(&self) -> CallFrame {
        self.call_stack.last().cloned().unwrap_or(CallFrame {
            chunk_index: 0,
            ip: 0,
            stack_base: 0,
            upvalues: Vec::new(),
            direct: false,
        })
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Char(a), Value::Str(b)) => a.to_string().as_str() == b.as_ref(),
            (Value::Str(a), Value::Char(b)) => a.as_ref() == b.to_string().as_str(),
            _ => false,
        }
    }

    fn compare_values(&self, a: &Value, b: &Value) -> Result<i32, VmError> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b) as i32),
            (Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::Float(a), Value::Int(b)) => Ok(a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal) as i32),
            (Value::Str(a), Value::Str(b)) => Ok(a.as_ref().cmp(b.as_ref()) as i32),
            (Value::Char(a), Value::Char(b)) => Ok(a.cmp(b) as i32),
            (Value::Char(a), Value::Str(b)) => {
                let a_str = a.to_string();
                Ok(a_str.as_str().cmp(b.as_ref()) as i32)
            }
            (Value::Str(a), Value::Char(b)) => {
                let b_str = b.to_string();
                Ok(a.as_ref().cmp(b_str.as_str()) as i32)
            }
            _ => Err(VmError::TypeError("Cannot compare these types".to_string())),
        }
    }

    fn add_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b).into())),
            (Value::Str(a), b) => Ok(Value::Str(format!("{}{}", a, mailang_stdlib::value_to_string(&b)).into())),
            (a, Value::Str(b)) => Ok(Value::Str(format!("{}{}", mailang_stdlib::value_to_string(&a), b).into())),
            (Value::Array(a), Value::Array(b)) => {
                let mut new_vec = a.borrow().clone();
                new_vec.extend(b.borrow().iter().cloned());
                Ok(Value::Array(Rc::new(RefCell::new(new_vec))))
            }
            _ => Err(VmError::TypeError("Cannot add these types".to_string())),
        }
    }

    fn sub_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
            _ => Err(VmError::TypeError("Cannot subtract these types".to_string())),
        }
    }

    fn mul_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
            (Value::Str(s), Value::Int(n)) => Ok(Value::Str(s.repeat(n.max(0) as usize).into())),
            _ => Err(VmError::TypeError("Cannot multiply these types".to_string())),
        }
    }

    fn div_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(_), Value::Int(0)) => Err(VmError::DivisionByZero),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Float(a), Value::Float(b)) => {
                if b == 0.0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a / b)) }
            }
            (Value::Int(a), Value::Float(b)) => {
                if b == 0.0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a as f64 / b)) }
            }
            (Value::Float(a), Value::Int(b)) => {
                if b == 0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a / b as f64)) }
            }
            _ => Err(VmError::TypeError("Cannot divide these types".to_string())),
        }
    }

    fn mod_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(_), Value::Int(0)) => Err(VmError::DivisionByZero),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
            (Value::Float(a), Value::Float(b)) => {
                if b == 0.0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a % b)) }
            }
            (Value::Int(a), Value::Float(b)) => {
                if b == 0.0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a as f64 % b)) }
            }
            (Value::Float(a), Value::Int(b)) => {
                if b == 0 { Err(VmError::DivisionByZero) } else { Ok(Value::Float(a % b as f64)) }
            }
            _ => Err(VmError::TypeError("Cannot modulo these types".to_string())),
        }
    }

    fn pow_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.pow(b.max(0) as u32))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float((a as f64).powf(b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(b as f64))),
            _ => Err(VmError::TypeError("Cannot power these types".to_string())),
        }
    }

    fn bitand_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
            _ => Err(VmError::TypeError("Bitwise AND requires integers".to_string())),
        }
    }

    fn bitor_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
            _ => Err(VmError::TypeError("Bitwise OR requires integers".to_string())),
        }
    }

    fn bitxor_values(&self, left: Value, right: Value) -> Result<Value, VmError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
            _ => Err(VmError::TypeError("Bitwise XOR requires integers".to_string())),
        }
    }
}

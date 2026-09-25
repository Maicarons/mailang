//! Register-based virtual machine.
//!
//! Executes [`RegChunk`] three-address code with a flat register file per
//! frame. Stack bytecode is lowered to this IR by
//! [`mailang_bytecode::StackToRegister`].

use crate::error::VmError;
use crate::vm::HostFn;
use mailang_bytecode::register::{BinKind, CmpKind, RegChunk, RegOp, UnKind, WrapKind};
use mailang_bytecode::{Bytecode, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct RegFrame {
    chunk: usize,
    ip: usize,
    /// Base of this frame鈥檚 registers in the flat `regs` file.
    base: usize,
    #[allow(dead_code)]
    n_regs: usize,
    upvalues: Vec<usize>,
    argc: usize,
    /// Where to place the return value (in the caller鈥檚 register file).
    ret_dst: u16,
}

/// Register-mode interpreter. Shares builtins / host-fns / globals with the
/// stack [`Vm`] so embedders see one API.
pub struct RegisterVm {
    bytecode: Bytecode,
    chunks: Vec<RegChunk>,
    regs: Vec<Value>,
    frames: Vec<RegFrame>,
    globals: Vec<Value>,
    builtins: HashMap<String, fn(&[Value]) -> Result<Value, String>>,
    host_fns: HashMap<String, HostFn>,
    upvalue_store: Vec<Value>,
    fuel: Option<u64>,
    max_call_depth: usize,
    class_table: Vec<RegClass>,
    gc: mailang_gc::MarkSweepHeap,
}

#[derive(Clone)]
struct RegClass {
    name: String,
    superclass: Option<String>,
    methods: Vec<(String, usize)>,
    properties: Vec<(String, Value)>,
}

/// Parameter / `this` slot count for each chunk (used as the translator's
/// starting stack height so `let` locals land on their local registers).
fn chunk_start_sp(bytecode: &Bytecode, chunk_idx: usize) -> u16 {
    if chunk_idx == bytecode.main_chunk {
        return 0;
    }
    let mut best: Option<usize> = None;
    for ch in &bytecode.chunks {
        for c in &ch.constants {
            if let Value::Function(f) = c {
                if f.chunk_index == chunk_idx {
                    best = Some(best.map_or(f.arity, |b: usize| b.max(f.arity)));
                }
            }
            if let Value::Class(cls) = c {
                for (_, ci) in cls.methods.iter() {
                    if *ci == chunk_idx {
                        // Method chunk without a self-describing FunctionObj:
                        // at least `this`.
                        if best.is_none() {
                            best = Some(1);
                        }
                    }
                }
            }
        }
    }
    best.unwrap_or(0) as u16
}

impl RegisterVm {
    /// Lower every chunk of `bytecode` and prepare to run chunk 0.
    pub fn new(bytecode: Bytecode) -> Self {
        let mut chunks = Vec::with_capacity(bytecode.chunks.len());
        for (i, ch) in bytecode.chunks.iter().enumerate() {
            let start = chunk_start_sp(&bytecode, i);
            let reg =
                mailang_bytecode::StackToRegister::new(&ch.name, &ch.instructions, &ch.constants)
                    .with_start_sp(start)
                    .translate();
            chunks.push(reg);
        }
        let mut globals = vec![Value::Null; bytecode.global_names.len()];
        // Seed builtins into global slots (same names as stack Vm).
        let mut builtins: HashMap<String, fn(&[Value]) -> Result<Value, String>> = HashMap::new();
        register_builtins(&mut builtins);
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
            chunks,
            regs: vec![Value::Null; 256],
            frames: Vec::new(),
            globals,
            builtins,
            host_fns: HashMap::new(),
            upvalue_store: Vec::new(),
            fuel: None,
            max_call_depth: 512,
            class_table: Vec::new(),
            gc: mailang_gc::MarkSweepHeap::new(),
        }
    }

    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
    }

    pub fn set_max_call_depth(&mut self, limit: usize) {
        self.max_call_depth = limit.max(1);
    }

    pub fn register_host_fn(&mut self, name: impl Into<String>, f: HostFn) {
        let name = name.into();
        self.host_fns.insert(name.clone(), f);
        let slot = self.bytecode.intern_global(&name) as usize;
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Null);
        }
        self.globals[slot] = Value::Builtin {
            name: name.as_str().into(),
            arity: 0,
        };
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        let slot = self.bytecode.intern_global(name) as usize;
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Null);
        }
        self.globals[slot] = value;
    }

    pub fn get_global(&mut self, name: &str) -> Value {
        let slot = self.bytecode.intern_global(name) as usize;
        self.globals.get(slot).cloned().unwrap_or(Value::Null)
    }

    #[inline(always)]
    fn burn_fuel(&mut self) -> Result<(), VmError> {
        if self.fuel.is_none() {
            return Ok(());
        }
        if let Some(f) = self.fuel.as_mut() {
            if *f == 0 {
                return Err(VmError::FuelExhausted);
            }
            *f -= 1;
        }
        Ok(())
    }

    #[inline(always)]
    fn rget(&self, frame_base: usize, r: u16) -> Value {
        let i = frame_base + r as usize;
        if i < self.regs.len() {
            self.regs[i].clone()
        } else {
            Value::Null
        }
    }

    #[inline(always)]
    fn rset(&mut self, frame_base: usize, r: u16, v: Value) {
        let i = frame_base + r as usize;
        if i >= self.regs.len() {
            self.regs.resize(i + 32, Value::Null);
        }
        self.regs[i] = v;
    }

    /// Run the main chunk (chunk 0) to completion.
    pub fn run(&mut self) -> Result<Value, VmError> {
        let n_regs = self.chunks.first().map(|c| c.n_regs).unwrap_or(16) as usize;
        let n_regs = n_regs.max(8);
        self.regs = vec![Value::Null; n_regs + 64];
        self.frames.push(RegFrame {
            chunk: 0,
            ip: 0,
            base: 0,
            n_regs,
            upvalues: Vec::new(),
            argc: 0,
            ret_dst: 0,
        });
        self.run_loop()
    }

    fn run_loop(&mut self) -> Result<Value, VmError> {
        loop {
            self.burn_fuel()?;
            let (chunk_idx, ip) = {
                let f = self
                    .frames
                    .last()
                    .ok_or(VmError::Internal("no frame".into()))?;
                (f.chunk, f.ip)
            };
            let op = {
                let ch = &self.chunks[chunk_idx];
                if ip >= ch.code.len() {
                    return Err(VmError::Internal("reg IP out of bounds".into()));
                }
                ch.code[ip]
            };
            if let Some(f) = self.frames.last_mut() {
                f.ip += 1;
            }
            let base = self.frames.last().map(|f| f.base).unwrap_or(0);

            match op {
                RegOp::Nop => {}
                RegOp::Mov { dst, src } => {
                    let v = self.rget(base, src);
                    self.rset(base, dst, v);
                }
                RegOp::LoadConst { dst, k } => {
                    let v = self.chunks[chunk_idx]
                        .constants
                        .get(k as usize)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.rset(base, dst, v);
                }
                RegOp::LoadGlobal { dst, slot } => {
                    let v = self
                        .globals
                        .get(slot as usize)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.rset(base, dst, v);
                }
                RegOp::StoreGlobal { slot, src } => {
                    let v = self.rget(base, src);
                    let slot = slot as usize;
                    if slot >= self.globals.len() {
                        self.globals.resize(slot + 1, Value::Null);
                    }
                    self.globals[slot] = v;
                }
                RegOp::LoadUpvalue { dst, idx } => {
                    let v = self
                        .frames
                        .last()
                        .and_then(|f| f.upvalues.get(idx as usize).copied())
                        .and_then(|i| self.upvalue_store.get(i).cloned())
                        .unwrap_or(Value::Null);
                    self.rset(base, dst, v);
                }
                RegOp::StoreUpvalue { idx, src } => {
                    let v = self.rget(base, src);
                    if let Some(slot) = self
                        .frames
                        .last()
                        .and_then(|f| f.upvalues.get(idx as usize).copied())
                    {
                        if slot < self.upvalue_store.len() {
                            self.upvalue_store[slot] = v;
                        }
                    }
                }
                RegOp::Bin {
                    op: kind,
                    dst,
                    lhs,
                    rhs,
                } => {
                    let a = self.rget(base, lhs);
                    let b = self.rget(base, rhs);
                    let v = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => {
                            use BinKind::*;
                            match kind {
                                Add => Value::Int(x.wrapping_add(*y)),
                                Sub => Value::Int(x.wrapping_sub(*y)),
                                Mul => Value::Int(x.wrapping_mul(*y)),
                                Div => {
                                    if *y == 0 {
                                        return Err(VmError::DivisionByZero);
                                    }
                                    Value::Int(x.wrapping_div(*y))
                                }
                                Mod => {
                                    if *y == 0 {
                                        return Err(VmError::DivisionByZero);
                                    }
                                    Value::Int(x.wrapping_rem(*y))
                                }
                                _ => bin_apply(kind, &a, &b)?,
                            }
                        }
                        _ => bin_apply(kind, &a, &b)?,
                    };
                    self.rset(base, dst, v);
                }
                RegOp::BinImm {
                    op: kind,
                    dst,
                    lhs,
                    imm,
                } => {
                    let a = self.rget(base, lhs);
                    let v = bin_apply(kind, &a, &Value::Int(imm))?;
                    self.rset(base, dst, v);
                }
                RegOp::Un { op: kind, dst, src } => {
                    let a = self.rget(base, src);
                    let v = match kind {
                        UnKind::Neg => match a {
                            Value::Int(n) => Value::Int(-n),
                            Value::Float(f) => Value::Float(-f),
                            _ => return Err(VmError::TypeError("Cannot negate non-number".into())),
                        },
                        UnKind::Not => Value::Bool(!truthy(&a)),
                        UnKind::BitNot => match a {
                            Value::Int(n) => Value::Int(!n),
                            _ => return Err(VmError::TypeError("BitNot requires int".into())),
                        },
                    };
                    self.rset(base, dst, v);
                }
                RegOp::Cmp {
                    op: kind,
                    dst,
                    lhs,
                    rhs,
                } => {
                    let a = self.rget(base, lhs);
                    let b = self.rget(base, rhs);
                    let v = Value::Bool(match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => {
                            use CmpKind::*;
                            match kind {
                                Eq => x == y,
                                Ne => x != y,
                                Lt => x < y,
                                Le => x <= y,
                                Gt => x > y,
                                Ge => x >= y,
                            }
                        }
                        _ => cmp_apply(kind, &a, &b),
                    });
                    self.rset(base, dst, v);
                }
                RegOp::CmpImm {
                    op: kind,
                    dst,
                    lhs,
                    imm,
                } => {
                    let a = self.rget(base, lhs);
                    let v = Value::Bool(cmp_apply(kind, &a, &Value::Int(imm)));
                    self.rset(base, dst, v);
                }
                RegOp::Jump { target } => {
                    if let Some(f) = self.frames.last_mut() {
                        f.ip = target as usize;
                    }
                }
                RegOp::JumpIfFalse { target, cond } => {
                    let c = self.rget(base, cond);
                    if !truthy(&c) {
                        if let Some(f) = self.frames.last_mut() {
                            f.ip = target as usize;
                        }
                    }
                }
                RegOp::JumpIfTrue { target, cond } => {
                    let c = self.rget(base, cond);
                    if truthy(&c) {
                        if let Some(f) = self.frames.last_mut() {
                            f.ip = target as usize;
                        }
                    }
                }
                RegOp::Call {
                    dst,
                    func,
                    args,
                    argc,
                } => {
                    let fval = self.rget(base, func);
                    let ret = self.call_value(fval, base, args, argc, dst)?;
                    if let Some(v) = ret {
                        return Ok(v);
                    }
                }
                RegOp::TailCall { func, args, argc } => {
                    let fval = self.rget(base, func);
                    // `args` live in the current frame; `ret_dst` is in the parent.
                    let ret_dst = self.frames.last().map(|f| f.ret_dst).unwrap_or(0);
                    let args_base = base;
                    self.frames.pop();
                    let ret = self.call_value(fval, args_base, args, argc, ret_dst)?;
                    if let Some(v) = ret {
                        return Ok(v);
                    }
                }
                RegOp::Ret { src } => {
                    let v = self.rget(base, src);
                    if let Some(frame) = self.frames.pop() {
                        // Free callee registers so the file does not grow unbounded.
                        self.regs.truncate(frame.base);
                        // `ret_dst` is relative to the **caller's** register file.
                        if let Some(caller) = self.frames.last() {
                            self.rset(caller.base, frame.ret_dst, v.clone());
                        }
                        if self.frames.is_empty() {
                            return Ok(v);
                        }
                    } else {
                        return Ok(v);
                    }
                }
                RegOp::Halt { src } => {
                    return Ok(self.rget(base, src));
                }
                RegOp::GetProp { dst, obj, name } => {
                    let o = self.rget(base, obj);
                    let key = self.const_str(chunk_idx, name)?;
                    let v = get_prop(&o, &key)?;
                    self.rset(base, dst, v);
                }
                RegOp::SetProp { obj, name, src } => {
                    let o = self.rget(base, obj);
                    let v = self.rget(base, src);
                    let key = self.const_str(chunk_idx, name)?;
                    set_prop(&o, &key, v)?;
                    self.rset(base, obj, o);
                }
                RegOp::Invoke {
                    dst,
                    obj,
                    name,
                    args,
                    argc,
                } => {
                    let o = self.rget(base, obj);
                    let key = self.const_str(chunk_idx, name)?;
                    let ret = self.invoke(o, &key, base, args, argc, dst)?;
                    if let Some(v) = ret {
                        return Ok(v);
                    }
                }
                RegOp::BuildArr { dst, start, count } => {
                    let mut items = Vec::with_capacity(count as usize);
                    for i in 0..count {
                        items.push(self.rget(base, start + i));
                    }
                    self.rset(base, dst, Value::Array(Rc::new(RefCell::new(items))));
                }
                RegOp::BuildMap { dst, start, count } => {
                    let mut pairs = Vec::new();
                    let mut i = 0u16;
                    while i + 1 < count {
                        let k = self.rget(base, start + i);
                        let v = self.rget(base, start + i + 1);
                        pairs.push((k, v));
                        i += 2;
                    }
                    self.rset(base, dst, Value::Map(Rc::new(RefCell::new(pairs))));
                }
                RegOp::IndexGet { dst, obj, idx } => {
                    let o = self.rget(base, obj);
                    let i = self.rget(base, idx);
                    let v = index_get(&o, &i)?;
                    self.rset(base, dst, v);
                }
                RegOp::IndexSet { obj, idx, src } => {
                    let o = self.rget(base, obj);
                    let i = self.rget(base, idx);
                    let v = self.rget(base, src);
                    index_set(&o, &i, v)?;
                }
                RegOp::CreateClass { dst, k } => {
                    let v = self.chunks[chunk_idx]
                        .constants
                        .get(k as usize)
                        .cloned()
                        .unwrap_or(Value::Null);
                    if let Value::Class(cls) = &v {
                        self.class_table.push(RegClass {
                            name: cls.name.to_string(),
                            superclass: cls.superclass.as_ref().map(|s| s.to_string()),
                            methods: cls.methods.iter().map(|(n, ci)| (n.clone(), *ci)).collect(),
                            properties: cls.properties.as_ref().clone(),
                        });
                    }
                    self.rset(base, dst, v);
                }
                RegOp::CreateInstance {
                    dst,
                    class,
                    args,
                    argc,
                } => {
                    // Mirror stack Call-on-Class: allocate + run init.
                    let c = self.rget(base, class);
                    let ret = self.call_value(c, base, args, argc, dst)?;
                    if let Some(v) = ret {
                        return Ok(v);
                    }
                }
                RegOp::MakeClosure {
                    dst,
                    func,
                    upvals,
                    count,
                } => {
                    let f = self.rget(base, func);
                    let mut uv_idx = Vec::new();
                    for i in 0..count as u16 {
                        let v = self.rget(base, upvals + i);
                        let slot = self.upvalue_store.len();
                        self.upvalue_store.push(v);
                        uv_idx.push(slot);
                    }
                    if let Value::Function(fobj) = f {
                        self.rset(
                            base,
                            dst,
                            Value::Closure(Rc::new(mailang_bytecode::ClosureObj {
                                function_index: fobj.chunk_index,
                                arity: fobj.arity,
                                required: fobj.required,
                                upvalues: uv_idx,
                            })),
                        );
                    }
                }
                RegOp::Wrap { kind, dst, src } => {
                    let v = self.rget(base, src);
                    let w = match kind {
                        WrapKind::Ok => Value::Ok(Box::new(v)),
                        WrapKind::Err => Value::Err(Box::new(v)),
                        WrapKind::Some => Value::Some(Box::new(v)),
                    };
                    self.rset(base, dst, w);
                }
                RegOp::Unwrap {
                    kind,
                    dst,
                    flag,
                    src,
                } => {
                    let v = self.rget(base, src);
                    let (inner, ok) = match (kind, v) {
                        (WrapKind::Ok, Value::Ok(i)) => (*i, true),
                        (WrapKind::Err, Value::Err(i)) => (*i, true),
                        (WrapKind::Some, Value::Some(i)) => (*i, true),
                        (_, other) => (other, false),
                    };
                    // Fail: write false to both dst and flag so a following
                    // JumpIfFalse peeks `flag` and the fail-block top (`dst`)
                    // is also the boolean.
                    if ok {
                        self.rset(base, dst, inner);
                        self.rset(base, flag, Value::Bool(true));
                    } else {
                        self.rset(base, dst, Value::Bool(false));
                        self.rset(base, flag, Value::Bool(false));
                    }
                }
                RegOp::TryQ { dst, src } => {
                    let v = self.rget(base, src);
                    match v {
                        Value::Ok(i) | Value::Some(i) => {
                            self.rset(base, dst, *i);
                        }
                        other => {
                            // Early-return the Err/None from the frame.
                            if let Some(frame) = self.frames.pop() {
                                if let Some(caller) = self.frames.last() {
                                    self.rset(caller.base, frame.ret_dst, other.clone());
                                }
                                if self.frames.is_empty() {
                                    return Ok(other);
                                }
                            } else {
                                return Ok(other);
                            }
                        }
                    }
                }
                RegOp::Argc { dst } => {
                    let n = self.frames.last().map(|f| f.argc).unwrap_or(0);
                    self.rset(base, dst, Value::Int(n as i64));
                }
                RegOp::CallChunk {
                    dst,
                    chunk,
                    args,
                    argc,
                } => {
                    // Stack IR CallDirect does not carry a FunctionObj, so
                    // RegChunk.arity is often unset (0). Accept the provided argc.
                    self.enter_chunk(
                        chunk as usize,
                        argc as usize,
                        0,
                        base,
                        args,
                        argc,
                        dst,
                        Vec::new(),
                    )?;
                }
            }
        }
    }

    fn const_str(&self, chunk: usize, k: u32) -> Result<String, VmError> {
        match self.chunks[chunk].constants.get(k as usize) {
            Some(Value::Str(s)) => Ok(s.to_string()),
            _ => Err(VmError::Internal("expected string constant".into())),
        }
    }

    /// Invoke a function value. Returns `Some(v)` if the program returned.
    ///
    /// `args_base` is the register-file base of the frame that owns `args`
    /// (the caller for a normal call; the discarded frame for a tail call).
    /// `ret_dst` is relative to the frame that will still be on the stack
    /// after the call (the original caller).
    fn call_value(
        &mut self,
        fval: Value,
        args_base: usize,
        args: u16,
        argc: u8,
        ret_dst: u16,
    ) -> Result<Option<Value>, VmError> {
        match fval {
            Value::Function(f) => self
                .enter_chunk(
                    f.chunk_index,
                    f.arity,
                    f.required,
                    args_base,
                    args,
                    argc,
                    ret_dst,
                    Vec::new(),
                )
                .map(|_| None),
            Value::Closure(c) => self
                .enter_chunk(
                    c.function_index,
                    c.arity,
                    c.required,
                    args_base,
                    args,
                    argc,
                    ret_dst,
                    c.upvalues.clone(),
                )
                .map(|_| None),
            Value::Class(cls) => {
                // Constructor: allocate instance from property defaults, then
                // run `init` with `this` as local 0 and user args as 1.. .
                let class_idx = self.class_table.len();
                self.class_table.push(RegClass {
                    name: cls.name.to_string(),
                    superclass: cls.superclass.as_ref().map(|s| s.to_string()),
                    methods: cls.methods.iter().map(|(n, ci)| (n.clone(), *ci)).collect(),
                    properties: cls.properties.as_ref().clone(),
                });
                let mut fields = Vec::new();
                for (n, v) in cls.properties.iter() {
                    fields.push((n.clone(), v.clone()));
                }
                let instance = Value::Instance {
                    class_index: class_idx,
                    fields: Rc::new(RefCell::new(fields)),
                };
                self.gc.track(&instance);
                let init_chunk = cls
                    .methods
                    .iter()
                    .find(|(n, _)| n == "init")
                    .map(|(_, ci)| *ci);
                if let Some(chunk) = init_chunk {
                    self.enter_method(chunk, instance, args_base, args, argc, ret_dst)?;
                    Ok(None)
                } else {
                    let ret_base = self.frames.last().map(|f| f.base).unwrap_or(0);
                    self.rset(ret_base, ret_dst, instance);
                    Ok(None)
                }
            }
            Value::Builtin { name, .. } => {
                let mut vals = Vec::new();
                for i in 0..argc as u16 {
                    vals.push(self.rget(args_base, args + i));
                }
                let out = if let Some(host) = self.host_fns.get(name.as_ref()) {
                    host(&vals)
                } else if let Some(b) = self.builtins.get(name.as_ref()) {
                    b(&vals)
                } else {
                    return Err(VmError::UndefinedFunction(name.to_string()));
                };
                let out = out.map_err(VmError::RuntimeError)?;
                let ret_base = self.frames.last().map(|f| f.base).unwrap_or(0);
                self.rset(ret_base, ret_dst, out);
                Ok(None)
            }
            _ => Err(VmError::TypeError("Cannot call non-function".into())),
        }
    }

    /// Enter a method/constructor chunk: local 0 = `this`, locals 1.. = args.
    fn enter_method(
        &mut self,
        chunk: usize,
        this: Value,
        args_base: usize,
        args: u16,
        argc: u8,
        ret_dst: u16,
    ) -> Result<(), VmError> {
        if self.frames.len() >= self.max_call_depth {
            return Err(VmError::CallDepthExceeded {
                limit: self.max_call_depth,
            });
        }
        let n_regs = self.chunks.get(chunk).map(|c| c.n_regs).unwrap_or(8).max(8) as usize;
        let base = self.regs.len();
        self.regs.resize(base + n_regs + 16, Value::Null);
        self.rset(base, 0, this);
        for i in 0..argc as u16 {
            let v = self.rget(args_base, args + i);
            self.rset(base, i + 1, v);
        }
        self.frames.push(RegFrame {
            chunk,
            ip: 0,
            base,
            n_regs,
            upvalues: Vec::new(),
            argc: argc as usize,
            ret_dst,
        });
        Ok(())
    }

    fn enter_chunk(
        &mut self,
        chunk: usize,
        arity: usize,
        required: usize,
        args_base: usize,
        args: u16,
        argc: u8,
        ret_dst: u16,
        upvalues: Vec<usize>,
    ) -> Result<(), VmError> {
        if self.frames.len() >= self.max_call_depth {
            return Err(VmError::CallDepthExceeded {
                limit: self.max_call_depth,
            });
        }
        let argc_us = argc as usize;
        if argc_us > arity || argc_us < required {
            return Err(VmError::WrongArgumentCount {
                expected: arity,
                found: argc_us,
            });
        }
        let n_regs = self.chunks[chunk].n_regs.max(8) as usize;
        let base = self.regs.len();
        self.regs.resize(base + n_regs + 16, Value::Null);
        // Copy args into the new frame鈥檚 param registers.
        for i in 0..argc_us {
            let v = self.rget(args_base, args + i as u16);
            self.rset(base, i as u16, v);
        }
        self.frames.push(RegFrame {
            chunk,
            ip: 0,
            base,
            n_regs,
            upvalues,
            argc: argc_us,
            ret_dst,
        });
        Ok(())
    }

    fn invoke(
        &mut self,
        obj: Value,
        method: &str,
        base: usize,
        args: u16,
        argc: u8,
        dst: u16,
    ) -> Result<Option<Value>, VmError> {
        // Builtin collection/string methods fall back to the stack Vm helpers
        // via a tiny shim: run them as builtins when possible.
        let mut vals = Vec::with_capacity(argc as usize + 1);
        vals.push(obj.clone());
        for i in 0..argc as u16 {
            vals.push(self.rget(base, args + i));
        }
        // Instance method: look up Function/Closure in instance map or class 鈥?simplified:
        // treat missing methods as errors (same as stack VM user methods need class table).
        if let Value::Map(entries) = &obj {
            for (k, v) in entries.borrow().iter() {
                if let Value::Str(s) = k {
                    if s.as_ref() == method {
                        let f = v.clone();
                        return self.call_value(f, base, args, argc, dst);
                    }
                }
            }
        }
        // Array/map/str builtins via a throwaway stack Vm is too heavy; call
        // the stdlib methods directly.
        match &obj {
            Value::Array(arr) => {
                let argvals: Vec<Value> = vals[1..].to_vec();
                let out = invoke_support::array_method(arr, method, &argvals)?;
                self.rset(base, dst, out);
                Ok(None)
            }
            Value::Map(entries) => {
                let argvals: Vec<Value> = vals[1..].to_vec();
                let out = invoke_support::map_method(entries, method, &argvals)?;
                self.rset(base, dst, out);
                Ok(None)
            }
            Value::Str(s) => {
                let argvals: Vec<Value> = vals[1..].to_vec();
                let out = invoke_support::str_method(s, method, &argvals)?;
                self.rset(base, dst, out);
                Ok(None)
            }
            Value::Instance {
                fields,
                class_index,
            } => {
                // Instance field holding a callable wins; else class method.
                for (k, v) in fields.borrow().iter() {
                    if k == method {
                        let f = v.clone();
                        return self.call_value(f, base, args, argc, dst);
                    }
                }
                if let Some(cls) = self.class_table.get(*class_index).cloned() {
                    if let Some((_, chunk)) = cls.methods.iter().find(|(n, _)| n == method) {
                        let chunk = *chunk;
                        return self
                            .enter_method(chunk, obj, base, args, argc, dst)
                            .map(|_| None);
                    }
                }
                Err(VmError::UndefinedFunction(method.to_string()))
            }
            _ => Err(VmError::TypeError("Cannot invoke on this value".into())),
        }
    }

    /// Break Rc cycles reachable from registers / globals / upvalues.
    pub fn collect_cycles(&mut self) -> usize {
        let mut roots = self.regs.clone();
        roots.extend_from_slice(&self.globals);
        roots.extend(self.upvalue_store.iter().cloned());
        let broken = mailang_gc::collect_cycles(&mut roots);
        for v in roots.iter() {
            self.gc.track(v);
        }
        let freed = self.gc.collect(&roots);
        if broken > 0 {
            broken
        } else {
            freed
        }
    }

    /// Heap statistics: (tracked, collections, freed).
    pub fn gc_stats(&self) -> (usize, usize, usize) {
        (
            self.gc.tracked_count(),
            self.gc.collections,
            self.gc.freed_total,
        )
    }

    fn maybe_auto_collect(&mut self) {
        if self.gc.should_collect() {
            let mut roots = self.regs.clone();
            roots.extend_from_slice(&self.globals);
            roots.extend(self.upvalue_store.iter().cloned());
            for v in roots.iter() {
                self.gc.track(v);
            }
            self.gc.collect(&roots);
        }
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty(),
        _ => true,
    }
}

fn bin_apply(kind: BinKind, a: &Value, b: &Value) -> Result<Value, VmError> {
    use BinKind::*;
    match (kind, a, b) {
        (Add, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
        (Add, Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
        (Add, Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 + y)),
        (Add, Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + *y as f64)),
        (Add, Value::Str(x), Value::Str(y)) => Ok(Value::Str(format!("{}{}", x, y).into())),
        (Add, Value::Str(x), other) => Ok(Value::Str(
            format!("{}{}", x, mailang_stdlib::value_to_string(other)).into(),
        )),
        (Add, other, Value::Str(y)) => Ok(Value::Str(
            format!("{}{}", mailang_stdlib::value_to_string(other), y).into(),
        )),
        (Sub, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x - y)),
        (Sub, Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
        (Mul, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x * y)),
        (Mul, Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
        (Div, Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                Err(VmError::DivisionByZero)
            } else {
                Ok(Value::Int(x / y))
            }
        }
        (Div, Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
        (Mod, Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                Err(VmError::DivisionByZero)
            } else {
                Ok(Value::Int(x % y))
            }
        }
        (Pow, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.pow(*y as u32))),
        (BitAnd, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x & y)),
        (BitOr, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x | y)),
        (BitXor, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x ^ y)),
        (Shl, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x << y)),
        (Shr, Value::Int(x), Value::Int(y)) => Ok(Value::Int(x >> y)),
        (And, _, _) => Ok(Value::Bool(truthy(a) && truthy(b))),
        (Or, _, _) => Ok(Value::Bool(truthy(a) || truthy(b))),
        _ => Err(VmError::TypeError("Cannot apply binary op".into())),
    }
}

fn cmp_apply(kind: CmpKind, a: &Value, b: &Value) -> bool {
    use CmpKind::*;
    match kind {
        Eq => a == b,
        Ne => a != b,
        Lt => match (a, b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            (Value::Float(x), Value::Float(y)) => x < y,
            (Value::Str(x), Value::Str(y)) => x < y,
            _ => false,
        },
        Le => match (a, b) {
            (Value::Int(x), Value::Int(y)) => x <= y,
            (Value::Float(x), Value::Float(y)) => x <= y,
            (Value::Str(x), Value::Str(y)) => x <= y,
            _ => false,
        },
        Gt => match (a, b) {
            (Value::Int(x), Value::Int(y)) => x > y,
            (Value::Float(x), Value::Float(y)) => x > y,
            (Value::Str(x), Value::Str(y)) => x > y,
            _ => false,
        },
        Ge => match (a, b) {
            (Value::Int(x), Value::Int(y)) => x >= y,
            (Value::Float(x), Value::Float(y)) => x >= y,
            (Value::Str(x), Value::Str(y)) => x >= y,
            _ => false,
        },
    }
}

fn get_prop(o: &Value, key: &str) -> Result<Value, VmError> {
    match o {
        Value::Instance { fields, .. } => fields
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| VmError::UndefinedProperty(key.into())),
        Value::Map(m) => m
            .borrow()
            .iter()
            .find(|(k, _)| matches!(k, Value::Str(s) if s.as_ref() == key))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| VmError::UndefinedProperty(key.into())),
        Value::Array(a) if key == "len" => Ok(Value::Int(a.borrow().len() as i64)),
        Value::Str(s) if key == "len" => Ok(Value::Int(s.chars().count() as i64)),
        _ => Err(VmError::UndefinedProperty(key.into())),
    }
}

fn set_prop(o: &Value, key: &str, v: Value) -> Result<(), VmError> {
    match o {
        Value::Instance { fields, .. } => {
            let mut f = fields.borrow_mut();
            if let Some(slot) = f.iter_mut().find(|(k, _)| k == key) {
                slot.1 = v;
            } else {
                f.push((key.to_string(), v));
            }
            Ok(())
        }
        Value::Map(m) => {
            let mut f = m.borrow_mut();
            if let Some(slot) = f
                .iter_mut()
                .find(|(k, _)| matches!(k, Value::Str(s) if s.as_ref() == key))
            {
                slot.1 = v;
            } else {
                f.push((Value::Str(key.into()), v));
            }
            Ok(())
        }
        _ => Err(VmError::TypeError("Cannot set property".into())),
    }
}

fn index_get(o: &Value, i: &Value) -> Result<Value, VmError> {
    match (o, i) {
        (Value::Array(a), Value::Int(n)) => {
            let a = a.borrow();
            if *n < 0 || *n as usize >= a.len() {
                Err(VmError::IndexOutOfBounds {
                    index: *n,
                    length: a.len(),
                })
            } else {
                Ok(a[*n as usize].clone())
            }
        }
        (Value::Map(m), k) => m
            .borrow()
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| VmError::UndefinedProperty("key".into())),
        (Value::Tuple(t), Value::Int(n)) => {
            let t = t.borrow();
            if *n < 0 || *n as usize >= t.len() {
                Err(VmError::IndexOutOfBounds {
                    index: *n,
                    length: t.len(),
                })
            } else {
                Ok(t[*n as usize].clone())
            }
        }
        _ => Err(VmError::TypeError("Cannot index".into())),
    }
}

fn index_set(o: &Value, i: &Value, v: Value) -> Result<(), VmError> {
    match (o, i) {
        (Value::Array(a), Value::Int(n)) => {
            let mut a = a.borrow_mut();
            if *n < 0 || *n as usize >= a.len() {
                Err(VmError::IndexOutOfBounds {
                    index: *n,
                    length: a.len(),
                })
            } else {
                a[*n as usize] = v;
                Ok(())
            }
        }
        (Value::Map(m), k) => {
            let mut m = m.borrow_mut();
            if let Some(slot) = m.iter_mut().find(|(key, _)| key == k) {
                slot.1 = v;
            } else {
                m.push((k.clone(), v));
            }
            Ok(())
        }
        _ => Err(VmError::TypeError("Cannot index-assign".into())),
    }
}

fn register_builtins(map: &mut HashMap<String, fn(&[Value]) -> Result<Value, String>>) {
    // Mirror the stack VM's stdlib surface so both backends see one API.
    map.insert("println".into(), mailang_stdlib::builtin_println);
    map.insert("print".into(), mailang_stdlib::builtin_print);
    map.insert("input".into(), mailang_stdlib::builtin_input);
    map.insert("sqrt".into(), mailang_stdlib::builtin_sqrt);
    map.insert("abs".into(), mailang_stdlib::builtin_abs);
    map.insert("sin".into(), mailang_stdlib::builtin_sin);
    map.insert("cos".into(), mailang_stdlib::builtin_cos);
    map.insert("floor".into(), mailang_stdlib::builtin_floor);
    map.insert("ceil".into(), mailang_stdlib::builtin_ceil);
    map.insert("round".into(), mailang_stdlib::builtin_round);
    map.insert("min".into(), mailang_stdlib::builtin_min);
    map.insert("max".into(), mailang_stdlib::builtin_max);
    map.insert("len".into(), mailang_stdlib::builtin_len);
    map.insert("to_string".into(), mailang_stdlib::builtin_to_string);
    map.insert("parse_int".into(), mailang_stdlib::builtin_parse_int);
    map.insert("parse_float".into(), mailang_stdlib::builtin_parse_float);
    map.insert("time_now".into(), mailang_stdlib::builtin_time_now);
    map.insert(
        "time_now_secs".into(),
        mailang_stdlib::builtin_time_now_secs,
    );
    map.insert("time_year".into(), mailang_stdlib::builtin_time_year);
    map.insert("time_month".into(), mailang_stdlib::builtin_time_month);
    map.insert("time_day".into(), mailang_stdlib::builtin_time_day);
    map.insert("time_hour".into(), mailang_stdlib::builtin_time_hour);
    map.insert("time_minute".into(), mailang_stdlib::builtin_time_minute);
    map.insert("time_second".into(), mailang_stdlib::builtin_time_second);
    map.insert("time_date".into(), mailang_stdlib::builtin_time_date);
    map.insert(
        "time_datetime".into(),
        mailang_stdlib::builtin_time_datetime,
    );
    map.insert("time_elapsed".into(), mailang_stdlib::builtin_time_elapsed);
    map.insert("time_sleep".into(), mailang_stdlib::builtin_time_sleep);
    map.insert("read_file".into(), mailang_stdlib::builtin_read_file);
    map.insert("write_file".into(), mailang_stdlib::builtin_write_file);
    map.insert("gpio_write".into(), mailang_stdlib::hal::builtin_gpio_write);
    map.insert("gpio_read".into(), mailang_stdlib::hal::builtin_gpio_read);
    map.insert("delay_ms".into(), mailang_stdlib::hal::builtin_delay_ms);
    map.insert("adc_read".into(), mailang_stdlib::hal::builtin_adc_read);
    map.insert("pwm_write".into(), mailang_stdlib::hal::builtin_pwm_write);
    map.insert("pwm_freq".into(), mailang_stdlib::hal::builtin_pwm_freq);
    map.insert("uart_write".into(), mailang_stdlib::hal::builtin_uart_write);
    map.insert("uart_read".into(), mailang_stdlib::hal::builtin_uart_read);
    map.insert("i2c_xfer".into(), mailang_stdlib::hal::builtin_i2c_xfer);
    map.insert("spi_xfer".into(), mailang_stdlib::hal::builtin_spi_xfer);
    map.insert("json_parse".into(), mailang_stdlib::builtin_json_parse);
    map.insert(
        "json_stringify".into(),
        mailang_stdlib::builtin_json_stringify,
    );
    map.insert("env".into(), mailang_stdlib::builtin_env);
    map.insert("process_exit".into(), mailang_stdlib::builtin_process_exit);
}

/// Bridge to the stack VM鈥檚 collection-method implementations so the register
/// VM does not duplicate them.
pub mod invoke_support {
    use crate::error::VmError;
    use mailang_bytecode::Value;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub fn array_method(
        arr: &Rc<RefCell<Vec<Value>>>,
        name: &str,
        args: &[Value],
    ) -> Result<Value, VmError> {
        crate::vm::invoke_array_method(arr, name, args)
    }

    pub fn map_method(
        m: &Rc<RefCell<Vec<(Value, Value)>>>,
        name: &str,
        args: &[Value],
    ) -> Result<Value, VmError> {
        crate::vm::invoke_map_method(m, name, args)
    }

    pub fn str_method(s: &Rc<str>, name: &str, args: &[Value]) -> Result<Value, VmError> {
        crate::vm::invoke_str_method(s, name, args)
    }
}

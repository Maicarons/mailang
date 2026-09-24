//! Register-based bytecode and a stack→register translator.
//!
//! The classic compiler emits stack bytecode. A real **register VM** executes
//! three-address instructions (`Add dst, lhs, rhs`) with a flat register file.
//! This module defines that IR and a translator that lowers stack bytecode to
//! register bytecode by simulating the operand stack and assigning each stack
//! slot a virtual register.

use crate::{Opcode, Value};

/// Three-address register instruction. Registers are `u16` indices into a
/// flat frame-local register file. Constants live in the chunk const table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegOp {
    /// `dst = src`
    Mov { dst: u16, src: u16 },
    /// `dst = constants[k]`
    LoadConst { dst: u16, k: u32 },
    /// `dst = globals[slot]`
    LoadGlobal { dst: u16, slot: u32 },
    /// `globals[slot] = src`
    StoreGlobal { slot: u32, src: u16 },
    /// `dst = upvalues[idx]`
    LoadUpvalue { dst: u16, idx: u32 },
    /// `upvalues[idx] = src`
    StoreUpvalue { idx: u32, src: u16 },
    /// `dst = lhs <op> rhs`
    Bin {
        op: BinKind,
        dst: u16,
        lhs: u16,
        rhs: u16,
    },
    /// `dst = <op> src`
    Un { op: UnKind, dst: u16, src: u16 },
    /// `dst = lhs <cmp> rhs`
    Cmp {
        op: CmpKind,
        dst: u16,
        lhs: u16,
        rhs: u16,
    },
    /// `dst = lhs <cmp> imm` (fused int immediate)
    CmpImm {
        op: CmpKind,
        dst: u16,
        lhs: u16,
        imm: i64,
    },
    /// `dst = lhs <op> imm`
    BinImm {
        op: BinKind,
        dst: u16,
        lhs: u16,
        imm: i64,
    },
    /// Unconditional jump to instruction index `target` (in this chunk).
    Jump { target: u32 },
    /// Jump if `cond` register is falsy.
    JumpIfFalse { target: u32, cond: u16 },
    /// Jump if `cond` register is truthy.
    JumpIfTrue { target: u32, cond: u16 },
    /// `dst = call(func, args[argc])` — args live in consecutive registers.
    Call {
        dst: u16,
        func: u16,
        args: u16,
        argc: u8,
    },
    /// Tail call: same as Call but reuses the frame.
    TailCall { func: u16, args: u16, argc: u8 },
    /// `return src`
    Ret { src: u16 },
    /// `dst = obj.name`
    GetProp { dst: u16, obj: u16, name: u32 },
    /// `obj.name = src`
    SetProp { obj: u16, name: u32, src: u16 },
    /// `dst = obj.name(args[argc])`
    Invoke {
        dst: u16,
        obj: u16,
        name: u32,
        args: u16,
        argc: u8,
    },
    /// `dst = [regs[start] .. start+count)`
    BuildArr { dst: u16, start: u16, count: u16 },
    /// `dst = { k/v pairs from regs[start] .. }`
    BuildMap { dst: u16, start: u16, count: u16 },
    /// `dst = obj[idx]`
    IndexGet { dst: u16, obj: u16, idx: u16 },
    /// `obj[idx] = src`
    IndexSet { obj: u16, idx: u16, src: u16 },
    /// Class / instance helpers (operand carries the same meaning as stack IR).
    CreateClass { dst: u16, k: u32 },
    CreateInstance {
        dst: u16,
        class: u16,
        args: u16,
        argc: u8,
    },
    MakeClosure {
        dst: u16,
        func: u16,
        upvals: u16,
        count: u8,
    },
    /// `dst = wrap(src)` for Ok/Err/Some
    Wrap { kind: WrapKind, dst: u16, src: u16 },
    /// Unwrap Ok/Err/Some: `dst = inner` (bool success in `flag`)
    Unwrap {
        kind: WrapKind,
        dst: u16,
        flag: u16,
        src: u16,
    },
    /// `?` operator: dst = inner or early-return
    TryQ { dst: u16, src: u16 },
    /// Current frame argc (for default-parameter prologues)
    Argc { dst: u16 },
    /// Call a known chunk by index (lowered from stack `CallDirect`).
    CallChunk {
        dst: u16,
        chunk: u32,
        args: u16,
        argc: u8,
    },
    /// No-op
    Nop,
    /// Stop and return `src`
    Halt { src: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnKind {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmpKind {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WrapKind {
    Ok,
    Err,
    Some,
}

/// A register-mode chunk (function body).
#[derive(Debug, Clone)]
pub struct RegChunk {
    pub name: String,
    pub code: Vec<RegOp>,
    pub constants: Vec<Value>,
    /// Number of registers the frame must allocate (max index + 1).
    pub n_regs: u16,
    /// Declared parameter count (max).
    pub arity: usize,
    pub required: usize,
}

impl RegChunk {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            code: Vec::new(),
            constants: Vec::new(),
            n_regs: 0,
            arity: 0,
            required: 0,
        }
    }
}

/// Lower a stack instruction stream to register form.
///
/// Strategy: simulate the operand stack. Each stack slot at depth `d` maps to
/// register `base + d`. Temporaries that need destinations reuse the top slot
/// (so `Add` is `dst = pop(); lhs = pop(); rhs = pop(); push dst` in stack
/// terms, which becomes `Add dst=sp-2, lhs=sp-2, rhs=sp-1` then `sp -= 2`).
pub struct StackToRegister<'a> {
    src_code: &'a [crate::Instruction],
    #[allow(dead_code)]
    src_consts: &'a [Value],
    out: RegChunk,
    /// Maps stack IP → register IP (for jump patching).
    ip_map: Vec<u32>,
    /// Pending (stack_target, reg_site) jumps to patch.
    jumps: Vec<(usize, usize)>,
}

impl<'a> StackToRegister<'a> {
    pub fn new(name: &str, code: &'a [crate::Instruction], consts: &'a [Value]) -> Self {
        let mut out = RegChunk::new(name);
        out.constants = consts.to_vec();
        out.arity = 0;
        out.required = 0;
        Self {
            src_code: code,
            src_consts: consts,
            out,
            ip_map: Vec::new(),
            jumps: Vec::new(),
        }
    }

    fn emit(&mut self, op: RegOp) -> usize {
        self.out.code.push(op);
        self.out.code.len() - 1
    }

    fn use_reg(&mut self, r: u16) {
        if r + 1 > self.out.n_regs {
            self.out.n_regs = r + 1;
        }
    }

    pub fn translate(mut self) -> RegChunk {
        // Virtual stack height (for register assignment of expression slots).
        // Registers 0..n_locals-1 are frame locals (same indices as stack locals).
        // Expression stack occupies registers starting at `n_locals`, so temps
        // never clobber parameters / `let` slots.
        let mut max_local = 0u16;
        for ins in self.src_code {
            if matches!(ins.opcode, Opcode::LoadLocal | Opcode::StoreLocal) {
                if let Some(op) = ins.operand {
                    let idx = op as u16;
                    if idx + 1 > max_local {
                        max_local = idx + 1;
                    }
                }
            }
        }
        let mut sp: u16 = max_local;
        self.out.n_regs = self.out.n_regs.max(max_local);
        for (sip, ins) in self.src_code.iter().enumerate() {
            self.ip_map.push(self.out.code.len() as u32);
            let _ = sip;
            match ins.opcode {
                Opcode::Push => {
                    let k = ins.operand.unwrap_or(0);
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::LoadConst { dst, k });
                    sp += 1;
                }
                Opcode::Pop => {
                    sp = sp.saturating_sub(1);
                }
                Opcode::Dup => {
                    let src = sp.saturating_sub(1);
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::Mov { dst, src });
                    sp += 1;
                }
                Opcode::LoadLocal => {
                    let idx = ins.operand.unwrap_or(0) as u16;
                    let dst = sp;
                    self.use_reg(idx.max(dst));
                    self.emit(RegOp::Mov { dst, src: idx });
                    sp += 1;
                }
                Opcode::StoreLocal => {
                    let idx = ins.operand.unwrap_or(0) as u16;
                    let src = sp.saturating_sub(1);
                    self.use_reg(idx.max(src));
                    self.emit(RegOp::Mov { dst: idx, src });
                    sp = sp.saturating_sub(1);
                }
                Opcode::LoadGlobal => {
                    let slot = ins.operand.unwrap_or(0);
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::LoadGlobal { dst, slot });
                    sp += 1;
                }
                Opcode::StoreGlobal => {
                    let slot = ins.operand.unwrap_or(0);
                    let src = sp.saturating_sub(1);
                    self.use_reg(src);
                    self.emit(RegOp::StoreGlobal { slot, src });
                    sp = sp.saturating_sub(1);
                }
                Opcode::LoadUpvalue => {
                    let idx = ins.operand.unwrap_or(0);
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::LoadUpvalue { dst, idx });
                    sp += 1;
                }
                Opcode::StoreUpvalue => {
                    let idx = ins.operand.unwrap_or(0);
                    let src = sp.saturating_sub(1);
                    self.use_reg(src);
                    self.emit(RegOp::StoreUpvalue { idx, src });
                    sp = sp.saturating_sub(1);
                }
                Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Mod
                | Opcode::Pow
                | Opcode::BitAnd
                | Opcode::BitOr
                | Opcode::BitXor
                | Opcode::Shl
                | Opcode::Shr
                | Opcode::And
                | Opcode::Or => {
                    let kind = bin_kind(ins.opcode);
                    let rhs = sp.saturating_sub(1);
                    let lhs = sp.saturating_sub(2);
                    let dst = lhs;
                    self.use_reg(rhs.max(dst));
                    self.emit(RegOp::Bin {
                        op: kind,
                        dst,
                        lhs,
                        rhs,
                    });
                    sp = sp.saturating_sub(1);
                }
                Opcode::AddImm
                | Opcode::SubImm
                | Opcode::MulImm
                | Opcode::EqImm
                | Opcode::NeImm
                | Opcode::LtImm
                | Opcode::LeImm
                | Opcode::GtImm
                | Opcode::GeImm => {
                    let imm = ins.operand.unwrap_or(0) as i64;
                    let lhs = sp.saturating_sub(1);
                    let dst = lhs;
                    self.use_reg(dst);
                    let op = ins.opcode;
                    if matches!(op, Opcode::AddImm | Opcode::SubImm | Opcode::MulImm) {
                        self.emit(RegOp::BinImm {
                            op: bin_kind_imm(op),
                            dst,
                            lhs,
                            imm,
                        });
                    } else {
                        self.emit(RegOp::CmpImm {
                            op: cmp_kind_imm(op),
                            dst,
                            lhs,
                            imm,
                        });
                    }
                }
                Opcode::Neg | Opcode::Not | Opcode::BitNot => {
                    let src = sp.saturating_sub(1);
                    let dst = src;
                    self.use_reg(dst);
                    self.emit(RegOp::Un {
                        op: un_kind(ins.opcode),
                        dst,
                        src,
                    });
                }
                Opcode::Eq | Opcode::Ne | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge => {
                    let rhs = sp.saturating_sub(1);
                    let lhs = sp.saturating_sub(2);
                    let dst = lhs;
                    self.use_reg(rhs.max(dst));
                    self.emit(RegOp::Cmp {
                        op: cmp_kind(ins.opcode),
                        dst,
                        lhs,
                        rhs,
                    });
                    sp = sp.saturating_sub(1);
                }
                Opcode::Jump => {
                    let t = ins.operand.unwrap_or(0) as usize;
                    let site = self.emit(RegOp::Jump { target: 0 });
                    self.jumps.push((t, site));
                }
                Opcode::JumpIfFalse => {
                    let t = ins.operand.unwrap_or(0) as usize;
                    let cond = sp.saturating_sub(1);
                    self.use_reg(cond);
                    let site = self.emit(RegOp::JumpIfFalse { target: 0, cond });
                    self.jumps.push((t, site));
                }
                Opcode::JumpIfTrue => {
                    let t = ins.operand.unwrap_or(0) as usize;
                    let cond = sp.saturating_sub(1);
                    self.use_reg(cond);
                    let site = self.emit(RegOp::JumpIfTrue { target: 0, cond });
                    self.jumps.push((t, site));
                }
                Opcode::Call => {
                    let argc = ins.operand.unwrap_or(0) as u8;
                    let argc_us = argc as u16;
                    // stack: [func, a0, a1, …]
                    let func = sp.saturating_sub(argc_us + 1);
                    let args = func + 1;
                    let dst = func;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::Call {
                        dst,
                        func,
                        args,
                        argc,
                    });
                    sp = func + 1;
                }
                Opcode::TailCall => {
                    let argc = ins.operand.unwrap_or(0) as u8;
                    let argc_us = argc as u16;
                    let func = sp.saturating_sub(argc_us + 1);
                    let args = func + 1;
                    self.use_reg(sp.saturating_sub(1));
                    self.emit(RegOp::TailCall { func, args, argc });
                    sp = 0;
                }
                Opcode::Return => {
                    let src = sp.saturating_sub(1);
                    self.use_reg(src);
                    self.emit(RegOp::Ret { src });
                    sp = 0;
                }
                Opcode::GetProperty => {
                    let name = ins.operand.unwrap_or(0);
                    let obj = sp.saturating_sub(1);
                    let dst = obj;
                    self.use_reg(dst);
                    self.emit(RegOp::GetProp { dst, obj, name });
                }
                Opcode::SetProperty => {
                    let name = ins.operand.unwrap_or(0);
                    let src = sp.saturating_sub(1);
                    let obj = sp.saturating_sub(2);
                    self.use_reg(src.max(obj));
                    self.emit(RegOp::SetProp { obj, name, src });
                    // SetProperty leaves the object on the stack in the stack IR
                    // (value becomes the new stack top after consuming both).
                    // Stack IR: [obj, val] → [obj]. Our src/obj consumed val;
                    // keep obj as top: sp = obj + 1.
                    sp = obj + 1;
                }
                Opcode::Invoke => {
                    let packed = ins.operand.unwrap_or(0);
                    let argc = (packed >> 16) as u8;
                    let name = packed & 0xFFFF;
                    let argc_us = argc as u16;
                    let obj = sp.saturating_sub(argc_us + 1);
                    let args = obj + 1;
                    let dst = obj;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::Invoke {
                        dst,
                        obj,
                        name,
                        args,
                        argc,
                    });
                    sp = obj + 1;
                }
                Opcode::BuildArray => {
                    let count = ins.operand.unwrap_or(0) as u16;
                    let start = sp.saturating_sub(count);
                    let dst = start;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::BuildArr { dst, start, count });
                    sp = start + 1;
                }
                Opcode::BuildMap => {
                    let count = ins.operand.unwrap_or(0) as u16; // pairs
                    let slots = count * 2;
                    let start = sp.saturating_sub(slots);
                    let dst = start;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::BuildMap {
                        dst,
                        start,
                        count: slots,
                    });
                    sp = start + 1;
                }
                Opcode::IndexGet => {
                    let idx = sp.saturating_sub(1);
                    let obj = sp.saturating_sub(2);
                    let dst = obj;
                    self.use_reg(idx.max(dst));
                    self.emit(RegOp::IndexGet { dst, obj, idx });
                    sp = obj + 1;
                }
                Opcode::IndexSet => {
                    let src = sp.saturating_sub(1);
                    let idx = sp.saturating_sub(2);
                    let obj = sp.saturating_sub(3);
                    self.use_reg(src.max(obj));
                    self.emit(RegOp::IndexSet { obj, idx, src });
                    // Stack IR: [obj, idx, val] → [] after IndexSet+Pop in compiler.
                    sp = obj;
                }
                Opcode::CreateClass => {
                    let k = ins.operand.unwrap_or(0);
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::CreateClass { dst, k });
                    sp += 1;
                }
                Opcode::CreateInstance => {
                    // Stack: [class, args...] → [instance]
                    let argc = 0u8; // class call uses Call path in stack IR
                    let _ = argc;
                    let class = sp.saturating_sub(1);
                    let dst = class;
                    self.use_reg(dst);
                    self.emit(RegOp::CreateInstance {
                        dst,
                        class,
                        args: class,
                        argc: 0,
                    });
                }
                Opcode::GetMethod => {
                    let name = ins.operand.unwrap_or(0);
                    let obj = sp.saturating_sub(1);
                    let dst = obj;
                    self.use_reg(dst);
                    self.emit(RegOp::GetProp { dst, obj, name });
                }
                Opcode::MatchPattern => {
                    self.emit(RegOp::Nop);
                }
                Opcode::MakeClosure => {
                    let count = ins.operand.unwrap_or(0) as u8;
                    let count_us = count as u16;
                    let upvals = sp.saturating_sub(count_us + 1);
                    let func = sp.saturating_sub(count_us + 1);
                    // stack: [func, uv…]
                    let func_r = sp - count_us - 1;
                    let upvals_r = func_r + 1;
                    let dst = func_r;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::MakeClosure {
                        dst,
                        func: func_r,
                        upvals: upvals_r,
                        count,
                    });
                    sp = func_r + 1;
                    let _ = (upvals, func);
                }
                Opcode::Throw => {
                    let src = sp.saturating_sub(1);
                    self.use_reg(src);
                    self.emit(RegOp::Ret { src });
                    sp = 0;
                }
                Opcode::TryBegin | Opcode::TryEnd | Opcode::Nop => {
                    self.emit(RegOp::Nop);
                }
                Opcode::WrapOk | Opcode::WrapErr | Opcode::WrapSome => {
                    let kind = match ins.opcode {
                        Opcode::WrapOk => WrapKind::Ok,
                        Opcode::WrapErr => WrapKind::Err,
                        _ => WrapKind::Some,
                    };
                    let src = sp.saturating_sub(1);
                    let dst = src;
                    self.use_reg(dst);
                    self.emit(RegOp::Wrap { kind, dst, src });
                }
                Opcode::UnwrapOk | Opcode::UnwrapErr | Opcode::UnwrapSome => {
                    let kind = match ins.opcode {
                        Opcode::UnwrapOk => WrapKind::Ok,
                        Opcode::UnwrapErr => WrapKind::Err,
                        _ => WrapKind::Some,
                    };
                    // Stack: pop v → push inner, true  OR  push false
                    let src = sp.saturating_sub(1);
                    let dst = src;
                    let flag = src + 1;
                    self.use_reg(flag);
                    self.emit(RegOp::Unwrap {
                        kind,
                        dst,
                        flag,
                        src,
                    });
                    // On success stack is [inner, bool] (2), on failure [bool] (1).
                    // Conservative: assume 2 (success path); translator is used
                    // with the register VM that pushes both and keeps height by
                    // using flag as the bool top. Model as height += 1.
                    sp = src + 2;
                }
                Opcode::Try => {
                    let src = sp.saturating_sub(1);
                    let dst = src;
                    self.use_reg(dst);
                    self.emit(RegOp::TryQ { dst, src });
                }
                Opcode::Argc => {
                    let dst = sp;
                    self.use_reg(dst);
                    self.emit(RegOp::Argc { dst });
                    sp += 1;
                }
                Opcode::Halt => {
                    let src = sp.saturating_sub(1);
                    self.use_reg(src);
                    self.emit(RegOp::Halt { src });
                    sp = 0;
                }
                Opcode::CallDirect => {
                    let packed = ins.operand.unwrap_or(0);
                    let chunk = packed >> 16;
                    let argc = (packed & 0xFFFF) as u8;
                    let argc_us = argc as u16;
                    // Stack: [args…] only (no func slot) in CallDirect
                    let args = sp.saturating_sub(argc_us);
                    let dst = args;
                    self.use_reg(sp.saturating_sub(1).max(dst));
                    self.emit(RegOp::CallChunk {
                        dst,
                        chunk,
                        args,
                        argc,
                    });
                    sp = args + 1;
                }
            }
        }
        // Patch jumps
        let mut code = std::mem::take(&mut self.out.code);
        let ip_map = self.ip_map.clone();
        let jumps = self.jumps.clone();
        for (stack_target, site) in jumps {
            let reg_target = if stack_target < ip_map.len() {
                ip_map[stack_target]
            } else {
                code.len() as u32
            };
            match &mut code[site] {
                RegOp::Jump { target } => *target = reg_target,
                RegOp::JumpIfFalse { target, .. } => *target = reg_target,
                RegOp::JumpIfTrue { target, .. } => *target = reg_target,
                _ => {}
            }
        }
        self.out.code = code;
        self.out
    }

    /// Declare parameter counts (for CallChunk arity checks).
    pub fn with_arity(self, arity: usize, required: usize) -> RegChunk {
        let mut c = self.translate();
        c.arity = arity;
        c.required = required;
        c
    }

    pub fn translate_named(self) -> RegChunk {
        self.translate()
    }
}

fn bin_kind(op: Opcode) -> BinKind {
    match op {
        Opcode::Add | Opcode::AddImm => BinKind::Add,
        Opcode::Sub | Opcode::SubImm => BinKind::Sub,
        Opcode::Mul | Opcode::MulImm => BinKind::Mul,
        Opcode::Div => BinKind::Div,
        Opcode::Mod => BinKind::Mod,
        Opcode::Pow => BinKind::Pow,
        Opcode::BitAnd => BinKind::BitAnd,
        Opcode::BitOr => BinKind::BitOr,
        Opcode::BitXor => BinKind::BitXor,
        Opcode::Shl => BinKind::Shl,
        Opcode::Shr => BinKind::Shr,
        Opcode::And => BinKind::And,
        Opcode::Or => BinKind::Or,
        _ => BinKind::Add,
    }
}

fn bin_kind_imm(op: Opcode) -> BinKind {
    match op {
        Opcode::SubImm => BinKind::Sub,
        Opcode::MulImm => BinKind::Mul,
        _ => BinKind::Add,
    }
}

fn cmp_kind(op: Opcode) -> CmpKind {
    match op {
        Opcode::Eq | Opcode::EqImm => CmpKind::Eq,
        Opcode::Ne | Opcode::NeImm => CmpKind::Ne,
        Opcode::Lt | Opcode::LtImm => CmpKind::Lt,
        Opcode::Le | Opcode::LeImm => CmpKind::Le,
        Opcode::Gt | Opcode::GtImm => CmpKind::Gt,
        Opcode::Ge | Opcode::GeImm => CmpKind::Ge,
        _ => CmpKind::Eq,
    }
}

fn cmp_kind_imm(op: Opcode) -> CmpKind {
    cmp_kind(op)
}

fn un_kind(op: Opcode) -> UnKind {
    match op {
        Opcode::Not => UnKind::Not,
        Opcode::BitNot => UnKind::BitNot,
        _ => UnKind::Neg,
    }
}

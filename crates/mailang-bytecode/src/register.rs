//! Register-based bytecode and a stack→register translator.
//!
//! The classic compiler emits stack bytecode. A real **register VM** executes
//! three-address instructions (`Add dst, lhs, rhs`) with a flat register file.
//! This module defines that IR and a translator that lowers stack bytecode to
//! register bytecode by simulating the operand stack and assigning each stack
//! slot a virtual register.

use crate::{Opcode, Value};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

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
/// Strategy: simulate the operand stack with **dataflow stack heights** so
/// control-flow joins (match arms, if/else, Unwrap success vs fail) get the
/// correct register indices. Registers are the frame's contiguous stack slots
/// (locals occupy `0..start_sp`, expression temps grow above).
pub struct StackToRegister<'a> {
    src_code: &'a [crate::Instruction],
    #[allow(dead_code)]
    src_consts: &'a [Value],
    out: RegChunk,
    /// Maps stack IP → register IP (for jump patching).
    ip_map: Vec<u32>,
    /// Pending (stack_target, reg_site) jumps to patch.
    jumps: Vec<(usize, usize)>,
    /// Initial stack height (= parameter / `this` slot count).
    start_sp: u16,
    /// Height before each source instruction (dataflow).
    heights: Vec<u16>,
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
            start_sp: 0,
            heights: Vec::new(),
        }
    }

    /// Set the frame's initial stack height (parameter count, including `this`).
    pub fn with_start_sp(mut self, start_sp: u16) -> Self {
        self.start_sp = start_sp;
        self
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
        let start_sp = self.start_sp;
        self.heights = compute_heights(self.src_code, start_sp);
        self.out.n_regs = self.out.n_regs.max(start_sp);
        for (sip, ins) in self.src_code.iter().enumerate() {
            self.ip_map.push(self.out.code.len() as u32);
            let mut sp: u16 = self.heights[sip];
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
                    // Stack VM pushes Bool(true); keep the same effect.
                    let dst = sp;
                    self.use_reg(dst);
                    let k = {
                        // Reuse/add a true constant.
                        let v = Value::Bool(true);
                        if let Some(i) = self.out.constants.iter().position(|c| c == &v) {
                            i as u32
                        } else {
                            self.out.constants.push(v);
                            (self.out.constants.len() - 1) as u32
                        }
                    };
                    self.emit(RegOp::LoadConst { dst, k });
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
                    // Success: [inner, true] (dst, flag). Fail: [false] at dst,
                    // and flag is also set false so a following JumpIfFalse
                    // (which peeks the success-layout top) still branches.
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
                    // Dataflow already models out-height as +1 (success layout).
                    let _ = sp;
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
        let mut code = core::mem::take(&mut self.out.code);
        let ip_map = self.ip_map.clone();
        let jumps = self.jumps.clone();
        for (stack_target, site) in jumps {
            let reg_target = if stack_target < ip_map.len() {
                ip_map[stack_target]
            } else {
                code.len() as u32
            };
            match &mut code[site] {
                RegOp::Jump { ref mut target } => *target = reg_target,
                RegOp::JumpIfFalse { ref mut target, .. } => *target = reg_target,
                RegOp::JumpIfTrue { ref mut target, .. } => *target = reg_target,
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

fn is_unwrap(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::UnwrapOk | Opcode::UnwrapErr | Opcode::UnwrapSome
    )
}

/// Stack height after `op` given height `h` before it (success layout for Unwrap).
fn stack_out(op: Opcode, h: u16, operand: Option<u32>) -> u16 {
    match op {
        Opcode::Push
        | Opcode::Dup
        | Opcode::LoadLocal
        | Opcode::LoadGlobal
        | Opcode::LoadUpvalue
        | Opcode::Argc
        | Opcode::CreateClass
        | Opcode::CreateInstance
        | Opcode::GetMethod
        | Opcode::MatchPattern => h.saturating_add(1),
        Opcode::Pop
        | Opcode::StoreGlobal
        | Opcode::StoreUpvalue
        | Opcode::Throw => h.saturating_sub(1),
        Opcode::StoreLocal => {
            let idx = operand.unwrap_or(0) as u16;
            let after = h.saturating_sub(1);
            if idx >= after {
                idx + 1
            } else {
                after
            }
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
        | Opcode::Or
        | Opcode::Eq
        | Opcode::Ne
        | Opcode::Lt
        | Opcode::Le
        | Opcode::Gt
        | Opcode::Ge
        | Opcode::IndexGet => h.saturating_sub(1),
        Opcode::IndexSet => h.saturating_sub(2),
        Opcode::Call => {
            let argc = operand.unwrap_or(0) as u16;
            // pop func + argc, push result
            h.saturating_sub(argc + 1).saturating_add(1)
        }
        Opcode::CallDirect => {
            let argc = operand.unwrap_or(0) as u16;
            h.saturating_sub(argc).saturating_add(1)
        }
        Opcode::Invoke => {
            let packed = operand.unwrap_or(0);
            let argc = (packed >> 16) as u16;
            h.saturating_sub(argc + 1).saturating_add(1)
        }
        Opcode::BuildArray => {
            let count = operand.unwrap_or(0) as u16;
            h.saturating_sub(count).saturating_add(1)
        }
        Opcode::BuildMap => {
            let pairs = operand.unwrap_or(0) as u16;
            h.saturating_sub(pairs * 2).saturating_add(1)
        }
        Opcode::SetProperty => h.saturating_sub(1),
        // pop object, push property value → net 0
        Opcode::GetProperty => h,
        Opcode::MakeClosure => {
            let count = operand.unwrap_or(0) as u16;
            h.saturating_sub(count + 1).saturating_add(1)
        }
        Opcode::WrapOk | Opcode::WrapErr | Opcode::WrapSome => h,
        // Success layout: pop 1, push 2 → +1. Fail path is handled by the
        // JumpIfFalse taken-edge rule (one less value).
        Opcode::UnwrapOk | Opcode::UnwrapErr | Opcode::UnwrapSome => h.saturating_add(1),
        Opcode::Try => h,
        Opcode::AddImm
        | Opcode::SubImm
        | Opcode::MulImm
        | Opcode::EqImm
        | Opcode::NeImm
        | Opcode::LtImm
        | Opcode::LeImm
        | Opcode::GtImm
        | Opcode::GeImm
        | Opcode::Neg
        | Opcode::Not
        | Opcode::BitNot
        | Opcode::Jump
        | Opcode::JumpIfFalse
        | Opcode::JumpIfTrue
        | Opcode::TryBegin
        | Opcode::TryEnd
        | Opcode::Nop => h,
        Opcode::Return | Opcode::Halt | Opcode::TailCall => h,
    }
}

/// Dataflow stack height before each instruction.
fn compute_heights(code: &[crate::Instruction], start_sp: u16) -> Vec<u16> {
    let n = code.len();
    if n == 0 {
        return Vec::new();
    }
    let mut height = vec![u16::MAX; n];
    let mut work = alloc::collections::VecDeque::new();
    height[0] = start_sp;
    work.push_back(0usize);

    while let Some(ip) = work.pop_front() {
        if ip >= n {
            continue;
        }
        let h = height[ip];
        let ins = &code[ip];
        let out = stack_out(ins.opcode, h, ins.operand);
        match ins.opcode {
            Opcode::Jump => {
                let t = ins.operand.unwrap_or(0) as usize;
                if t < n {
                    let cur = height[t];
                    if cur == u16::MAX {
                        height[t] = out;
                        work.push_back(t);
                    } else if cur != out {
                        let m = cur.max(out);
                        if m != cur {
                            height[t] = m;
                            work.push_back(t);
                        }
                    }
                }
            }
            Opcode::JumpIfFalse | Opcode::JumpIfTrue => {
                // Peeks top; both edges keep `out` unless the previous op was
                // Unwrap, whose fail path has one fewer value.
                let taken = if ip > 0 && is_unwrap(code[ip - 1].opcode) {
                    out.saturating_sub(1)
                } else {
                    out
                };
                // fallthrough
                let ft = ip + 1;
                if ft < n {
                    let cur = height[ft];
                    if cur == u16::MAX {
                        height[ft] = out;
                        work.push_back(ft);
                    } else if cur != out {
                        let m = cur.max(out);
                        if m != cur {
                            height[ft] = m;
                            work.push_back(ft);
                        }
                    }
                }
                let t = ins.operand.unwrap_or(0) as usize;
                if t < n {
                    let cur = height[t];
                    if cur == u16::MAX {
                        height[t] = taken;
                        work.push_back(t);
                    } else if cur != taken {
                        let m = cur.max(taken);
                        if m != cur {
                            height[t] = m;
                            work.push_back(t);
                        }
                    }
                }
            }
            Opcode::Return | Opcode::Halt | Opcode::TailCall | Opcode::Throw => {
                if ip + 1 < n {
                    let cur = height[ip + 1];
                    if cur == u16::MAX {
                        height[ip + 1] = out;
                        work.push_back(ip + 1);
                    }
                }
            }
            _ => {
                let ft = ip + 1;
                if ft < n {
                    let cur = height[ft];
                    if cur == u16::MAX {
                        height[ft] = out;
                        work.push_back(ft);
                    } else if cur != out {
                        let m = cur.max(out);
                        if m != cur {
                            height[ft] = m;
                            work.push_back(ft);
                        }
                    }
                }
            }
        }
    }

    for h in height.iter_mut() {
        if *h == u16::MAX {
            *h = start_sp;
        }
    }
    height
}

//! Human-readable dumps of compiled bytecode (`mailang disasm`).
//!
//! Two forms can be printed: the stack instruction stream the compiler emits
//! (what the default VM runs) and the three-address register lowering that
//! `--vm=register` executes. Operands are decoded exactly like the VMs decode
//! them — constant-pool indices with their value, global slots with their
//! name, jump targets as instruction indices, local slots as register numbers
//! — so a dump can be read without counting bytes.
//!
//! This is the view that the Phase N work needed and did not have: `match`
//! delivering its value through the wrong protocol and the register backend's
//! dataflow heights are both visible in a few adjacent lines of a dump.

use crate::register::{parameter_slots, RegOp, StackToRegister};
use crate::{Bytecode, Chunk, Instruction, Opcode, Value};
use alloc::format;
use alloc::string::{String, ToString};
use core::fmt::Write;

/// Stack bytecode as the compiler emitted it. `only` limits the dump to one
/// chunk index (all chunks when `None`).
pub fn disassemble_stack(bytecode: &Bytecode, only: Option<usize>) -> String {
    let mut out = String::new();
    for (i, ch) in bytecode.chunks.iter().enumerate() {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let _ = writeln!(
            out,
            "== chunk {} ({}) == {} instructions, {} constants, params {}",
            i,
            ch.name,
            ch.instructions.len(),
            ch.constants.len(),
            parameter_slots(bytecode, i),
        );
        for (ip, ins) in ch.instructions.iter().enumerate() {
            let _ = writeln!(
                out,
                "{:5}  {:<14} {:<10} {}",
                ip,
                opcode_name(ins.opcode),
                operand_text(ins.operand),
                stack_note(bytecode, ch, ins),
            );
        }
        out.push('\n');
    }
    out
}

/// Register (three-address) IR for the same bytecode, lowered exactly like
/// `RegisterVm::new` lowers it (same `parameter_slots` starting height).
pub fn disassemble_register(bytecode: &Bytecode, only: Option<usize>) -> String {
    let mut out = String::new();
    for (i, ch) in bytecode.chunks.iter().enumerate() {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let start_sp = parameter_slots(bytecode, i);
        let reg = StackToRegister::new(&ch.name, &ch.instructions, &ch.constants)
            .with_start_sp(start_sp)
            .translate();
        let _ = writeln!(
            out,
            "== chunk {} ({}) == {} regops, {} regs, start_sp {} (params 0..{})",
            i,
            reg.name,
            reg.code.len(),
            reg.n_regs,
            reg.start_sp,
            reg.start_sp,
        );
        for (rip, op) in reg.code.iter().enumerate() {
            let _ = writeln!(
                out,
                "{:5}  {:<46} {}",
                rip,
                format!("{:?}", op),
                reg_note(&reg.constants, op),
            );
        }
        out.push('\n');
    }
    out
}

fn opcode_name(op: Opcode) -> String {
    format!("{:?}", op)
}

fn operand_text(operand: Option<u32>) -> String {
    match operand {
        Some(o) => format!("#{}", o),
        None => String::new(),
    }
}

/// Decoded meaning of a stack instruction's operand.
fn stack_note(bytecode: &Bytecode, ch: &Chunk, ins: &Instruction) -> String {
    let k = ins.operand.unwrap_or(0);
    match ins.opcode {
        Opcode::Push => match ch.constants.get(k as usize) {
            Some(v) => format!("; {}", value_text(v)),
            None => String::new(),
        },
        Opcode::GetProperty | Opcode::SetProperty => match ch.constants.get(k as usize) {
            Some(Value::Str(name)) => format!("; \"{}\"", name),
            Some(v) => format!("; {}", value_text(v)),
            None => String::new(),
        },
        Opcode::LoadGlobal | Opcode::StoreGlobal => match bytecode.global_names.get(k as usize) {
            Some(name) => format!("; global {}", name),
            None => String::new(),
        },
        Opcode::Jump
        | Opcode::JumpIfFalse
        | Opcode::JumpIfTrue
        | Opcode::TryBegin
        | Opcode::TryEnd => format!("; -> {}", k),
        Opcode::LoadLocal | Opcode::StoreLocal => format!("; local r{}", k),
        Opcode::Slide => format!("; keep top, drop {} below", k),
        Opcode::LoadUpvalue | Opcode::StoreUpvalue => format!("; upvalue {}", k),
        Opcode::Call | Opcode::TailCall | Opcode::BuildArray | Opcode::BuildMap => {
            format!("; {} operands", k)
        }
        Opcode::CallDirect => {
            // Packed as (argc << 16) | const index of the callee name.
            let argc = k >> 16;
            let callee = k & 0xFFFF;
            match ch.constants.get(callee as usize) {
                Some(Value::Str(name)) => format!("; {}({} args, direct)", name, argc),
                _ => format!("; {} args, direct", argc),
            }
        }
        _ => String::new(),
    }
}

/// Decoded meaning of a register instruction.
fn reg_note(consts: &[Value], op: &RegOp) -> String {
    match op {
        RegOp::LoadConst { k, .. } => match consts.get(*k as usize) {
            Some(v) => format!("; {}", value_text(v)),
            None => String::new(),
        },
        RegOp::LoadGlobal { slot, .. } | RegOp::StoreGlobal { slot, .. } => {
            format!("; global #{}", slot)
        }
        RegOp::GetProp { name, .. } | RegOp::SetProp { name, .. } | RegOp::Invoke { name, .. } => {
            match consts.get(*name as usize) {
                Some(Value::Str(s)) => format!("; \"{}\"", s),
                _ => String::new(),
            }
        }
        RegOp::Jump { target } => format!("; -> {}", target),
        RegOp::JumpIfFalse { target, .. } | RegOp::JumpIfTrue { target, .. } => {
            format!("; -> {}", target)
        }
        RegOp::CreateClass { k, .. } => match consts.get(*k as usize) {
            Some(v) => format!("; {}", value_text(v)),
            None => String::new(),
        },
        _ => String::new(),
    }
}

/// Short rendering of a value. Heap nodes print their shape, not their
/// contents, so a self-referential constant cannot recurse forever.
fn value_text(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => format!("\"{}\"", s),
        Value::Char(c) => format!("'{}'", c),
        Value::Array(a) => format!("<array len={}>", a.borrow().len()),
        Value::Tuple(t) => format!("<tuple len={}>", t.borrow().len()),
        Value::Map(m) => format!("<map len={}>", m.borrow().len()),
        Value::Function(f) => format!("<fn {}>", f.name),
        Value::Closure(_) => "<closure>".to_string(),
        Value::Class(c) => format!("<class {}>", c.name),
        Value::Instance { class_index, .. } => format!("<instance of class {}>", class_index),
        Value::Ok(_) => "Ok(..)".to_string(),
        Value::Err(_) => "Err(..)".to_string(),
        Value::Some(_) => "Some(..)".to_string(),
        Value::Builtin { name, .. } => format!("<builtin {}>", name),
    }
}

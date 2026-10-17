//! `gc_stats()` for MaìLang scripts.
//!
//! A builtin is a plain `fn` pointer with no access to the interpreter, so the
//! two VMs publish their census counters into a shared cell after every
//! collection and this module reads them back. The name is bound as a *host*
//! function (host functions take precedence over builtins), which also gives
//! `gc_stats` a global slot through `register_host_fn`.

use crate::HostFn;
use mailang_bytecode::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Census counters published by a VM: `(tracked, collections, freed_total)`.
pub type GcCounters = Rc<Cell<(usize, usize, usize)>>;

/// A zeroed counter cell; the owning VM keeps it in sync.
pub fn new_counters() -> GcCounters {
    Rc::new(Cell::new((0, 0, 0)))
}

/// The `gc_stats()` host function. Returns a map so a script reads the fields
/// by name: `gc_stats()["tracked"]`.
pub fn gc_stats_host_fn(counters: GcCounters) -> HostFn {
    Rc::new(move |_args| {
        let (tracked, collections, freed) = counters.get();
        Ok(Value::Map(Rc::new(RefCell::new(vec![
            (Value::Str("tracked".into()), Value::Int(tracked as i64)),
            (
                Value::Str("collections".into()),
                Value::Int(collections as i64),
            ),
            (Value::Str("freed".into()), Value::Int(freed as i64)),
        ]))))
    })
}

//! Mark-sweep garbage collector for MaìLang heap nodes.
//!
//! Values share heap nodes via `Rc<RefCell<…>>`. A cycle never hits refcount
//! zero. This module maintains a **heap census** of tracked nodes and runs a
//! real mark-sweep:
//!
//! 1. **Mark** — walk the object graph from VM roots (stack / globals / upvalues)
//!    and mark every reachable node.
//! 2. **Sweep** — every tracked node that is unmarked is garbage. Clear its
//!    child edges (breaks cycles) and drop the collector’s `Rc` handle so the
//!    cycle can actually deallocate.
//!
//! Automatic collection runs when the allocation counter crosses a threshold
//! (the threshold grows after each collect, like a classic generational hint).

use mailang_bytecode::Value;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub mod cycle;
pub use cycle::{collect_cycles, collect_cycles_value};

/// Identity of a tracked heap node (Array / Map / Tuple / Instance / …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GcId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcState {
    White,
    Gray,
    Black,
}

fn node_id(v: &Value) -> Option<GcId> {
    match v {
        Value::Array(a) => Some(GcId(Rc::as_ptr(a) as *const u8 as usize)),
        Value::Map(m) => Some(GcId(Rc::as_ptr(m) as *const u8 as usize)),
        Value::Tuple(t) => Some(GcId(Rc::as_ptr(t) as *const u8 as usize)),
        Value::Instance { fields, .. } => Some(GcId(Rc::as_ptr(fields) as *const u8 as usize)),
        Value::Function(f) => Some(GcId(Rc::as_ptr(f) as *const u8 as usize)),
        Value::Closure(c) => Some(GcId(Rc::as_ptr(c) as *const u8 as usize)),
        Value::Class(c) => Some(GcId(Rc::as_ptr(c) as *const u8 as usize)),
        _ => None,
    }
}

/// Immediate child values of a heap node (for tracing).
pub fn children(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.borrow().clone(),
        Value::Map(m) => {
            let mut out = Vec::new();
            for (k, val) in m.borrow().iter() {
                out.push(k.clone());
                out.push(val.clone());
            }
            out
        }
        Value::Tuple(t) => t.borrow().clone(),
        Value::Instance { fields, .. } => fields.borrow().iter().map(|(_, v)| v.clone()).collect(),
        Value::Class(c) => c.properties.iter().map(|(_, v)| v.clone()).collect(),
        Value::Ok(i) | Value::Err(i) | Value::Some(i) => vec![(**i).clone()],
        _ => Vec::new(),
    }
}

/// Clear all child edges of a heap node (breaks cycles so Rc can drop).
fn clear_children(v: &Value) {
    match v {
        Value::Array(a) => a.borrow_mut().clear(),
        Value::Map(m) => m.borrow_mut().clear(),
        Value::Tuple(t) => t.borrow_mut().clear(),
        Value::Instance { fields, .. } => fields.borrow_mut().clear(),
        _ => {}
    }
}

/// A tracked heap node kept alive by the collector’s own `Rc` handle.
struct Tracked {
    #[allow(dead_code)]
    value: Value,
    #[allow(dead_code)]
    state: GcState,
}

/// Mark-sweep heap. The VM registers heap nodes as they are allocated;
/// `collect` traces from roots and frees unreachable cyclic garbage.
pub struct MarkSweepHeap {
    nodes: HashMap<GcId, Tracked>,
    /// Allocation counter since last collect.
    allocations: usize,
    /// Collect when `allocations` reaches this.
    threshold: usize,
    /// Stats: total objects freed.
    pub freed_total: usize,
    /// Stats: total collections.
    pub collections: usize,
}

impl MarkSweepHeap {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            allocations: 0,
            threshold: 256,
            freed_total: 0,
            collections: 0,
        }
    }

    /// Track a newly allocated heap node. Returns its `GcId`.
    pub fn track(&mut self, value: &Value) -> Option<GcId> {
        let id = node_id(value)?;
        self.allocations += 1;
        self.nodes.entry(id).or_insert_with(|| Tracked {
            value: value.clone(),
            state: GcState::White,
        });
        Some(id)
    }

    pub fn tracked_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn should_collect(&self) -> bool {
        self.allocations >= self.threshold
    }

    /// Mark-sweep from `roots`. Returns the number of nodes freed.
    pub fn collect(&mut self, roots: &[Value]) -> usize {
        self.collections += 1;
        self.allocations = 0;
        // Grow threshold (classic heap-growth heuristic).
        self.threshold = (self.threshold as f64 * 1.5) as usize;

        // --- Mark ---
        let mut marked: HashSet<GcId> = HashSet::new();
        let mut stack: Vec<Value> = roots.to_vec();
        while let Some(v) = stack.pop() {
            let Some(id) = node_id(&v) else {
                // Scalars / boxes: still trace into Ok/Err/Some.
                for c in children(&v) {
                    stack.push(c);
                }
                continue;
            };
            if !marked.insert(id) {
                continue;
            }
            for c in children(&v) {
                stack.push(c);
            }
        }

        // --- Sweep ---
        let dead: Vec<GcId> = self
            .nodes
            .iter()
            .filter(|(id, _)| !marked.contains(id))
            .map(|(id, _)| *id)
            .collect();
        let mut freed = 0usize;
        for id in dead {
            if let Some(tracked) = self.nodes.remove(&id) {
                // Cut interior edges so the Rc cycle can drop.
                clear_children(&tracked.value);
                drop(tracked.value);
                freed += 1;
            }
        }
        self.freed_total += freed;
        freed
    }
}

impl Default for MarkSweepHeap {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: one-shot mark-sweep without a persistent heap (tests / tools).
pub fn mark_sweep_once(roots: &[Value]) -> usize {
    let mut heap = MarkSweepHeap::new();
    let mut stack: Vec<Value> = roots.to_vec();
    let mut seen: HashSet<GcId> = HashSet::new();
    while let Some(v) = stack.pop() {
        if let Some(id) = node_id(&v) {
            if !seen.insert(id) {
                continue;
            }
            heap.track(&v);
        }
        for c in children(&v) {
            stack.push(c);
        }
    }
    heap.collect(roots)
}

// Keep the old `Gc` name as a thin alias so existing call sites compile.
#[derive(Debug)]
pub struct GcObject<T> {
    pub value: T,
    pub state: GcState,
    pub ref_count: usize,
    pub marked: bool,
}

impl<T> GcObject<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            state: GcState::White,
            ref_count: 1,
            marked: false,
        }
    }
    pub fn inc_ref(&mut self) {
        self.ref_count += 1;
    }
    pub fn dec_ref(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
}

/// Legacy handle type — the mark-sweep heap is the real collector now.
pub struct Gc {
    heap: MarkSweepHeap,
}

impl Gc {
    pub fn new() -> Self {
        Self {
            heap: MarkSweepHeap::new(),
        }
    }
    pub fn heap(&mut self) -> &mut MarkSweepHeap {
        &mut self.heap
    }
    pub fn object_count(&self) -> usize {
        self.heap.tracked_count()
    }
}

impl Default for Gc {
    fn default() -> Self {
        Self::new()
    }
}

// Silence unused import warnings for types used in docs/tests.
#[allow(unused_imports)]
use std::cell::RefCell as _RefCell;
#[allow(unused_imports)]
use std::rc::Rc as _Rc;

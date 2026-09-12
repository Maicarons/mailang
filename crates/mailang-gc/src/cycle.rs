//! Cycle detection / collection for `Rc` graphs.
//!
//! MaìLang values share heap nodes via `Rc` / `Rc<RefCell<…>>`. A cycle
//! (instance field pointing at itself, mutually referencing arrays, …) will
//! never hit refcount zero. This module traces from roots, finds back-edges
//! in the gray set, and **breaks** them by replacing the back-edge with
//! `Null` (the only sound in-place fix without a full mark-sweep heap).

use mailang_bytecode::Value;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Key identifying a shared heap node (Array / Map / Tuple / Instance / …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct NodeId(usize);

fn node_id(v: &Value) -> Option<NodeId> {
    match v {
        Value::Array(a) => Some(NodeId(Rc::as_ptr(a) as *const u8 as usize)),
        Value::Map(m) => Some(NodeId(Rc::as_ptr(m) as *const u8 as usize)),
        Value::Tuple(t) => Some(NodeId(Rc::as_ptr(t) as *const u8 as usize)),
        Value::Instance { fields, .. } => {
            Some(NodeId(Rc::as_ptr(fields) as *const u8 as usize))
        }
        Value::Function(f) => Some(NodeId(Rc::as_ptr(f) as *const u8 as usize)),
        Value::Closure(c) => Some(NodeId(Rc::as_ptr(c) as *const u8 as usize)),
        Value::Class(c) => Some(NodeId(Rc::as_ptr(c) as *const u8 as usize)),
        _ => None,
    }
}

fn children(v: &Value) -> Vec<Value> {
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
        Value::Class(c) => c
            .properties
            .iter()
            .map(|(_, v)| v.clone())
            .collect(),
        Value::Ok(i) | Value::Err(i) | Value::Some(i) => vec![(**i).clone()],
        _ => Vec::new(),
    }
}

/// Break cyclic back-edges reachable from `roots`.
/// Returns the number of edges cut.
pub fn collect_cycles(roots: &mut [Value]) -> usize {
    let mut color: HashMap<NodeId, u8> = HashMap::new();
    let mut on_path: HashSet<NodeId> = HashSet::new();
    let mut broken = 0usize;
    for root in roots.iter_mut() {
        broken += dfs_cut(root, &mut color, &mut on_path);
    }
    broken
}

fn dfs_cut(value: &mut Value, color: &mut HashMap<NodeId, u8>, on_path: &mut HashSet<NodeId>) -> usize {
    let mut broken = 0;
    let Some(id) = node_id(value) else { return 0 };
    if on_path.contains(&id) {
        return 0; // caller should have cut
    }
    if color.get(&id) == Some(&2) {
        return 0;
    }
    color.insert(id, 1);
    on_path.insert(id);

    // Walk children; for shared RefCell containers, check each child for cycle.
    match value {
        Value::Array(a) => {
            let mut arr = a.borrow_mut();
            for item in arr.iter_mut() {
                if let Some(cid) = node_id(item) {
                    if on_path.contains(&cid) {
                        *item = Value::Null;
                        broken += 1;
                    } else {
                        broken += dfs_cut(item, color, on_path);
                    }
                }
            }
        }
        Value::Map(m) => {
            let mut map = m.borrow_mut();
            for (_, val) in map.iter_mut() {
                if let Some(cid) = node_id(val) {
                    if on_path.contains(&cid) {
                        *val = Value::Null;
                        broken += 1;
                    } else {
                        broken += dfs_cut(val, color, on_path);
                    }
                }
            }
        }
        Value::Tuple(t) => {
            let mut tup = t.borrow_mut();
            for item in tup.iter_mut() {
                if let Some(cid) = node_id(item) {
                    if on_path.contains(&cid) {
                        *item = Value::Null;
                        broken += 1;
                    } else {
                        broken += dfs_cut(item, color, on_path);
                    }
                }
            }
        }
        Value::Instance { fields, .. } => {
            let mut fields = fields.borrow_mut();
            for (_, val) in fields.iter_mut() {
                if let Some(cid) = node_id(val) {
                    if on_path.contains(&cid) {
                        *val = Value::Null;
                        broken += 1;
                    } else {
                        broken += dfs_cut(val, color, on_path);
                    }
                }
            }
        }
        Value::Ok(inner) | Value::Err(inner) | Value::Some(inner) => {
            if let Some(cid) = node_id(inner) {
                if on_path.contains(&cid) {
                    **inner = Value::Null;
                    broken += 1;
                } else {
                    broken += dfs_cut(inner, color, on_path);
                }
            }
        }
        _ => {}
    }

    on_path.remove(&id);
    color.insert(id, 2);
    broken
}

/// Convenience: collect from a single root.
pub fn collect_cycles_value(v: &mut Value) -> usize {
    collect_cycles(std::slice::from_mut(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn detects_and_breaks_self_instance_cycle() {
        let fields = Rc::new(RefCell::new(vec![("next".to_string(), Value::Null)]));
        let inst = Value::Instance {
            class_index: 0,
            fields: fields.clone(),
        };
        // Point next at itself
        fields.borrow_mut()[0].1 = inst.clone();
        let mut root = inst;
        let broken = collect_cycles_value(&mut root);
        assert!(broken >= 1);
        // After collection, next should be Null
        if let Value::Instance { fields, .. } = &root {
            assert!(matches!(fields.borrow()[0].1, Value::Null));
        } else {
            panic!("expected instance");
        }
    }

    #[test]
    fn detects_array_cycle() {
        let a = Rc::new(RefCell::new(vec![Value::Int(1)]));
        let arr = Value::Array(a.clone());
        a.borrow_mut().push(arr.clone());
        let mut root = arr;
        let broken = collect_cycles_value(&mut root);
        assert!(broken >= 1);
    }
}

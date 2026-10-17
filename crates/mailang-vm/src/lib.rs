#![allow(clippy::type_complexity, clippy::too_many_arguments, dead_code)]
mod error;
mod gcstats;
mod register_vm;
mod vm;

pub use error::VmError;
pub use gcstats::{gc_stats_host_fn, new_counters, GcCounters};
pub use register_vm::RegisterVm;
pub use vm::{HostFn, Vm};

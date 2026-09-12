//! C ABI for embedding MaìLang.
//!
//! All `extern "C"` entry points are wrapped in `catch_unwind` so a Rust panic
//! cannot unwind across the FFI boundary.

use mailang_bytecode::Value;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

/// Opaque interpreter handle.
pub struct MailangInterpreter {
    inner: mailang_core::MailangInterpreter,
    last_error: Option<CString>,
}

#[repr(C)]
pub struct MailangResult {
    pub code: i32,
    pub output: *mut c_char,
}

/// Tagged value for host callbacks.
/// `tag`: 0=null, 1=bool, 2=int, 3=float, 4=str
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MailangValue {
    pub tag: i32,
    pub i: i64,
    pub f: f64,
    pub s: *const c_char,
}

impl MailangValue {
    pub fn null() -> Self {
        Self { tag: 0, i: 0, f: 0.0, s: ptr::null() }
    }

    pub fn from_i64(v: i64) -> Self {
        Self { tag: 2, i: v, f: 0.0, s: ptr::null() }
    }

    pub fn from_bool(v: bool) -> Self {
        Self { tag: 1, i: v as i64, f: 0.0, s: ptr::null() }
    }

    pub fn from_f64(v: f64) -> Self {
        Self { tag: 3, i: 0, f: v, s: ptr::null() }
    }
}

/// Host function callback.
/// Return 0 on success and write `*out`; non-zero on error (message via last_error).
pub type MailangHostFn = extern "C" fn(
    user_data: *mut c_void,
    argc: i32,
    argv: *const MailangValue,
    out: *mut MailangValue,
) -> i32;

/// Error codes returned by FFI entry points.
pub const MAILANG_OK: i32 = 0;
pub const MAILANG_ERR_NULL: i32 = -1;
pub const MAILANG_ERR_EVAL: i32 = -2;
pub const MAILANG_ERR_UTF8: i32 = -3;
pub const MAILANG_ERR_HOST: i32 = -4;
pub const MAILANG_ERR_PANIC: i32 = -99;

fn set_result_ok(result: &mut MailangResult, output: String) {
    let c = CString::new(output).unwrap_or_default();
    result.code = MAILANG_OK;
    result.output = c.into_raw();
}

fn set_result_err(result: &mut MailangResult, code: i32, msg: String) {
    let c = CString::new(msg).unwrap_or_default();
    result.code = code;
    result.output = c.into_raw();
}

fn value_to_ffi(v: &Value) -> MailangValue {
    match v {
        Value::Null => MailangValue::null(),
        Value::Bool(b) => MailangValue::from_bool(*b),
        Value::Int(n) => MailangValue::from_i64(*n),
        Value::Float(f) => MailangValue::from_f64(*f),
        Value::Str(s) => {
            // Leak a CString for the host; host should copy immediately.
            // We keep a side table is complex — instead convert to int fallback
            // is wrong. Use a thread-local last string buffer? For simplicity
            // return as int length if we cannot hold. Better: Box a CString and
            // store pointer; host is told values are valid until next eval.
            // Here we just create a CString and leak it (small, demo-grade).
            let c = CString::new(s.as_ref()).unwrap_or_default();
            MailangValue {
                tag: 4,
                i: 0,
                f: 0.0,
                s: c.into_raw() as *const c_char,
            }
        }
        other => {
            // Fallback: stringify to int 0
            let _ = other;
            MailangValue::null()
        }
    }
}

fn ffi_to_value(v: &MailangValue) -> Result<Value, String> {
    unsafe {
        match v.tag {
            0 => Ok(Value::Null),
            1 => Ok(Value::Bool(v.i != 0)),
            2 => Ok(Value::Int(v.i)),
            3 => Ok(Value::Float(v.f)),
            4 => {
                if v.s.is_null() {
                    return Ok(Value::Str("".into()));
                }
                let s = CStr::from_ptr(v.s)
                    .to_str()
                    .map_err(|_| "invalid utf8 in MailangValue.s".to_string())?;
                Ok(Value::Str(s.into()))
            }
            other => Err(format!("unknown MailangValue tag {}", other)),
        }
    }
}

#[no_mangle]
pub extern "C" fn mailang_create() -> *mut MailangInterpreter {
    catch_unwind(|| {
        let interp = Box::new(MailangInterpreter {
            inner: mailang_core::MailangInterpreter::new(),
            last_error: None,
        });
        Box::into_raw(interp)
    })
    .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn mailang_destroy(interp: *mut MailangInterpreter) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !interp.is_null() {
            drop(Box::from_raw(interp));
        }
    }));
}

#[no_mangle]
pub unsafe extern "C" fn mailang_eval(
    interp: *mut MailangInterpreter,
    code: *const c_char,
    result: *mut MailangResult,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || code.is_null() || result.is_null() {
            return MAILANG_ERR_NULL;
        }

        let interp = &mut *interp;
        let code_str = match CStr::from_ptr(code).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };

        match interp.inner.eval(code_str) {
            Ok(output) => {
                set_result_ok(&mut *result, output);
                MAILANG_OK
            }
            Err(e) => {
                let c_error = CString::new(e.clone()).unwrap_or_default();
                interp.last_error = Some(c_error);
                set_result_err(&mut *result, MAILANG_ERR_EVAL, e);
                MAILANG_ERR_EVAL
            }
        }
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

#[no_mangle]
pub unsafe extern "C" fn mailang_eval_file(
    interp: *mut MailangInterpreter,
    path: *const c_char,
    result: *mut MailangResult,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || path.is_null() || result.is_null() {
            return MAILANG_ERR_NULL;
        }

        let interp = &mut *interp;
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };

        match interp.inner.eval_file(path_str) {
            Ok(output) => {
                set_result_ok(&mut *result, output);
                MAILANG_OK
            }
            Err(e) => {
                let c_error = CString::new(e.clone()).unwrap_or_default();
                interp.last_error = Some(c_error);
                set_result_err(&mut *result, MAILANG_ERR_EVAL, e);
                MAILANG_ERR_EVAL
            }
        }
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

/// Read an int global. Returns 0 on success; `*out` is set.
#[no_mangle]
pub unsafe extern "C" fn mailang_get_global_int(
    interp: *mut MailangInterpreter,
    name: *const c_char,
    out: *mut i64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || name.is_null() || out.is_null() {
            return MAILANG_ERR_NULL;
        }
        let name = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };
        let interp = &mut *interp;
        match interp.inner.get_global(name) {
            Value::Int(n) => {
                *out = n;
                MAILANG_OK
            }
            Value::Bool(b) => {
                *out = b as i64;
                MAILANG_OK
            }
            Value::Float(f) => {
                *out = f as i64;
                MAILANG_OK
            }
            other => {
                let msg = format!("global '{}' is not an int (got {})", name, mailang_stdlib::value_to_string(&other));
                interp.last_error = CString::new(msg).ok();
                MAILANG_ERR_EVAL
            }
        }
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

#[no_mangle]
pub unsafe extern "C" fn mailang_set_global_int(
    interp: *mut MailangInterpreter,
    name: *const c_char,
    value: i64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || name.is_null() {
            return MAILANG_ERR_NULL;
        }
        let name = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };
        let interp = &mut *interp;
        interp.inner.set_global(name, Value::Int(value));
        MAILANG_OK
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

/// Get a string global. Caller must free `*out` with `mailang_free_string`.
#[no_mangle]
pub unsafe extern "C" fn mailang_get_global_str(
    interp: *mut MailangInterpreter,
    name: *const c_char,
    out: *mut *mut c_char,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || name.is_null() || out.is_null() {
            return MAILANG_ERR_NULL;
        }
        let name = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };
        let interp = &mut *interp;
        match interp.inner.get_global(name) {
            Value::Str(s) => {
                let c = CString::new(s.as_ref()).unwrap_or_default();
                *out = c.into_raw();
                MAILANG_OK
            }
            other => {
                let s = mailang_stdlib::value_to_string(&other);
                let c = CString::new(s).unwrap_or_default();
                *out = c.into_raw();
                MAILANG_OK
            }
        }
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

#[no_mangle]
pub unsafe extern "C" fn mailang_set_global_str(
    interp: *mut MailangInterpreter,
    name: *const c_char,
    value: *const c_char,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || name.is_null() || value.is_null() {
            return MAILANG_ERR_NULL;
        }
        let name = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };
        let value = match CStr::from_ptr(value).to_str() {
            Ok(s) => s,
            Err(_) => return MAILANG_ERR_UTF8,
        };
        let interp = &mut *interp;
        interp.inner.set_global(name, Value::Str(value.into()));
        MAILANG_OK
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

/// Register a host function. The callback stays valid for the interpreter lifetime.
#[no_mangle]
pub unsafe extern "C" fn mailang_register_host_fn(
    interp: *mut MailangInterpreter,
    name: *const c_char,
    callback: MailangHostFn,
    user_data: *mut c_void,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() || name.is_null() {
            return MAILANG_ERR_NULL;
        }
        let name = match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return MAILANG_ERR_UTF8,
        };

        // user_data is not Send/Sync; we store it as usize and reconstruct.
        let ud = user_data as usize;
        let interp = &mut *interp;
        interp.inner.register_host_fn(name, move |args: &[Value]| {
            let mut ffi_args: Vec<MailangValue> = args.iter().map(value_to_ffi).collect();
            let mut out = MailangValue::null();
            let rc = callback(
                ud as *mut c_void,
                ffi_args.len() as i32,
                ffi_args.as_ptr(),
                &mut out,
            );
            // Free any temporary strings we allocated for str args.
            for a in &ffi_args {
                if a.tag == 4 && !a.s.is_null() {
                    drop(CString::from_raw(a.s as *mut c_char));
                }
            }
            ffi_args.clear();
            if rc != 0 {
                return Err(format!("host function returned {}", rc));
            }
            ffi_to_value(&out)
        });
        MAILANG_OK
    }))
    .unwrap_or(MAILANG_ERR_PANIC)
}

#[no_mangle]
pub unsafe extern "C" fn mailang_last_error(interp: *const MailangInterpreter) -> *const c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if interp.is_null() {
            return ptr::null();
        }
        let interp = &*interp;
        match &interp.last_error {
            Some(err) => err.as_ptr(),
            None => ptr::null(),
        }
    }))
    .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn mailang_free_string(ptr: *mut c_char) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !ptr.is_null() {
            drop(CString::from_raw(ptr));
        }
    }));
}

#[no_mangle]
pub extern "C" fn mailang_version() -> *mut c_char {
    catch_unwind(|| CString::new(env!("CARGO_PKG_VERSION")).unwrap_or_default().into_raw())
        .unwrap_or(ptr::null_mut())
}

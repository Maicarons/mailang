use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

pub struct MailangInterpreter {
    inner: mailang_core::MailangInterpreter,
    last_error: Option<CString>,
}

#[repr(C)]
pub struct MailangResult {
    pub code: i32,
    pub output: *mut c_char,
}

#[no_mangle]
pub extern "C" fn mailang_create() -> *mut MailangInterpreter {
    let interp = Box::new(MailangInterpreter {
        inner: mailang_core::MailangInterpreter::new(),
        last_error: None,
    });
    Box::into_raw(interp)
}

#[no_mangle]
pub unsafe extern "C" fn mailang_destroy(interp: *mut MailangInterpreter) {
    if !interp.is_null() {
        drop(Box::from_raw(interp));
    }
}

#[no_mangle]
pub unsafe extern "C" fn mailang_eval(
    interp: *mut MailangInterpreter,
    code: *const c_char,
    result: *mut MailangResult,
) -> i32 {
    if interp.is_null() || code.is_null() || result.is_null() {
        return -1;
    }

    let interp = &mut *interp;
    let code_str = match CStr::from_ptr(code).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match interp.inner.eval(code_str) {
        Ok(output) => {
            let c_output = CString::new(output).unwrap_or_default();
            (*result).code = 0;
            (*result).output = c_output.into_raw();
            0
        }
        Err(e) => {
            let c_error = CString::new(e).unwrap_or_default();
            interp.last_error = Some(c_error.clone());
            (*result).code = -1;
            (*result).output = c_error.into_raw();
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn mailang_eval_file(
    interp: *mut MailangInterpreter,
    path: *const c_char,
    result: *mut MailangResult,
) -> i32 {
    if interp.is_null() || path.is_null() || result.is_null() {
        return -1;
    }

    let interp = &mut *interp;
    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match interp.inner.eval_file(path_str) {
        Ok(output) => {
            let c_output = CString::new(output).unwrap_or_default();
            (*result).code = 0;
            (*result).output = c_output.into_raw();
            0
        }
        Err(e) => {
            let c_error = CString::new(e).unwrap_or_default();
            interp.last_error = Some(c_error.clone());
            (*result).code = -1;
            (*result).output = c_error.into_raw();
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn mailang_last_error(
    interp: *const MailangInterpreter,
) -> *const c_char {
    if interp.is_null() {
        return ptr::null();
    }
    let interp = &*interp;
    match &interp.last_error {
        Some(err) => err.as_ptr(),
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn mailang_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

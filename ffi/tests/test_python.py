#!/usr/bin/env python3
"""Python ctypes binding test for MaìLang C FFI."""
import ctypes
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DLL_DIR = os.path.join(ROOT, "target", "debug")
if os.name == "nt":
    DLL = os.path.join(DLL_DIR, "mailang_ffi.dll")
    os.add_dll_directory(DLL_DIR)
else:
    DLL = os.path.join(DLL_DIR, "libmailang_ffi.so")

lib = ctypes.CDLL(DLL)


class MailangResult(ctypes.Structure):
    _fields_ = [
        ("code", ctypes.c_int32),
        ("_pad", ctypes.c_int32),
        ("output", ctypes.c_void_p),
    ]


class MailangValue(ctypes.Structure):
    _fields_ = [
        ("tag", ctypes.c_int32),
        ("_pad", ctypes.c_int32),
        ("i", ctypes.c_int64),
        ("f", ctypes.c_double),
        ("s", ctypes.c_void_p),
    ]


HostFn = ctypes.CFUNCTYPE(
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_int32,
    ctypes.POINTER(MailangValue),
    ctypes.POINTER(MailangValue),
)

lib.mailang_create.restype = ctypes.c_void_p
lib.mailang_destroy.argtypes = [ctypes.c_void_p]
lib.mailang_eval.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.POINTER(MailangResult),
]
lib.mailang_eval.restype = ctypes.c_int
lib.mailang_set_global_int.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_int64,
]
lib.mailang_get_global_int.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_int64),
]
lib.mailang_set_global_str.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_char_p,
]
lib.mailang_register_host_fn.argtypes = [
    ctypes.c_void_p,
    ctypes.c_char_p,
    HostFn,
    ctypes.c_void_p,
]
lib.mailang_free_string.argtypes = [ctypes.c_void_p]


def take_output(r):
    if not r.output:
        return ""
    s = ctypes.cast(r.output, ctypes.c_char_p).value or b""
    text = s.decode()
    lib.mailang_free_string(r.output)
    r.output = None
    return text


def host_add(ud, argc, argv, out):
    if argc != 2:
        return 1
    out.contents.tag = 2
    out.contents.i = argv[0].i + argv[1].i
    return 0


def host_triple(ud, argc, argv, out):
    out.contents.tag = 2
    out.contents.i = argv[0].i * 3
    return 0


def main():
    interp = lib.mailang_create()
    assert interp, "create failed"

    r = MailangResult()
    rc = lib.mailang_eval(interp, b"2 * 21", ctypes.byref(r))
    out = take_output(r)
    assert rc == 0 and out == "42", f"eval failed: {rc} {out!r}"
    print("eval 2*21 =>", out)

    lib.mailang_set_global_int(interp, b"n", 5)
    n = ctypes.c_int64()
    assert lib.mailang_get_global_int(interp, b"n", ctypes.byref(n)) == 0
    assert n.value == 5
    print("n =", n.value)

    rc = lib.mailang_eval(interp, b"n + 1", ctypes.byref(r))
    out = take_output(r)
    assert rc == 0 and out == "6", f"n+1 failed: {out!r}"
    print("n+1 =>", out)

    add_cb = HostFn(host_add)
    triple_cb = HostFn(host_triple)
    assert lib.mailang_register_host_fn(interp, b"host_add", add_cb, None) == 0
    assert lib.mailang_register_host_fn(interp, b"host_triple", triple_cb, None) == 0

    rc = lib.mailang_eval(interp, b"host_add(10, 32)", ctypes.byref(r))
    out = take_output(r)
    assert rc == 0 and out == "42", f"host_add failed: {out!r}"
    print("host_add(10,32) =>", out)

    rc = lib.mailang_eval(interp, b"host_triple(14)", ctypes.byref(r))
    out = take_output(r)
    assert rc == 0 and out == "42", f"host_triple failed: {out!r}"
    print("host_triple(14) =>", out)

    rc = lib.mailang_eval(interp, b"@@@", ctypes.byref(r))
    out = take_output(r)
    assert rc != 0
    print("error path ok, code=", r.code, "msg=", out[:60])

    lib.mailang_destroy(interp)
    print("Python FFI OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

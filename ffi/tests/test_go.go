// Go cgo binding test for MaìLang C FFI.
// Run: go run ffi/tests/test_go.go
package main

/*
#cgo CFLAGS: -I${SRCDIR}/../../crates/mailang-ffi/include
#cgo windows LDFLAGS: ${SRCDIR}/../../target/debug/mailang_ffi.dll
#cgo !windows LDFLAGS: -L${SRCDIR}/../../target/debug -lmailang_ffi
#include <stdlib.h>
#include "mailang.h"

extern int goHostAdd(void* ud, int32_t argc, MailangValue* argv, MailangValue* out);
static int c_go_host_add(void* ud, int32_t argc, const MailangValue* argv, MailangValue* out) {
	return goHostAdd(ud, argc, (MailangValue*)argv, out);
}
static int register_go_add(MailangInterpreter* interp, const char* name) {
	return mailang_register_host_fn(interp, name, c_go_host_add, NULL);
}
*/
import "C"

import (
	"fmt"
	"os"
	"unsafe"
)

//export goHostAdd
func goHostAdd(ud unsafe.Pointer, argc C.int32_t, argv *C.MailangValue, out *C.MailangValue) C.int {
	if argc != 2 {
		return 1
	}
	args := unsafe.Slice(argv, int(argc))
	out.tag = 2
	out.i = args[0].i + args[1].i
	return 0
}

func eval(interp *C.MailangInterpreter, code string) (string, int) {
	ccode := C.CString(code)
	defer C.free(unsafe.Pointer(ccode))
	var res C.MailangResult
	rc := C.mailang_eval(interp, ccode, &res)
	out := C.GoString(res.output)
	if res.output != nil {
		C.mailang_free_string(res.output)
	}
	return out, int(rc)
}

func main() {
	interp := C.mailang_create()
	if interp == nil {
		fmt.Fprintln(os.Stderr, "create failed")
		os.Exit(1)
	}
	defer C.mailang_destroy(interp)

	out, rc := eval(interp, "6 * 7")
	if rc != 0 || out != "42" {
		fmt.Fprintf(os.Stderr, "eval failed: %d %s\n", rc, out)
		os.Exit(1)
	}
	fmt.Println("eval 6*7 =>", out)

	name := C.CString("go_add")
	defer C.free(unsafe.Pointer(name))
	if rc := C.register_go_add(interp, name); rc != 0 {
		fmt.Fprintln(os.Stderr, "register failed", rc)
		os.Exit(1)
	}
	out, rc = eval(interp, "go_add(20, 22)")
	if rc != 0 || out != "42" {
		fmt.Fprintf(os.Stderr, "go_add failed: %d %s\n", rc, out)
		os.Exit(1)
	}
	fmt.Println("go_add(20,22) =>", out)

	fmt.Println("Go FFI OK")
}

// Go demo for MaìLang
// Requires: go get github.com/Maicarons/mailang-go

package main

/*
#cgo LDFLAGS: -L../../target/release -lmailang_ffi
#include <stdlib.h>

typedef struct MailangInterpreter MailangInterpreter;

typedef struct {
    int code;
    char* output;
} MailangResult;

extern MailangInterpreter* mailang_create(void);
extern void mailang_destroy(MailangInterpreter* interp);
extern MailangResult mailang_eval(MailangInterpreter* interp, const char* code);
extern void mailang_free_string(char* ptr);
*/
import "C"
import "fmt"

func main() {
    interp := C.mailang_create()
    defer C.mailang_destroy(interp)

    code := C.CString("1 + 2")
    defer C.free(unsafe.Pointer(code))

    result := C.mailang_eval(interp, code)
    if result.code == 0 {
        fmt.Println("Result:", C.GoString(result.output))
        C.mailang_free_string(result.output)
    }
}

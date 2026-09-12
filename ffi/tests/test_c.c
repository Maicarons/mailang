/* C FFI smoke test for MaìLang.
 * Build (MinGW):
 *   gcc -I crates/mailang-ffi/include -L target/debug -o test_c.exe ffi/tests/test_c.c -lmailang_ffi
 * Then run with PATH including target/debug.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "mailang.h"

static int host_add(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 2 || argv[0].tag != 2 || argv[1].tag != 2) {
        return 1;
    }
    out->tag = 2;
    out->i = argv[0].i + argv[1].i;
    out->f = 0;
    out->s = NULL;
    return 0;
}

static int host_greet(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    (void)argc;
    (void)argv;
    out->tag = 2;
    out->i = 42;
    out->f = 0;
    out->s = NULL;
    return 0;
}

static int fail(const char *msg) {
    fprintf(stderr, "FAIL: %s\n", msg);
    return 1;
}

int main(void) {
    MailangInterpreter *interp = mailang_create();
    if (!interp) return fail("create");

    /* eval arithmetic */
    MailangResult r;
    if (mailang_eval(interp, "1 + 2", &r) != MAILANG_OK) {
        fprintf(stderr, "eval err: %s\n", r.output);
        return fail("eval");
    }
    printf("eval 1+2 => %s\n", r.output);
    if (strcmp(r.output, "3") != 0) return fail("eval result");
    mailang_free_string(r.output);

    /* set/get global int */
    if (mailang_set_global_int(interp, "counter", 7) != MAILANG_OK)
        return fail("set_global_int");
    int64_t n = 0;
    if (mailang_get_global_int(interp, "counter", &n) != MAILANG_OK || n != 7)
        return fail("get_global_int");
    printf("counter = %lld\n", (long long)n);

    /* use host-set global from script */
    if (mailang_eval(interp, "counter * 2", &r) != MAILANG_OK)
        return fail("eval counter");
    printf("counter*2 => %s\n", r.output);
    if (strcmp(r.output, "14") != 0) return fail("counter*2");
    mailang_free_string(r.output);

    /* set/get global str */
    if (mailang_set_global_str(interp, "device", "esp32") != MAILANG_OK)
        return fail("set_global_str");
    char *s = NULL;
    if (mailang_get_global_str(interp, "device", &s) != MAILANG_OK || !s)
        return fail("get_global_str");
    printf("device = %s\n", s);
    if (strcmp(s, "esp32") != 0) return fail("device value");
    mailang_free_string(s);

    /* host function: add */
    if (mailang_register_host_fn(interp, "host_add", host_add, NULL) != MAILANG_OK)
        return fail("register host_add");
    if (mailang_register_host_fn(interp, "host_greet", host_greet, NULL) != MAILANG_OK)
        return fail("register host_greet");
    if (mailang_eval(interp, "host_add(20, 22)", &r) != MAILANG_OK) {
        fprintf(stderr, "eval err: %s\n", r.output);
        return fail("host_add eval");
    }
    printf("host_add(20,22) => %s\n", r.output);
    if (strcmp(r.output, "42") != 0) return fail("host_add result");
    mailang_free_string(r.output);

    /* second eval still sees host fn (Rc re-apply) */
    if (mailang_eval(interp, "host_greet()", &r) != MAILANG_OK)
        return fail("host_greet eval");
    printf("host_greet() => %s\n", r.output);
    mailang_free_string(r.output);

    /* error path */
    if (mailang_eval(interp, "this is not valid !!!", &r) == MAILANG_OK)
        return fail("expected eval error");
    printf("error code=%d last=%s\n", r.code, mailang_last_error(interp) ? mailang_last_error(interp) : "(null)");
    mailang_free_string(r.output);

    mailang_destroy(interp);
    printf("C FFI OK\n");
    return 0;
}

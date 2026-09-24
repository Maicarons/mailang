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

/* Minimal .mailangbc: one "main" chunk with a single Halt (opcode 69).
 * Layout matches crates/mailang-bytecode/src/format.rs (LE integers). */
static const uint8_t MINIMAL_MAILANGBC[] = {
    /* magic */ 'M', 'A', 'I', 'L', 'B', 'C', '0', '1',
    /* version = 1 */ 0x01, 0x00,
    /* flags = 0 */ 0x00, 0x00,
    /* main chunk = 0 */ 0x00, 0x00, 0x00, 0x00,
    /* n_globals = 0 */ 0x00, 0x00, 0x00, 0x00,
    /* n_chunks = 1 */ 0x01, 0x00, 0x00, 0x00,
    /* name_len = 4 */ 0x04, 0x00, 0x00, 0x00,
    /* name = "main" */ 'm', 'a', 'i', 'n',
    /* n_const = 0 */ 0x00, 0x00, 0x00, 0x00,
    /* n_instr = 1 */ 0x01, 0x00, 0x00, 0x00,
    /* opcode = Halt (69) */ 0x45,
    /* has_operand = 0 */ 0x00,
    /* line = 1 */ 0x01, 0x00, 0x00, 0x00,
};

static int smoke_bytecode_apis(void) {
    char *out = NULL;
    MailangStatus st = mailang_eval_bytecode(
        MINIMAL_MAILANGBC, sizeof(MINIMAL_MAILANGBC), &out);
    if (st != MAILANG_OK) {
        fprintf(stderr, "eval_bytecode status=%d out=%s\n", st, out ? out : "(null)");
        if (out) mailang_free_string(out);
        return fail("mailang_eval_bytecode");
    }
    printf("eval_bytecode => %s\n", out ? out : "(null)");
    if (!out || strcmp(out, "null") != 0) {
        if (out) mailang_free_string(out);
        return fail("eval_bytecode result");
    }
    mailang_free_string(out);

    /* write blob to a temp file and load it */
    const char *bc_path = "ffi_test_minimal.mailangbc";
    FILE *fp = fopen(bc_path, "wb");
    if (!fp) return fail("open bytecode file");
    if (fwrite(MINIMAL_MAILANGBC, 1, sizeof(MINIMAL_MAILANGBC), fp) != sizeof(MINIMAL_MAILANGBC)) {
        fclose(fp);
        return fail("write bytecode file");
    }
    fclose(fp);

    out = NULL;
    st = mailang_load_bytecode_file(bc_path, &out);
    if (st != MAILANG_OK) {
        fprintf(stderr, "load_bytecode_file status=%d out=%s\n", st, out ? out : "(null)");
        if (out) mailang_free_string(out);
        remove(bc_path);
        return fail("mailang_load_bytecode_file");
    }
    printf("load_bytecode_file => %s\n", out ? out : "(null)");
    if (!out || strcmp(out, "null") != 0) {
        if (out) mailang_free_string(out);
        remove(bc_path);
        return fail("load_bytecode_file result");
    }
    mailang_free_string(out);
    remove(bc_path);

    /* bad magic must fail decode */
    static const uint8_t BAD[] = {'N', 'O', 'T', 'M', 'A', 'G', 'I', 'C', '0', '0', '0', '0'};
    out = NULL;
    st = mailang_eval_bytecode(BAD, sizeof(BAD), &out);
    if (st != MAILANG_ERR_DECODE) {
        if (out) mailang_free_string(out);
        return fail("expected MAILANG_ERR_DECODE");
    }
    if (out) mailang_free_string(out);
    return 0;
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

    /* precompiled .mailangbc via C API */
    if (smoke_bytecode_apis() != 0) {
        mailang_destroy(interp);
        return 1;
    }

    mailang_destroy(interp);
    printf("C FFI OK\n");
    return 0;
}

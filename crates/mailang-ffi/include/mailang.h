/* mailang.h — C API for embedding MaìLang */
#ifndef MAILANG_H
#define MAILANG_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

typedef struct MailangInterpreter MailangInterpreter;

typedef int32_t MailangStatus;

typedef struct MailangResult {
    int32_t code;
    char *output; /* free with mailang_free_string */
} MailangResult;

/* tag: 0=null, 1=bool, 2=int, 3=float, 4=str (s must remain valid for the call) */
typedef struct MailangValue {
    int32_t tag;
    int64_t i;
    double f;
    const char *s;
} MailangValue;

typedef int (*MailangHostFn)(
    void *user_data,
    int32_t argc,
    const MailangValue *argv,
    MailangValue *out);

#define MAILANG_OK 0
#define MAILANG_ERR_NULL -1
#define MAILANG_ERR_EVAL -2
#define MAILANG_ERR_UTF8 -3
#define MAILANG_ERR_HOST -4
#define MAILANG_ERR_DECODE -5
#define MAILANG_ERR_IO -6
#define MAILANG_ERR_PANIC -99

MailangInterpreter *mailang_create(void);
void mailang_destroy(MailangInterpreter *interp);

int mailang_eval(MailangInterpreter *interp, const char *code, MailangResult *result);
int mailang_eval_file(MailangInterpreter *interp, const char *path, MailangResult *result);

/* Decode a .mailangbc blob and run it in a fresh interpreter.
 * On success *out_result is the program output; on failure an error message.
 * Free *out_result with mailang_free_string. */
MailangStatus mailang_eval_bytecode(const uint8_t *data, size_t len, char **out_result);

/* Read a .mailangbc file, decode and run it in a fresh interpreter.
 * Same out_result contract as mailang_eval_bytecode. */
MailangStatus mailang_load_bytecode_file(const char *path, char **out_result);

int mailang_get_global_int(MailangInterpreter *interp, const char *name, int64_t *out);
int mailang_set_global_int(MailangInterpreter *interp, const char *name, int64_t value);
int mailang_get_global_str(MailangInterpreter *interp, const char *name, char **out);
int mailang_set_global_str(MailangInterpreter *interp, const char *name, const char *value);

int mailang_register_host_fn(
    MailangInterpreter *interp,
    const char *name,
    MailangHostFn callback,
    void *user_data);

const char *mailang_last_error(MailangInterpreter *interp);
void mailang_free_string(char *ptr);
char *mailang_version(void);

#ifdef __cplusplus
}
#endif

#endif /* MAILANG_H */

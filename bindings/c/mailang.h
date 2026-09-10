/**
 * MaìLang C FFI Header
 */

#ifndef MAILANG_H
#define MAILANG_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MailangInterpreter MailangInterpreter;

typedef struct {
    int32_t code;
    char* output;
} MailangResult;

MailangInterpreter* mailang_create(void);
void mailang_destroy(MailangInterpreter* interp);

/**
 * Evaluate MaìLang code.
 * @return 0 on success, -1 on error.
 */
int mailang_eval(MailangInterpreter* interp, const char* code, MailangResult* result);

/**
 * Evaluate a MaìLang file.
 * @return 0 on success, -1 on error.
 */
int mailang_eval_file(MailangInterpreter* interp, const char* path, MailangResult* result);

const char* mailang_last_error(const MailangInterpreter* interp);
void mailang_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* MAILANG_H */

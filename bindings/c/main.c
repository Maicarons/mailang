/**
 * MaìLang C Binding Demo
 */

#include <stdio.h>
#include <stdlib.h>
#include "mailang.h"

int main(void) {
    printf("=== MaìLang C Binding Demo ===\n\n");

    MailangInterpreter* interp = mailang_create();
    if (!interp) {
        fprintf(stderr, "Failed to create interpreter\n");
        return 1;
    }

    MailangResult result;
    int ret;

    /* Test 1: Integer arithmetic */
    printf("Test 1: Integer arithmetic\n");
    ret = mailang_eval(interp, "1 + 2", &result);
    if (ret == 0 && result.code == 0) {
        printf("  1 + 2 = %s\n", result.output);
        mailang_free_string(result.output);
    }

    /* Test 2: String concatenation */
    printf("Test 2: String concatenation\n");
    ret = mailang_eval(interp, "\"Hello, \" + \"MaìLang!\"", &result);
    if (ret == 0 && result.code == 0) {
        printf("  Result: %s\n", result.output);
        mailang_free_string(result.output);
    }

    /* Test 3: Float arithmetic */
    printf("Test 3: Float arithmetic\n");
    ret = mailang_eval(interp, "3.14 * 2.0", &result);
    if (ret == 0 && result.code == 0) {
        printf("  3.14 * 2.0 = %s\n", result.output);
        mailang_free_string(result.output);
    }

    /* Test 4: Boolean */
    printf("Test 4: Boolean\n");
    ret = mailang_eval(interp, "true", &result);
    if (ret == 0 && result.code == 0) {
        printf("  true = %s\n", result.output);
        mailang_free_string(result.output);
    }

    /* Test 5: Null */
    printf("Test 5: Null\n");
    ret = mailang_eval(interp, "null", &result);
    if (ret == 0 && result.code == 0) {
        printf("  null = %s\n", result.output);
        mailang_free_string(result.output);
    }

    /* Test 6: Variable */
    printf("Test 6: Variable\n");
    mailang_eval(interp, "let x = 42", &result);
    ret = mailang_eval(interp, "x", &result);
    if (ret == 0 && result.code == 0) {
        printf("  x = %s\n", result.output);
        mailang_free_string(result.output);
    }

    mailang_destroy(interp);

    printf("\n=== All tests completed ===\n");
    return 0;
}

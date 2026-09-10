use mailang_core::MailangInterpreter;

fn eval(code: &str) -> String {
    let mut interp = MailangInterpreter::new();
    interp.eval(code).unwrap_or_else(|e| panic!("eval failed: {}", e))
}

// ===== 基本算术 =====

#[test]
fn test_integer_add() {
    assert_eq!(eval("1 + 2"), "3");
}

#[test]
fn test_integer_sub() {
    assert_eq!(eval("10 - 3"), "7");
}

#[test]
fn test_integer_mul() {
    assert_eq!(eval("4 * 5"), "20");
}

#[test]
fn test_integer_div() {
    assert_eq!(eval("10 / 3"), "3");
}

#[test]
fn test_integer_mod() {
    assert_eq!(eval("10 % 3"), "1");
}

#[test]
fn test_float_arithmetic() {
    assert_eq!(eval("3.14 * 2.0"), "6.28");
}

#[test]
fn test_precedence() {
    assert_eq!(eval("2 + 3 * 4"), "14");
}

#[test]
fn test_parentheses() {
    assert_eq!(eval("(2 + 3) * 4"), "20");
}

// ===== 字面量 =====

#[test]
fn test_string_literal() {
    assert_eq!(eval("\"hello\""), "hello");
}

#[test]
fn test_bool_true() {
    assert_eq!(eval("true"), "true");
}

#[test]
fn test_bool_false() {
    assert_eq!(eval("false"), "false");
}

#[test]
fn test_null() {
    assert_eq!(eval("null"), "null");
}

// ===== 变量 =====

#[test]
fn test_let_variable() {
    assert_eq!(eval("let x = 42\nx"), "42");
}

#[test]
fn test_var_mutation() {
    assert_eq!(eval("var x = 1\nx = 2\nx"), "2");
}

#[test]
fn test_const() {
    assert_eq!(eval("const PI = 3\nPI"), "3");
}

// ===== 字符串 =====

#[test]
fn test_string_concat() {
    assert_eq!(eval("\"Hello, \" + \"World!\""), "Hello, World!");
}

#[test]
fn test_string_interpolation() {
    assert_eq!(eval("let name = \"MaìLang\"\n\"Hello, {name}!\""), "Hello, MaìLang!");
}

// ===== 比较 =====

#[test]
fn test_eq() {
    assert_eq!(eval("1 == 1"), "true");
}

#[test]
fn test_ne() {
    assert_eq!(eval("1 != 2"), "true");
}

#[test]
fn test_lt() {
    assert_eq!(eval("1 < 2"), "true");
}

#[test]
fn test_gt() {
    assert_eq!(eval("2 > 1"), "true");
}

// ===== 逻辑 =====

#[test]
fn test_and() {
    assert_eq!(eval("true && false"), "false");
}

#[test]
fn test_or() {
    assert_eq!(eval("true || false"), "true");
}

#[test]
fn test_not() {
    assert_eq!(eval("!true"), "false");
}

// ===== 控制流 =====

#[test]
fn test_if_true() {
    assert_eq!(eval("var r = 0\nif true { r = 1 } else { r = 2 }\nr"), "1");
}

#[test]
fn test_if_false() {
    assert_eq!(eval("var r = 0\nif false { r = 1 } else { r = 2 }\nr"), "2");
}

#[test]
fn test_while_loop() {
    assert_eq!(eval("var i = 0\nwhile i < 5 { i = i + 1 }\ni"), "5");
}

#[test]
fn test_for_loop() {
    assert_eq!(eval("var sum = 0\nfor i in 0..5 { sum = sum + i }\nsum"), "10");
}

// ===== 函数 =====

#[test]
fn test_function_call() {
    assert_eq!(eval("fn add(a, b) { return a + b }\nadd(1, 2)"), "3");
}

#[test]
fn test_function_recursion() {
    assert_eq!(eval("fn fib(n) { if n <= 1 { return n } return fib(n-1) + fib(n-2) }\nfib(10)"), "55");
}

#[test]
fn test_lambda() {
    assert_eq!(eval("let sq = fn(x) -> x * x\nsq(5)"), "25");
}

// ===== 数组 =====

#[test]
fn test_array_literal() {
    assert_eq!(eval("[1, 2, 3]"), "[1, 2, 3]");
}

#[test]
fn test_array_index() {
    assert_eq!(eval("let a = [10, 20, 30]\na[1]"), "20");
}

#[test]
fn test_array_len() {
    assert_eq!(eval("let a = [1, 2, 3]\na.len"), "3");
}

// ===== 字典 =====

#[test]
fn test_map_literal() {
    assert_eq!(eval("let m = {\"a\": 1}\nm[\"a\"]"), "1");
}

// ===== match =====

#[test]
fn test_match_literal() {
    assert_eq!(eval("let x = 2\nmatch x {\n1 => \"one\"\n2 => \"two\"\n_ => \"other\"\n}"), "two");
}

#[test]
fn test_match_wildcard() {
    assert_eq!(eval("let x = 99\nmatch x {\n1 => \"one\"\n_ => \"other\"\n}"), "other");
}

// ===== UTF-8 =====

#[test]
fn test_utf8_identifier() {
    assert_eq!(eval("let 中文 = 42\n中文"), "42");
}

#[test]
fn test_utf8_string() {
    assert_eq!(eval("\"你好世界\""), "你好世界");
}

#[test]
fn test_utf8_string_len() {
    // Should count characters, not bytes
    assert_eq!(eval("\"你好\".len"), "2");
}

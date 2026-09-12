use mailang_core::MailangInterpreter;

fn eval(code: &str) -> String {
    let mut interp = MailangInterpreter::new();
    interp
        .eval(code)
        .unwrap_or_else(|e| panic!("eval failed: {}", e))
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
    assert_eq!(
        eval("let name = \"MaìLang\"\n\"Hello, {name}!\""),
        "Hello, MaìLang!"
    );
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
    assert_eq!(
        eval("var sum = 0\nfor i in 0..5 { sum = sum + i }\nsum"),
        "10"
    );
}

// ===== 函数 =====

#[test]
fn test_function_call() {
    assert_eq!(eval("fn add(a, b) { return a + b }\nadd(1, 2)"), "3");
}

#[test]
fn test_function_recursion() {
    assert_eq!(
        eval("fn fib(n) { if n <= 1 { return n } return fib(n-1) + fib(n-2) }\nfib(10)"),
        "55"
    );
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
    assert_eq!(
        eval("let x = 2\nmatch x {\n1 => \"one\"\n2 => \"two\"\n_ => \"other\"\n}"),
        "two"
    );
}

#[test]
fn test_match_wildcard() {
    assert_eq!(
        eval("let x = 99\nmatch x {\n1 => \"one\"\n_ => \"other\"\n}"),
        "other"
    );
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

// ===== Phase B: lexer =====

#[test]
fn test_hex_literal() {
    assert_eq!(eval("0x10"), "16");
}

#[test]
fn test_octal_literal() {
    assert_eq!(eval("0o17"), "15");
}

#[test]
fn test_binary_literal() {
    assert_eq!(eval("0b1010"), "10");
}

#[test]
fn test_block_comment() {
    assert_eq!(eval("/* comment */ 1 + 1"), "2");
}

#[test]
fn test_block_comment_multiline() {
    assert_eq!(eval("/* line1\nline2 */ 42"), "42");
}

// ===== Phase B: generics / Result =====

#[test]
fn test_generic_result_annotation() {
    let code = r#"
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("div0")
    }
    return Ok(a / b)
}
divide(10.0, 2.0)
"#;
    assert_eq!(eval(code), "Ok(5)");
}

#[test]
fn test_option_annotation() {
    let code = r#"
fn find(flag: bool) -> Option<int> {
    if flag {
        return Some(7)
    }
    return None
}
find(true)
"#;
    assert_eq!(eval(code), "Some(7)");
}

// ===== Phase B: Ok/Err/Some patterns =====

#[test]
fn test_match_ok() {
    assert_eq!(eval("match Ok(42) {\nOk(v) => v\nErr(e) => 0\n}"), "42");
}

#[test]
fn test_match_err() {
    assert_eq!(
        eval("match Err(\"boom\") {\nOk(v) => 1\nErr(e) => e\n}"),
        "boom"
    );
}

#[test]
fn test_match_some() {
    assert_eq!(eval("match Some(3) {\nSome(v) => v * 2\nNone => 0\n}"), "6");
}

#[test]
fn test_match_comma_separated_arms() {
    assert_eq!(eval("match Ok(5) { Ok(v) => v, Err(e) => 0 }"), "5");
}

// ===== Phase B: range patterns =====

#[test]
fn test_match_range_inclusive_start() {
    assert_eq!(
        eval("match 5 {\n1..10 => \"small\"\n_ => \"big\"\n}"),
        "small"
    );
}

#[test]
fn test_match_range_outside() {
    assert_eq!(
        eval("match 15 {\n1..10 => \"small\"\n_ => \"big\"\n}"),
        "big"
    );
}

#[test]
fn test_match_range_inclusive_end() {
    assert_eq!(eval("match 10 {\n1..=10 => \"in\"\n_ => \"out\"\n}"), "in");
}

// ===== Phase B: traits =====

#[test]
fn test_trait_default_method() {
    let code = r#"
trait Printable {
    fn to_string() -> str
    fn print() {
        println(this.to_string())
    }
}
class Point implements Printable {
    let x: float
    let y: float
    fn init(x: float, y: float) {
        this.x = x
        this.y = y
    }
    fn to_string() -> str {
        return "({this.x}, {this.y})"
    }
}
let p = Point(1.0, 2.0)
p.to_string()
"#;
    assert_eq!(eval(code), "(1, 2)");
}

#[test]
fn test_trait_required_method_override() {
    let code = r#"
trait Greeter {
    fn greet() -> str
    fn shout() -> str {
        return this.greet()
    }
}
class Hello implements Greeter {
    fn greet() -> str {
        return "hi"
    }
}
let h = Hello()
h.shout()
"#;
    assert_eq!(eval(code), "hi");
}

// ===== Phase B: performance / TCO =====

#[test]
fn test_tail_call_deep_recursion() {
    // Would overflow a non-TCO call stack at this depth.
    let code = r#"
fn count(n) {
    if n <= 0 {
        return 0
    }
    return count(n - 1)
}
count(5000)
"#;
    assert_eq!(eval(code), "0");
}

#[test]
fn test_array_mutation_shared() {
    // Array write is in-place via Rc<RefCell>
    assert_eq!(eval("let a = [1, 2, 3]\na[0] = 9\na"), "[9, 2, 3]");
}

#[test]
fn test_map_property_set() {
    assert_eq!(eval("let m = {\"a\": 1}\nm[\"b\"] = 2\nm[\"b\"]"), "2");
}

// ===== Phase C: bytecode format =====

#[test]
fn test_bytecode_roundtrip() {
    use mailang_core::bytecode::{decode, encode};
    let mut interp = MailangInterpreter::new();
    let bc = interp.compile("let x = 1 + 2\nx").expect("compile");
    let bytes = encode(&bc);
    let back = decode(&bytes).expect("decode");
    assert_eq!(bc, back);
    let out = interp.run_bytecode(back).expect("run");
    assert_eq!(out, "3");
}

// ===== Phase C: simulated HAL =====

#[test]
fn test_gpio_write_read() {
    assert_eq!(eval("gpio_write(5, true)\ngpio_read(5)"), "true");
}

#[test]
fn test_gpio_clear() {
    assert_eq!(
        eval("gpio_write(6, true)\ngpio_write(6, false)\ngpio_read(6)"),
        "false"
    );
}

#[test]
fn test_delay_ms_sim() {
    // delay_ms is a no-op in wall time but must succeed
    assert_eq!(eval("delay_ms(10)\n1"), "1");
}

#[test]
fn test_adc_read_default() {
    assert_eq!(eval("adc_read(0)"), "0");
}

// ===== Phase E: analyzer =====

#[test]
fn test_analyzer_undefined_variable() {
    let mut interp = MailangInterpreter::new();
    let err = interp.eval("not_defined_xyz").unwrap_err();
    assert!(err.contains("Undefined variable"), "got: {}", err);
}

#[test]
fn test_analyzer_allows_builtins() {
    assert_eq!(eval("sqrt(9)"), "3");
}

#[test]
fn test_analyzer_allows_this_super() {
    // oop_demo uses this/super; covered by example but assert via check
    let mut interp = MailangInterpreter::new();
    let src = r#"
class A {
    fn init() { this.x = 1 }
}
class B extends A {
    fn init() { super() }
}
let b = B()
b.x
"#;
    assert_eq!(interp.eval(src).unwrap(), "1");
}

// ===== Phase E: module system v2 =====

#[test]
fn test_module_export_table() {
    use mailang_module::extract_exports;
    let src = "fn a() { return 1 }\nlet b = 2\nconst c = 3\nfn _hidden() { return 0 }\n";
    let mut parser = mailang_core::parser::Parser::new(src).unwrap();
    let program = parser.parse_program().unwrap();
    let exports = extract_exports(&program);
    assert_eq!(exports, vec!["a", "b", "c"]);
}

#[test]
fn test_module_linked_eval() {
    // Relative import of examples/utils.mai from a temp file in examples/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");
    let mut interp = MailangInterpreter::with_modules(&dir);
    let src = "import \"./utils\"\nutils.add(20, 22)";
    assert_eq!(interp.eval(src).unwrap(), "42");
}

#[test]
fn test_subclass_inherits_parent_init() {
    // Child with only an override must still run parent constructor.
    let src = r#"
class Animal {
    let name
    fn init(name) {
        this.name = name
    }
    fn speak() {
        return "{this.name} speaks"
    }
}
class Dog extends Animal {
    fn speak() {
        return "{this.name} barks!"
    }
}
let dog = Dog("Rex")
dog.speak()
"#;
    assert_eq!(eval(src), "Rex barks!");
}

// ===== Phase F: cycle GC =====

#[test]
fn test_gc_breaks_instance_cycle() {
    // Instance field pointing at itself — classic Rc leak.
    let src = r#"
class Node {
    let next
    fn init() { this.next = null }
}
let a = Node()
a.next = a
a.next
"#;
    let mut interp = MailangInterpreter::new();
    assert_eq!(interp.eval(src).unwrap(), "<instance 1>");
    let broken = interp.collect_cycles();
    assert!(broken >= 1, "expected to break self-cycle, got {}", broken);
}

#[test]
fn test_gc_breaks_array_cycle() {
    let mut interp = MailangInterpreter::new();
    let out = interp
        .eval(
            r#"
class Box {
    let items
    fn init() { this.items = [] }
}
let b = Box()
b.items = [b]
1
"#,
        )
        .unwrap();
    assert_eq!(out, "1");
    let broken = interp.collect_cycles();
    assert!(broken >= 1, "expected cycle break, got {}", broken);
}

// ===== Phase F: analyzer span =====

#[test]
fn test_analyzer_diagnostic_position() {
    let src = "let x = 1\nnot_defined_xyz\n";
    let mut parser = mailang_core::parser::Parser::new(src).unwrap();
    let program = parser.parse_program().unwrap();
    let mut analyzer = mailang_core::analyzer::Analyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    let diags = mailang_core::analyzer::diagnose(src, &errs);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 0);
}

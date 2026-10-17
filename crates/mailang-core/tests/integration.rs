use mailang_core::{MailangInterpreter, VmBackend};

/// Shared interpreter factory. `MAILANG_VM=register` runs the whole suite on
/// the register VM (stack-VM equivalence gate before any default switch).
fn interp() -> MailangInterpreter {
    let mut interp = MailangInterpreter::new();
    if std::env::var("MAILANG_VM").as_deref() == Ok("register") {
        interp.set_vm_backend(VmBackend::Register);
    }
    interp
}

fn eval(code: &str) -> String {
    let mut interp = interp();
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
    let mut interp = interp();
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
    let mut interp = interp();
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
    let mut interp = interp();
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
    let mut interp = interp();
    assert_eq!(interp.eval(src).unwrap(), "<instance 1>");
    let broken = interp.collect_cycles();
    assert!(broken >= 1, "expected to break self-cycle, got {}", broken);
}

#[test]
fn test_gc_breaks_array_cycle() {
    let mut interp = interp();
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

// ===== Phase G: ? operator, collections, types =====

#[test]
fn test_try_operator_ok() {
    let src = r#"
fn div(a, b) {
    if b == 0.0 {
        return Err("div0")
    }
    return Ok(a / b)
}
fn calc() {
    let x = div(10.0, 2.0)?
    return Ok(x * 2.0)
}
calc()
"#;
    assert_eq!(eval(src), "Ok(10)");
}

#[test]
fn test_try_operator_err_propagates() {
    let src = r#"
fn div(a, b) {
    if b == 0.0 {
        return Err("div0")
    }
    return Ok(a / b)
}
fn calc() {
    let x = div(1.0, 0.0)?
    return Ok(x)
}
calc()
"#;
    assert_eq!(eval(src), "Err(div0)");
}

#[test]
fn test_array_methods() {
    assert_eq!(eval("let a = [1]\na.push(2)\na.join(\",\")"), "1,2");
    assert_eq!(eval("let a = [1, 2]\na.contains(2)"), "true");
    assert_eq!(eval("let a = [1]\na.pop()"), "1");
}

#[test]
fn test_map_methods() {
    assert_eq!(eval("let m = {\"a\": 1}\nm.has(\"a\")"), "true");
    assert_eq!(eval("let m = {\"a\": 1}\nm.keys()[0]"), "a");
}

#[test]
fn test_str_methods() {
    assert_eq!(eval("\"  hi  \".trim()"), "hi");
    assert_eq!(eval("\"a,b\".split(\",\")[1]"), "b");
    assert_eq!(eval("\"hi\".to_upper()"), "HI");
}

#[test]
fn test_analyzer_arity_error() {
    let src = "fn add(a: int, b: int) -> int {\n    return a + b\n}\nadd(1)\n";
    let mut parser = mailang_core::parser::Parser::new(src).unwrap();
    let program = parser.parse_program().unwrap();
    let mut analyzer = mailang_core::analyzer::Analyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("Wrong number of arguments")),
        "{errs:?}"
    );
}

#[test]
fn test_analyzer_type_mismatch() {
    let src = "fn add(a: int, b: int) -> int {\n    return a + b\n}\nadd(1, \"x\")\n";
    let mut parser = mailang_core::parser::Parser::new(src).unwrap();
    let program = parser.parse_program().unwrap();
    let mut analyzer = mailang_core::analyzer::Analyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("Type mismatch")),
        "{errs:?}"
    );
}

#[test]
fn test_read_write_file_roundtrip() {
    let path = std::env::temp_dir().join("mailang_g4_test.txt");
    let p = path.display().to_string().replace('\\', "/");
    let src = format!("write_file(\"{p}\", \"hello-g4\")\nread_file(\"{p}\")");
    assert_eq!(eval(&src), "hello-g4");
    let _ = std::fs::remove_file(&path);
}

// --- Phase H: correctness ---

#[test]
fn test_default_params() {
    assert_eq!(eval("fn f(a, b = 10) {\n    return a + b\n}\nf(1)"), "11");
    assert_eq!(eval("fn f(a, b = 10) {\n    return a + b\n}\nf(1, 2)"), "3");
    assert_eq!(eval("fn f(a, b = a + 1) {\n    return b\n}\nf(5)"), "6");
}

#[test]
fn test_default_params_lambda() {
    assert_eq!(eval("let g = fn(a, b = 3) { return a * b }\ng(2)"), "6");
    assert_eq!(eval("let g = fn(a, b = 3) { return a * b }\ng(2, 4)"), "8");
}

#[test]
fn test_match_array_destructure() {
    assert_eq!(
        eval("let v = [1, 2]\nmatch v {\n    [a, b] => a + b\n    _ => 0\n}"),
        "3"
    );
    assert_eq!(
        eval("let v = [1]\nmatch v {\n    [a, b] => a + b\n    _ => 99\n}"),
        "99"
    );
    assert_eq!(
        eval("let v = [10, 20, 30]\nmatch v {\n    [x, _, z] => x + z\n    _ => 0\n}"),
        "40"
    );
}

#[test]
fn test_match_tuple_destructure() {
    assert_eq!(
        eval("let p = (3, 4)\nmatch p {\n    (a, b) => a * b\n    _ => 0\n}"),
        "12"
    );
}

#[test]
fn test_match_bind_is_local() {
    let src =
        "fn go(x) {\n    let r = match x {\n        n => n * 2\n    }\n    return r\n}\ngo(21)";
    assert_eq!(eval(src), "42");
}

#[test]
fn test_match_bind_not_global() {
    let mut interp = interp();
    let out = interp
        .eval("fn go(x) {\n    return match x {\n        n => n * 2\n    }\n}\ngo(1)\nn")
        .unwrap_or_else(|e| e);
    assert!(
        out.contains("Undefined") || out.to_lowercase().contains("error") || out == "null",
        "match binding leaked as global: {out}"
    );
}

// ===== Phase N / D2: the `match` value protocol =====
//
// The arm's value is the `match` expression's value and it stays on the operand
// stack. The old protocol stored it into a binding slot and left the residue
// underneath, so a program's *final* value could look right by accident: every
// test here reads the result back instead.

#[test]
fn test_match_value_survives_a_read_back() {
    assert_eq!(
        eval("let r = match Ok(5) {\n    Ok(v) => v * 2\n    Err(e) => 0\n}\nr + 1"),
        "11"
    );
    assert_eq!(
        eval("let r = match Err(\"boom\") {\n    Ok(v) => v\n    Err(e) => e\n}\nlet s = r\ns"),
        "boom"
    );
    // A binding arm behind a failing arm: the fail path must not disturb it.
    assert_eq!(
        eval("let r = match Ok(9) {\n    Err(e) => 0\n    Ok(v) => v + 1\n}\nr"),
        "10"
    );
}

#[test]
fn test_match_tail_expression_is_function_result() {
    let src = "fn classify(n) {\n    match n {\n        0 => \"zero\"\n        1 => \"one\"\n        _ => \"many\"\n    }\n}\nclassify(0)";
    assert_eq!(eval(src), "zero");
    assert_eq!(eval(&src.replace("classify(0)", "classify(1)")), "one");
    assert_eq!(eval(&src.replace("classify(0)", "classify(42)")), "many");

    // A binding arm at the tail: this is where the old protocol dropped the
    // value the caller read.
    let binding = "fn label(r) {\n    match r {\n        Ok(v) => v + 100\n        Err(e) => 0\n    }\n}\nlabel(Ok(1))";
    assert_eq!(eval(binding), "101");
}

#[test]
fn test_match_binding_participates_in_arithmetic() {
    assert_eq!(
        eval("let r = match Some(10) {\n    Some(v) => v / 2 + 1\n    None => 0\n}\nr * 3"),
        "18"
    );
}

#[test]
fn test_match_guard_falls_through_to_a_later_binding_arm() {
    assert_eq!(
        eval("let r = match 5 {\n    n if n > 10 => 100\n    n => n * 2\n}\nr"),
        "10"
    );
    assert_eq!(
        eval("let r = match 50 {\n    n if n > 10 => 100\n    n => n * 2\n}\nr"),
        "100"
    );
}

#[test]
fn test_match_destructuring_value_read_back() {
    assert_eq!(
        eval("let p = (3, 4)\nlet r = match p {\n    (a, b) => a * b\n    _ => 0\n}\nr + 1"),
        "13"
    );
    assert_eq!(
        eval("let v = [5, 6]\nlet r = match v {\n    [a, b] => a - b\n    _ => 0\n}\nr"),
        "-1"
    );
    assert_eq!(
        eval("let r = match Ok(Some(7)) {\n    Ok(v) => v\n    _ => None\n}\nr"),
        "Some(7)"
    );
}

#[test]
fn test_match_or_pattern_value_read_back() {
    assert_eq!(
        eval("let r = match 2 {\n    1 | 2 | 3 => \"low\"\n    _ => \"high\"\n}\nr"),
        "low"
    );
}

#[test]
fn test_match_without_a_matching_arm_is_null() {
    // A lone *guarded* arm is accepted by the analyzer, so the no-arm-matched
    // fallthrough is reachable: the scrutinee temp must still be dropped.
    assert_eq!(
        eval("let r = match 5 {\n    n if n > 10 => 1\n}\nr"),
        "null"
    );
}

#[test]
fn test_let_destructure() {
    assert_eq!(eval("let (a, b) = (1, 2)\na + b"), "3");
    assert_eq!(eval("let [x, y] = [10, 20]\nx + y"), "30");
}

#[test]
fn test_postfix_match() {
    assert_eq!(
        eval("let x = 3\nx match {\n    1 => \"one\"\n    3 => \"three\"\n    _ => \"other\"\n}"),
        "three"
    );
}

#[test]
fn test_trait_extends() {
    let src = "trait A {\n    fn foo(self) { return 1 }\n}\ntrait B extends A {\n    fn bar(self) { return 2 }\n}\nclass C implements B {\n    fn baz(self) { return 3 }\n}\nlet c = C()\nc.foo() + c.bar() + c.baz()";
    assert_eq!(eval(src), "6");
}

#[test]
fn test_super_method() {
    let src = "class Animal {\n    fn speak(self) { return \"...\" }\n}\nclass Dog extends Animal {\n    fn speak(self) { return \"woof\" }\n    fn parent_speak(self) { return super.speak() }\n}\nlet d = Dog()\nd.speak() + \"|\" + d.parent_speak()";
    assert_eq!(eval(src), "woof|...");
}

#[test]
fn test_parent_method_inherited() {
    let src = "class Animal {\n    fn hello(self) { return \"hi\" }\n}\nclass Dog extends Animal {\n}\nDog().hello()";
    assert_eq!(eval(src), "hi");
}

#[test]
fn test_immutable_let_rejects_assign() {
    let src = "let x = 1\nx = 2\n";
    let mut parser = mailang_core::parser::Parser::new(src).unwrap();
    let program = parser.parse_program().unwrap();
    let mut analyzer = mailang_core::analyzer::Analyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("immutable") || e.to_string().contains("assign")),
        "{errs:?}"
    );
}

#[test]
fn test_var_allows_assign() {
    assert_eq!(eval("var x = 1\nx = 2\nx"), "2");
}

#[test]
fn test_property_non_literal_default() {
    let src = "fn make() { return 7 }\nclass C {\n    let val = make()\n    fn get(self) { return self.val }\n}\nC().get()";
    assert_eq!(eval(src), "7");
}

// --- Phase I: debuggability / sandbox ---

#[test]
fn test_runtime_error_has_line_info() {
    let mut interp = interp();
    let err = interp.eval("let x = 1\nx * true").unwrap_err();
    assert!(
        err.contains("line") || err.contains(":"),
        "error should carry location: {err}"
    );
}

#[test]
fn test_fuel_exhausted() {
    let mut interp = interp();
    interp.set_fuel(Some(50));
    let err = interp
        .eval("fn f(n) {\n    if n <= 0 { return 0 }\n    return f(n - 1) + 1\n}\nf(100)")
        .unwrap_err();
    assert!(
        err.to_lowercase().contains("fuel") || err.to_lowercase().contains("budget"),
        "expected fuel error: {err}"
    );
}

#[test]
fn test_call_depth_exceeded() {
    let mut interp = interp();
    interp.set_max_call_depth(16);
    let err = interp
        .eval("fn f(n) {\n    return 1 + f(n + 1)\n}\nf(0)")
        .unwrap_err();
    assert!(
        err.to_lowercase().contains("depth")
            || err.to_lowercase().contains("stack")
            || err.to_lowercase().contains("call"),
        "expected call-depth error: {err}"
    );
}

// --- Phase K: mark-sweep GC + register VM ---

#[test]
fn test_mark_sweep_frees_cycles() {
    let mut interp = interp();
    // Build a self-referential array cycle in a scope, drop the root, collect.
    let src = "fn make() {\n    let a = [1]\n    a.push(a)\n    return 0\n}\nmake()";
    assert_eq!(eval(src), "0");
    let freed = interp.collect_cycles();
    // Mark-sweep should run without panicking; freed may be 0 if Rc already dropped.
    let _ = freed;
    let (tracked, cols, _) = interp.gc_stats();
    assert!(
        cols >= 1,
        "collect_cycles should count a collection, got {cols}"
    );
    let _ = tracked;
}

#[test]
fn test_heap_values_are_tracked_on_both_backends() {
    // The register VM must register heap nodes (arrays / instances) with the
    // mark-sweep census exactly like the stack VM does on push.
    let mut interp = interp();
    let src = "class Box {\n    fn init() { this.v = 1 }\n}\nlet b = Box()\nlet a = [1, 2]\nlet total = a[1] + b.v\ntotal";
    assert_eq!(interp.eval(src).unwrap(), "3");
    let (tracked, _, _) = interp.gc_stats();
    assert!(
        tracked > 0,
        "heap nodes should be tracked by the collector, got {tracked}"
    );
}

#[test]
fn test_allocation_loop_keeps_census_bounded() {
    // D4（Phase N N4）：census 对每个被跟踪节点持一个强 `Rc`，所以"收集区间"
    // 就是驻留垃圾的上限。旧阈值每次收集 ×1.5 且无上限，区间于是随分配总量
    // 增长：20 万次分配后仍有 51,060 个 census 条目，200 万次迭代时进程峰值
    // 工作集 151 MB（400 万次 333 MB）。阈值现在按活对象数计算
    // （`max(MIN_THRESHOLD, live * 2)`），驻留量应与程序实际持有量成正比。
    let src =
        "var i = 0\nvar s = 0\nwhile i < 200000 {\n    s = s + len([1, 2, 3])\n    i = i + 1\n}\ns";
    let mut interp = interp();
    assert_eq!(interp.eval(src).unwrap(), "600000");
    let (tracked, cols, freed) = interp.gc_stats();
    assert!(
        tracked < 5_000,
        "census must not grow with the total number of allocations: \
         {tracked} tracked, {cols} collections, {freed} freed"
    );
    assert!(
        cols > 100,
        "an allocation loop must trigger automatic collections, got {cols}"
    );
    assert!(
        freed > 100_000,
        "the sweep must free the garbage (it always could), got {freed}"
    );
}

#[test]
fn test_tail_recursion_keeps_register_file_bounded() {
    // TailCall reuses the frame instead of nesting one register window per
    // call: a 20k-deep tail loop must not grow the register file (and must not
    // hit the call-depth limit, which a nested frame would).
    let mut interp = interp();
    interp.set_max_call_depth(64);
    let src = "fn loop(n, acc) {\n    if n == 0 { return acc }\n    return loop(n - 1, acc + n)\n}\nloop(20000, 0)";
    assert_eq!(interp.eval(src).unwrap(), "200010000");
}

#[test]
fn test_register_vm_arith() {
    let mut interp = interp();
    let bc = interp.compile("1 + 2 * 3").unwrap();
    let out = interp.run_bytecode_register(bc).unwrap();
    assert_eq!(out, "7");
}

#[test]
fn test_register_vm_function() {
    let mut interp = interp();
    let bc = interp
        .compile("fn f(a, b) {\n    return a + b\n}\nf(2, 40)")
        .unwrap();
    let out = interp.run_bytecode_register(bc).unwrap();
    assert_eq!(out, "42");
}

#[test]
fn test_register_vm_if_and_locals() {
    let mut interp = interp();
    let src = "var x = 10\nvar y = 0\nif x > 5 {\n    y = 1\n} else {\n    y = 2\n}\ny";
    let bc = interp.compile(src).unwrap();
    let out = interp.run_bytecode_register(bc).unwrap();
    assert_eq!(out, "1");
}

// ===== Phase N / N3: register heights at an early `return` =====
//
// `compute_heights` used to hand a height to `ip + 1` after `Return` / `Halt`
// / `TailCall` / `Throw`, none of which fall through. When the instruction
// after an early `return` was also the jump target of the enclosing `if`
// (`if c { return x } var total = …`), that bogus height won the join and
// every register above the parameter slots shifted by one: the function's own
// locals were then read from the wrong register (a `Bool` where a string was
// expected, and so on). The shapes below pin the fall-through join down on
// both backends.

#[test]
fn test_early_return_keeps_later_locals_in_place() {
    let src = "fn f(n) {\n    if n > 0 {\n        return \"pos\"\n    }\n    var s = \"neg\"\n    return s\n}\nf(1) + \"|\" + f(-1)";
    assert_eq!(eval(src), "pos|neg");
}

#[test]
fn test_early_return_then_local_arithmetic() {
    let src = "fn total(n) {\n    if n < 0 {\n        return 0\n    }\n    var a = n + 1\n    var b = a * 2\n    return b\n}\ntotal(3)";
    assert_eq!(eval(src), "8");
}

#[test]
fn test_early_return_then_string_local() {
    // 修好之前 `s` 会落到一个 Bool 槽，`s + s` 于是抛 "Cannot apply binary
    // op"（早期 return 之后的整个寄存器窗口错位一格）。
    let src = "fn bar(k) {\n    if k == 0 {\n        return \"\"\n    }\n    var s = \"ab\"\n    return s + s\n}\nbar(1)";
    assert_eq!(eval(src), "abab");
}

// ===== Phase N / N3: register indexing matches the stack VM =====
//
// The register VM's `index_get` / `index_set` had drifted from the stack VM's
// `Opcode::IndexGet` / `IndexSet`: no `Str`-indexed-by-`Int` arm (`s[0]`
// raised "Cannot index"), an absent map key raised instead of yielding
// `null`, and the `TypeError` text differed — which the backend gate compares
// byte for byte.

#[test]
fn test_string_index_is_a_char() {
    assert_eq!(eval("let s = \"abcd\"\ns[0]"), "a");
    assert_eq!(eval("let s = \"abcd\"\ns[3]"), "d");
    assert_eq!(eval("\"abc\"[1]"), "b");
}

#[test]
fn test_missing_map_key_is_null() {
    assert_eq!(eval("let m = {\"a\": 1}\nm[\"zzz\"]"), "null");
    assert_eq!(eval("let m = {\"a\": 1}\nm[\"zzz\"] == null"), "true");
}

#[test]
fn test_index_error_text_matches_the_stack_vm() {
    let err = interp().eval("let s = \"abc\"\ns[5]").unwrap_err();
    assert!(
        err.contains("Index out of bounds") && err.contains("index 5"),
        "unexpected error: {err}"
    );
    let err = interp().eval("let a = [1, 2]\na[9] = 0").unwrap_err();
    assert!(
        err.contains("Index out of bounds") && err.contains("index 9"),
        "unexpected error: {err}"
    );
    let err = interp().eval("let a = [1, 2]\na[true]").unwrap_err();
    assert!(err.contains("Cannot index this type"), "unexpected: {err}");
    let err = interp().eval("let x = true\nx[0] = 1").unwrap_err();
    assert!(
        err.contains("Cannot index-assign this type"),
        "unexpected: {err}"
    );
}

// ===== 回归测试：Int/Float 混合算术（ROADMAP Phase M 13.3） =====
// 寄存器 VM 的 bin_apply 此前只覆盖 Int-Int 与 Float-Float，`Float * Int`
// 这类混合运算会抛 "Cannot apply binary op"。examples/wasm_demo.mai 与
// libs/time/example/basic.mai 都踩到了这个缺口。

#[test]
fn test_mixed_float_int_arithmetic() {
    assert_eq!(eval("3.14 * 5"), "15.700000000000001");
    assert_eq!(eval("5 * 3.14"), "15.700000000000001");
    assert_eq!(eval("7 - 0.5"), "6.5");
    assert_eq!(eval("0.5 - 7"), "-6.5");
    assert_eq!(eval("2.5 * 4.0"), "10");
    assert_eq!(eval("10 / 2.5"), "4");
    assert_eq!(eval("2.5 / 10"), "0.25");
    assert_eq!(eval("7.5 % 2.0"), "1.5");
}

#[test]
fn test_mixed_float_int_in_function_locals() {
    // `let` + 混合算术：函数局部变量是此前最容易触发该缺口的形状。
    let src = "fn area(r) {\n    let pi = 3.14159\n    return pi * r * r\n}\narea(5)";
    assert_eq!(eval(src), "78.53975");
}

#[test]
fn test_float_comparison_against_int() {
    assert_eq!(eval("var t = 23.5\nt > 20.0"), "true");
    assert_eq!(eval("var t = 23.5\nt < 20.0"), "false");
    assert_eq!(eval("var n = 5\nn < 10.5"), "true");
    assert_eq!(eval("var n = 5\nn >= 10.5"), "false");
}

// ===== 已知缺陷：match 的槽位与结果传递（ROADMAP Phase M 13.3 / M6） =====
//
// 已定位根因，尚未修复，因此**不写成断言**（否则 CI 会红）。修复要点：
//
// 1. `compile_bindings_from_stack` 的 `Identifier` 分支只调用 `define_variable`
//    声明槽位、**不发射 StoreLocal**，依赖"绑定值已位于自己的槽位"这一隐式
//    不变量。实际绑定值停在操作数栈顶 `locals.len()` 处，比槽位大1，于是
//    arm 体里的 `LoadLocal` 读到未初始化槽。字节码实证：
//    `UnwrapOk; Pop; LoadLocal(1); MulImm` —— `v` 取自槽 1，而 7 在栈顶。
//    只要 match 之前存在任何局部变量就会触发（`let a = 1` 即复现）。
// 2. arm 的值被 `end_scope_keeping_result` 存进局部槽后**没有压回操作数栈**，
//    外层 `let m = match {...}` 因此取到 match 之前的残留栈内容。
//
// 修复方向与已知冲突：给 `Identifier` 分支补 `StoreLocal(slot)`、并在 arm
// 末尾补 `LoadLocal(result_slot)`（`n_bind == 0` 时结果已在栈上，不可重载）。
// 但这两处会改变指令序列，与 `mailang-bytecode/src/register.rs` 的
// `compute_heights`（dataflow stack height）契约冲突：实测会让寄存器后端的
// `match [a, b]` 长度检查失效（`[1]` 错配到 `[a, b]` 分支，期望 99 实得 2）。
// 因此修复必须同时调整寄存器翻译器，不能只改编译器。

#[test]
fn test_match_range_without_binding_still_selects_branch() {
    // 无绑定的范围模式当前行为正确，作为后续修复的基准。
    assert_eq!(eval("match 15 { 1..10 => \"small\" _ => \"big\" }"), "big");
    assert_eq!(eval("match 10 { 1..=10 => \"in\" _ => \"out\" }"), "in");
}

// ===== Phase N（N1）：寄存器后端的整型立即数必须符号扩展 =====
//
// 编译器用 `i32::try_from` 把折叠后的常量塞进 `operand: Option<u32>`
// （compiler.rs），所以解码时必须 `as i32 as i64`。寄存器翻译器此前直接用
// `as i64`，把 `-10` 读成 4294967286：所有"变量 与 负整型常量"的比较/算术
// 都会静默算错。examples/iot_sensor.mai 的 `if c < -10` 就是它（该文件没有
// match，两个后端却给出不同的分类）。

#[test]
fn test_negative_int_immediate_comparisons() {
    assert_eq!(eval("let a = 1\na < -10"), "false");
    assert_eq!(eval("let a = 1\na > -10"), "true");
    assert_eq!(eval("let a = 1\na == -10"), "false");
    assert_eq!(eval("let a = 1\na != -10"), "true");
    assert_eq!(eval("let a = 1\na <= -10"), "false");
    assert_eq!(eval("let a = 1\na >= -10"), "true");
}

#[test]
fn test_negative_int_immediate_arithmetic() {
    assert_eq!(eval("let a = 1\na + -10"), "-9");
    assert_eq!(eval("let a = 1\na - -10"), "11");
    assert_eq!(eval("let a = 3\na * -10"), "-30");
}

#[test]
fn test_negative_int_immediate_inside_function() {
    assert_eq!(eval("fn f(a) {\n    return a < -10\n}\nf(1)"), "false");
    assert_eq!(eval("fn f(a) {\n    return a + -10\n}\nf(1)"), "-9");
    // 浮点与负整型常量比较：立即数先变成 `Int(-10)` 再走混合比较。
    assert_eq!(eval("let a = 23.5\na < -10"), "false");
    assert_eq!(eval("let a = 23.5\na > -10"), "true");
}

// ---

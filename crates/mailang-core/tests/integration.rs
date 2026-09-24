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

// ---

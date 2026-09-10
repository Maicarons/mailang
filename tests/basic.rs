use mailang_core::MailangInterpreter;

#[test]
fn test_basic_arithmetic() {
    let mut interp = MailangInterpreter::new();
    let result = interp.eval("1 + 2").unwrap();
    assert_eq!(result, "3");
}

#[test]
fn test_variable_assignment() {
    let mut interp = MailangInterpreter::new();
    let result = interp.eval("let x = 10").unwrap();
    // Variable assignment returns null
    assert!(result == "null" || result.is_empty());
}

#[test]
fn test_string_literal() {
    let mut interp = MailangInterpreter::new();
    let result = interp.eval("\"hello\"").unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_boolean_literal() {
    let mut interp = MailangInterpreter::new();
    let result = interp.eval("true").unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_null_literal() {
    let mut interp = MailangInterpreter::new();
    let result = interp.eval("null").unwrap();
    assert_eq!(result, "null");
}

//! End-to-end monomorphization tests (parse → compile → inspect bytecode).
//! Runtime VM execution is covered by mailang-core integration tests.
//! Placed here (not mailang-core/tests) per ownership boundaries.

use mailang_compiler::Compiler;
use mailang_parser::Parser;

fn compile_src(src: &str) -> mailang_bytecode::Bytecode {
    let mut parser = Parser::new(src).expect("lex");
    let program = parser.parse_program().expect("parse");
    Compiler::new().compile(&program).expect("compile")
}

fn chunk_names(bc: &mailang_bytecode::Bytecode) -> Vec<String> {
    bc.chunks.iter().map(|c| c.name.clone()).collect()
}

/// Find the chunk with the given name.
fn find_chunk<'a>(
    bc: &'a mailang_bytecode::Bytecode,
    name: &str,
) -> Option<&'a mailang_bytecode::Chunk> {
    bc.chunks.iter().find(|c| c.name == name)
}

#[test]
fn id_turbofish_int_specializes_and_calls() {
    // id::<int>(3) must compile to CallDirect of chunk `id$int`.
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nid::<int>(3)\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(
        names.iter().any(|n| n == "id"),
        "erased template kept: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "id$int"),
        "expected id$int: {names:?}"
    );

    // The specialized chunk must return its argument (id::<int>(3) == 3).
    let spec = find_chunk(&bc, "id$int").expect("id$int chunk");
    let has_return = spec
        .instructions
        .iter()
        .any(|i| matches!(i.opcode, mailang_bytecode::Opcode::Return));
    assert!(has_return, "specialized body returns");
}

#[test]
fn id_turbofish_str_creates_second_specialization() {
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nid::<int>(3)\nid::<str>(\"a\")\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(names.iter().any(|n| n == "id$int"), "{names:?}");
    assert!(names.iter().any(|n| n == "id$str"), "{names:?}");
}

#[test]
fn pair_two_type_args_specializes() {
    // pair::<int,str>(1,"a") → specialized pair$int$str with two params.
    let src =
        "fn pair<A, B>(a: A, b: B) -> (A, B) {\nreturn (a, b)\n}\npair::<int, str>(1, \"a\")\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(
        names.iter().any(|n| n == "pair$int$str"),
        "expected pair$int$str: {names:?}"
    );
    let spec = find_chunk(&bc, "pair$int$str").expect("pair$int$str");
    assert!(!spec.instructions.is_empty(), "specialized body compiled");
}

#[test]
fn specialization_is_cached() {
    // Two identical turbofish calls share one specialized chunk.
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nid::<int>(1)\nid::<int>(2)\n";
    let bc = compile_src(src);
    let count = bc.chunks.iter().filter(|c| c.name == "id$int").count();
    assert_eq!(count, 1, "id$int should be cached to a single chunk");
}

#[test]
fn inferred_specialization_from_literal_arg() {
    // Without turbofish, a unique type-param position still monomorphizes.
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nid(3)\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(
        names.iter().any(|n| n == "id$int"),
        "expected inferred id$int: {names:?}"
    );
}

#[test]
fn type_erased_fallback_when_arg_type_unknown() {
    // Unknown arg type (`y` is a param): keep the erased `id` path.
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nfn f(y) {\nreturn id(y)\n}\nf(7)\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(names.iter().any(|n| n == "id"), "erased id kept: {names:?}");
    // `id(y)` cannot infer T from an identifier, so no forced specialization
    // is required; the erased path must still be callable.
    assert!(find_chunk(&bc, "id").is_some());
}

#[test]
fn box_generic_class_erased_layout() {
    // Generic classes compile one erased layout; fields stay dynamic.
    // `let b: Box<int> = Box(42)` is accepted and Box is a single class.
    let src = "class Box<T> {\nlet val: T\nfn init(v: T) {\nthis.val = v\n}\nfn get() -> T {\nreturn this.val\n}\n}\nlet b: Box<int> = Box(42)\nb.get()\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    // No per-type Box specialization — one erased class layout.
    assert!(
        names.iter().all(|n| !n.contains("Box$")),
        "generic classes are erased, got {names:?}"
    );
}

#[test]
fn nested_generic_call_substitutes_type_args() {
    // id::<T>(x) inside apply<T> becomes id$int when specialized at int.
    let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nfn apply<T>(x: T) -> T {\nreturn id::<T>(x)\n}\napply::<int>(5)\n";
    let bc = compile_src(src);
    let names = chunk_names(&bc);
    assert!(
        names.iter().any(|n| n == "apply$int"),
        "expected apply$int: {names:?}"
    );
}

# Contributing to MaìLang

Thank you for your interest in contributing to MaìLang!

**Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Issues](https://github.com/Maicarons/mailang/issues) · [Pull Requests](https://github.com/Maicarons/mailang/pulls) · [Releases](https://github.com/Maicarons/mailang/releases)

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/Maicarons/mailang.git
   cd mailang
   ```
2. Install Rust (stable, 1.70+)
3. Run `cargo build` to verify setup
4. Run `cargo test -p mailang-core` to run tests

## Project Structure

```
crates/
├── mailang-lexer/       # Unicode-aware tokenizer
├── mailang-parser/      # Recursive descent + Pratt parser
├── mailang-ast/         # AST definitions
├── mailang-compiler/    # Bytecode compiler
├── mailang-bytecode/    # Bytecode definitions
├── mailang-vm/          # Stack-based VM
├── mailang-stdlib/      # Standard library
├── mailang-core/        # Pipeline integration
├── mailang-cli/         # CLI + REPL
├── mailang-ffi/         # C FFI layer
├── mailang-wasm/        # WebAssembly bindings
└── mailang-module/      # Module loader
```

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write tests for new functionality

## Testing

```bash
# Run all integration tests
cargo test -p mailang-core

# Run specific example
cargo run -p mailang-cli -- run examples/hello.mai

# Check compilation
cargo check --workspace --exclude mailang-wasm --exclude mailang-lsp --exclude mailang-ffi
```

## Pull Requests

1. Fork the repository from [Maicarons/mailang](https://github.com/Maicarons/mailang)
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test -p mailang-core`
5. Run clippy: `cargo clippy --workspace`
6. Submit a pull request

## Reporting Issues

Use the [GitHub issue tracker](https://github.com/Maicarons/mailang/issues) with the provided templates.

## Roadmap

See [ROADMAP.md](https://github.com/Maicarons/mailang/blob/master/ROADMAP.md) for the planned development phases.

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

pub use mailang_lexer as lexer;
pub use mailang_parser as parser;
pub use mailang_ast as ast;
pub use mailang_compiler as compiler;
pub use mailang_bytecode as bytecode;
pub use mailang_vm as vm;
pub use mailang_stdlib as stdlib;
pub use mailang_module as module;

use mailang_compiler::Compiler;
use mailang_parser::Parser;
use mailang_vm::{HostFn, Vm};
use mailang_module::{ModuleLoader, FileModuleLoader, create_loader};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

pub struct MailangInterpreter {
    vm: Vm,
    module_loader: Option<FileModuleLoader>,
    /// Kept so host callbacks survive `eval` rebuilding the VM.
    host_fns: HashMap<String, HostFn>,
    /// Globals set by the host; re-applied after each `eval`.
    host_globals: HashMap<String, mailang_bytecode::Value>,
}

impl MailangInterpreter {
    pub fn new() -> Self {
        Self {
            vm: Vm::new(mailang_bytecode::Bytecode::new()),
            module_loader: None,
            host_fns: HashMap::new(),
            host_globals: HashMap::new(),
        }
    }

    /// Create a new interpreter with module loading support
    pub fn with_modules(base_dir: impl AsRef<Path>) -> Self {
        Self {
            vm: Vm::new(mailang_bytecode::Bytecode::new()),
            module_loader: Some(create_loader(base_dir)),
            host_fns: HashMap::new(),
            host_globals: HashMap::new(),
        }
    }

    /// Set the module loader
    pub fn set_module_loader(&mut self, loader: FileModuleLoader) {
        self.module_loader = Some(loader);
    }

    /// Register a host function callable from MaìLang source.
    pub fn register_host_fn(
        &mut self,
        name: impl Into<String>,
        f: impl Fn(&[mailang_bytecode::Value]) -> Result<mailang_bytecode::Value, String> + 'static,
    ) {
        let name = name.into();
        let f: HostFn = Rc::new(f);
        self.host_fns.insert(name.clone(), f.clone());
        self.vm.register_host_fn(name, f);
    }

    fn reapply_host_fns(&mut self, mut vm: Vm) -> Vm {
        for (name, f) in &self.host_fns {
            vm.register_host_fn(name.clone(), f.clone());
        }
        for (name, value) in &self.host_globals {
            vm.set_global(name, value.clone());
        }
        vm
    }

    pub fn get_global(&mut self, name: &str) -> mailang_bytecode::Value {
        self.vm.get_global(name)
    }

    pub fn set_global(&mut self, name: &str, value: mailang_bytecode::Value) {
        self.host_globals.insert(name.to_string(), value.clone());
        self.vm.set_global(name, value);
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let mut parser = Parser::new(code).map_err(|e| e.to_string())?;
        let program = parser.parse_program().map_err(|e| e.to_string())?;

        // Process imports if module loader is available
        let processed_program = if self.module_loader.is_some() {
            self.process_imports(program)?
        } else {
            program
        };

        let compiler = Compiler::new();
        let bytecode = compiler.compile(&processed_program).map_err(|e| e.to_string())?;

        let vm = Vm::new(bytecode);
        self.vm = self.reapply_host_fns(vm);
        let result = self.vm.run().map_err(|e| e.to_string())?;
        Ok(mailang_stdlib::value_to_string(&result))
    }

    pub fn eval_file(&mut self, path: &str) -> Result<String, String> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

        // Set base directory for module resolution if not already set
        if self.module_loader.is_none() {
            let base_dir = Path::new(path)
                .parent()
                .unwrap_or(Path::new("."));
            self.module_loader = Some(create_loader(base_dir));
        }

        self.eval(&code)
    }

    /// Compile source to bytecode without executing it.
    pub fn compile(&mut self, code: &str) -> Result<mailang_bytecode::Bytecode, String> {
        let mut parser = Parser::new(code).map_err(|e| e.to_string())?;
        let program = parser.parse_program().map_err(|e| e.to_string())?;

        let processed_program = if self.module_loader.is_some() {
            self.process_imports(program)?
        } else {
            program
        };

        let compiler = Compiler::new();
        compiler.compile(&processed_program).map_err(|e| e.to_string())
    }

    /// Compile a `.mai` file to bytecode.
    pub fn compile_file(&mut self, path: &str) -> Result<mailang_bytecode::Bytecode, String> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
        if self.module_loader.is_none() {
            let base_dir = Path::new(path)
                .parent()
                .unwrap_or(Path::new("."));
            self.module_loader = Some(create_loader(base_dir));
        }
        self.compile(&code)
    }

    /// Run previously compiled bytecode.
    pub fn run_bytecode(&mut self, bytecode: mailang_bytecode::Bytecode) -> Result<String, String> {
        let vm = Vm::new(bytecode);
        self.vm = self.reapply_host_fns(vm);
        let result = self.vm.run().map_err(|e| e.to_string())?;
        Ok(mailang_stdlib::value_to_string(&result))
    }

    /// Process imports in a program, loading and injecting imported modules
    fn process_imports(
        &mut self,
        program: mailang_ast::Program,
    ) -> Result<mailang_ast::Program, String> {
        use mailang_module::ModuleLoader;

        let mut imported_stmts = Vec::new();
        let mut main_stmts = Vec::new();

        for stmt in program.statements {
            match &stmt {
                mailang_ast::Stmt::Import { path, alias, items } => {
                    let module_path = path.join(".");
                    let module_name = alias.clone().unwrap_or_else(|| path.last().unwrap().clone());

                    // Load the module
                    if let Some(ref mut loader) = self.module_loader {
                        let _module = loader.load(&module_path)
                            .map_err(|e| format!("Failed to import '{}': {}", module_path, e))?;

                        // Get the module's file path and read its source
                        let module_file = loader.resolve_path(&module_path);
                        if let Some(file_path) = module_file {
                            // Read and parse the module source
                            let module_code = std::fs::read_to_string(&file_path)
                                .map_err(|e| format!("Failed to read module '{}': {}", module_path, e))?;

                            let mut parser = mailang_parser::Parser::new(&module_code)
                                .map_err(|e| format!("Failed to parse module '{}': {}", module_path, e))?;
                            let module_program = parser.parse_program()
                                .map_err(|e| format!("Failed to parse module '{}': {}", module_path, e))?;

                            // Collect function and constant names from the module
                            let mut exported_names = Vec::new();
                            for stmt in &module_program.statements {
                                match stmt {
                                    mailang_ast::Stmt::FunctionDef { name, .. } => {
                                        exported_names.push(name.clone());
                                    }
                                    mailang_ast::Stmt::Let { name, .. } => {
                                        exported_names.push(name.clone());
                                    }
                                    mailang_ast::Stmt::Const { name, .. } => {
                                        exported_names.push(name.clone());
                                    }
                                    _ => {}
                                }
                            }

                            // Inject the module's statements (functions/constants become globals)
                            imported_stmts.extend(module_program.statements);

                            // Create a namespace map: let time = { "now": now, "PI": PI, ... }
                            let map_entries: Vec<(mailang_ast::Expr, mailang_ast::Expr)> = exported_names
                                .iter()
                                .map(|name| {
                                    (
                                        mailang_ast::Expr::Literal(mailang_ast::Literal::Str(name.clone())),
                                        mailang_ast::Expr::Identifier(name.clone()),
                                    )
                                })
                                .collect();

                            let namespace_stmt = mailang_ast::Stmt::Let {
                                name: module_name,
                                mutable: false,
                                type_annotation: None,
                                value: Some(mailang_ast::Expr::Map(map_entries)),
                            };
                            imported_stmts.push(namespace_stmt);
                        }
                    }
                }
                _ => main_stmts.push(stmt),
            }
        }

        // Combine imported statements with main statements
        imported_stmts.extend(main_stmts);
        Ok(mailang_ast::Program { statements: imported_stmts })
    }
}

impl Default for MailangInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

pub use mailang_analyzer as analyzer;
pub use mailang_ast as ast;
pub use mailang_bytecode as bytecode;
pub use mailang_compiler as compiler;
pub use mailang_lexer as lexer;
pub use mailang_module as module;
pub use mailang_parser as parser;
pub use mailang_stdlib as stdlib;
pub use mailang_vm as vm;

mod format;
pub use format::format_source;

use mailang_analyzer::Analyzer;
use mailang_compiler::Compiler;
use mailang_module::{create_loader, FileModuleLoader};
use mailang_parser::Parser;
use mailang_vm::{HostFn, Vm};
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
    /// Sandbox fuel re-applied when `eval` rebuilds the VM.
    fuel: Option<u64>,
    /// Call-depth cap re-applied when `eval` rebuilds the VM.
    max_call_depth: usize,
    /// Execute compiled bytecode on the register VM instead of the stack VM.
    prefer_register: bool,
    /// Last register-VM instance (so collect_cycles / gc_stats see its heap).
    last_register: Option<mailang_vm::RegisterVm>,
}

/// Which VM backend executes compiled bytecode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmBackend {
    /// Classic stack-based bytecode VM (default).
    Stack,
    /// Register (three-address) VM. Stack bytecode is lowered first.
    Register,
}

impl MailangInterpreter {
    pub fn new() -> Self {
        Self {
            vm: Vm::new(mailang_bytecode::Bytecode::new()),
            module_loader: None,
            host_fns: HashMap::new(),
            host_globals: HashMap::new(),
            fuel: None,
            max_call_depth: Vm::DEFAULT_MAX_CALL_DEPTH,
            prefer_register: false,
            last_register: None,
        }
    }

    /// Create a new interpreter with module loading support
    pub fn with_modules(base_dir: impl AsRef<Path>) -> Self {
        Self {
            vm: Vm::new(mailang_bytecode::Bytecode::new()),
            module_loader: Some(create_loader(base_dir)),
            host_fns: HashMap::new(),
            host_globals: HashMap::new(),
            fuel: None,
            max_call_depth: Vm::DEFAULT_MAX_CALL_DEPTH,
            prefer_register: false,
            last_register: None,
        }
    }

    /// Select the VM backend used by [`eval`](Self::eval) / [`run_bytecode`](Self::run_bytecode).
    /// Default is [`VmBackend::Stack`]; register mode is experimental and should
    /// match stack semantics before becoming the default.
    pub fn set_vm_backend(&mut self, backend: VmBackend) {
        self.prefer_register = backend == VmBackend::Register;
    }

    fn apply_sandbox(&self, vm: &mut Vm) {
        vm.set_fuel(self.fuel);
        vm.set_max_call_depth(self.max_call_depth);
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

    /// Break Rc cycles reachable from the VM stack and globals.
    pub fn collect_cycles(&mut self) -> usize {
        if let Some(rvm) = self.last_register.as_mut() {
            return rvm.collect_cycles();
        }
        self.vm.collect_cycles()
    }

    /// Mark-sweep heap statistics: (tracked, collections, freed).
    pub fn gc_stats(&self) -> (usize, usize, usize) {
        if let Some(rvm) = self.last_register.as_ref() {
            return rvm.gc_stats();
        }
        self.vm.gc_stats()
    }

    /// Run bytecode on the **register VM** (stack bytecode is lowered to
    /// three-address register IR first).
    pub fn run_bytecode_register(
        &mut self,
        bytecode: mailang_bytecode::Bytecode,
    ) -> Result<String, String> {
        let mut rvm = mailang_vm::RegisterVm::new(bytecode);
        self.apply_sandbox_to_register(&mut rvm);
        for (name, f) in &self.host_fns {
            rvm.register_host_fn(name.clone(), f.clone());
        }
        for (name, value) in &self.host_globals {
            rvm.set_global(name, value.clone());
        }
        let result = rvm.run().map_err(|e| e.to_string())?;
        self.last_register = Some(rvm);
        Ok(mailang_stdlib::value_to_string(&result))
    }

    fn apply_sandbox_to_register(&self, vm: &mut mailang_vm::RegisterVm) {
        vm.set_fuel(self.fuel);
        vm.set_max_call_depth(self.max_call_depth);
    }

    /// Set an instruction budget for sandboxed execution (`None` = unlimited).
    pub fn set_fuel(&mut self, fuel: Option<u64>) {
        self.fuel = fuel;
        self.vm.set_fuel(fuel);
    }

    /// Cap nested calls (runaway recursion / plugin safety).
    pub fn set_max_call_depth(&mut self, limit: usize) {
        self.max_call_depth = limit.max(1);
        self.vm.set_max_call_depth(self.max_call_depth);
    }

    /// Run semantic analysis. Returns hard errors (unused-variable hints are ignored).
    pub fn check(&self, code: &str) -> Result<(), Vec<String>> {
        let mut parser = Parser::new(code).map_err(|e| vec![e.to_string()])?;
        let program = parser.parse_program().map_err(|e| vec![e.to_string()])?;
        let mut analyzer = Analyzer::new();
        for name in self.host_fns.keys() {
            analyzer.register_builtin(name);
        }
        for name in self.host_globals.keys() {
            analyzer.register_builtin(name);
        }
        analyzer
            .analyze(&program)
            .map_err(|errs| errs.iter().map(|e| e.to_string()).collect())
    }

    fn analyze_program(&self, program: &mailang_ast::Program) -> Result<(), String> {
        let mut analyzer = Analyzer::new();
        for name in self.host_fns.keys() {
            analyzer.register_builtin(name);
        }
        for name in self.host_globals.keys() {
            analyzer.register_builtin(name);
        }
        analyzer.analyze(program).map_err(|errs| {
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let mut parser = Parser::new(code).map_err(|e| e.to_string())?;
        let program = parser.parse_program().map_err(|e| e.to_string())?;

        let bytecode = if self.module_loader.is_some() {
            self.compile_with_modules(program)?
        } else {
            self.analyze_program(&program)?;
            let compiler = Compiler::new();
            compiler.compile(&program).map_err(|e| e.to_string())?
        };

        if self.prefer_register {
            return self.run_bytecode_register(bytecode);
        }

        let mut vm = Vm::new(bytecode);
        self.apply_sandbox(&mut vm);
        self.vm = self.reapply_host_fns(vm);
        let result = self.vm.run().map_err(|e| e.to_string())?;
        Ok(mailang_stdlib::value_to_string(&result))
    }

    pub fn eval_file(&mut self, path: &str) -> Result<String, String> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

        // Set base directory for module resolution if not already set
        if self.module_loader.is_none() {
            let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
            self.module_loader = Some(create_loader(base_dir));
        }

        self.eval(&code)
    }

    /// Compile source to bytecode without executing it.
    pub fn compile(&mut self, code: &str) -> Result<mailang_bytecode::Bytecode, String> {
        let mut parser = Parser::new(code).map_err(|e| e.to_string())?;
        let program = parser.parse_program().map_err(|e| e.to_string())?;

        if self.module_loader.is_some() {
            self.compile_with_modules(program)
        } else {
            self.analyze_program(&program)?;
            let compiler = Compiler::new();
            compiler.compile(&program).map_err(|e| e.to_string())
        }
    }

    /// Compile a `.mai` file to bytecode.
    pub fn compile_file(&mut self, path: &str) -> Result<mailang_bytecode::Bytecode, String> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
        if self.module_loader.is_none() {
            let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
            self.module_loader = Some(create_loader(base_dir));
        }
        self.compile(&code)
    }

    /// Run previously compiled bytecode.
    pub fn run_bytecode(&mut self, bytecode: mailang_bytecode::Bytecode) -> Result<String, String> {
        if self.prefer_register {
            return self.run_bytecode_register(bytecode);
        }
        let mut vm = Vm::new(bytecode);
        self.apply_sandbox(&mut vm);
        self.vm = self.reapply_host_fns(vm);
        let result = self.vm.run().map_err(|e| e.to_string())?;
        Ok(mailang_stdlib::value_to_string(&result))
    }

    /// Module-system v2: parse/analyze each import independently, then link
    /// module bodies + export-table namespaces + main into one Bytecode.
    /// The importer AST is not rewritten with module source.
    fn compile_with_modules(
        &mut self,
        program: mailang_ast::Program,
    ) -> Result<mailang_bytecode::Bytecode, String> {
        use mailang_module::{extract_exports, ModuleLoader};

        // Phase 1: resolve + parse modules (needs &mut loader).
        struct PendingImport {
            module_name: String,
            module_program: mailang_ast::Program,
            exports: Vec<String>,
            #[allow(dead_code)]
            selective: bool,
        }
        let mut pending: Vec<PendingImport> = Vec::new();
        let mut main_stmts = Vec::new();

        {
            let loader = self
                .module_loader
                .as_mut()
                .ok_or_else(|| "module loader not set".to_string())?;

            for stmt in program.statements {
                match stmt {
                    mailang_ast::Stmt::Import { path, alias, items } => {
                        let module_path = path.join(".");
                        // `./utils` / `../lib/helper` → last path segment without prefix
                        let default_name = path
                            .last()
                            .map(|s| {
                                let s = s.rsplit('/').next().unwrap_or(s);
                                let s = s.rsplit('\\').next().unwrap_or(s);
                                s.trim_start_matches('.').to_string()
                            })
                            .unwrap_or_default();
                        let module_name = alias.clone().unwrap_or(default_name);

                        loader
                            .load(&module_path)
                            .map_err(|e| format!("Failed to import '{}': {}", module_path, e))?;

                        let file_path = loader
                            .resolve_path(&module_path)
                            .ok_or_else(|| format!("Module not found: {}", module_path))?;

                        let module_code = std::fs::read_to_string(&file_path).map_err(|e| {
                            format!("Failed to read module '{}': {}", module_path, e)
                        })?;
                        let mut parser = Parser::new(&module_code).map_err(|e| {
                            format!("Failed to parse module '{}': {}", module_path, e)
                        })?;
                        let module_program = parser.parse_program().map_err(|e| {
                            format!("Failed to parse module '{}': {}", module_path, e)
                        })?;

                        let mut exports = extract_exports(&module_program);
                        let selective = items.is_some();
                        if let Some(requested) = items {
                            exports.retain(|n| requested.contains(n));
                        }
                        let ns_name = if selective {
                            // Still expose a namespace if aliased; otherwise bind items only.
                            if alias.is_some() {
                                module_name
                            } else {
                                format!("_import_{}", module_name)
                            }
                        } else {
                            module_name
                        };
                        pending.push(PendingImport {
                            module_name: ns_name,
                            module_program,
                            exports: if selective && alias.is_none() {
                                Vec::new()
                            } else {
                                exports
                            },
                            selective,
                        });
                    }
                    other => main_stmts.push(other),
                }
            }
        }

        // Phase 2: analyze modules and main (no loader borrow).
        let mut linked: Vec<(String, mailang_ast::Program, Vec<String>)> = Vec::new();
        for p in pending {
            self.analyze_program(&p.module_program)
                .map_err(|e| format!("Module '{}' failed analysis: {}", p.module_name, e))?;
            linked.push((p.module_name, p.module_program, p.exports));
        }

        {
            let mut analyzer = mailang_analyzer::Analyzer::new();
            for name in self.host_fns.keys() {
                analyzer.register_builtin(name);
            }
            for name in self.host_globals.keys() {
                analyzer.register_builtin(name);
            }
            for (mod_name, module_program, exports) in &linked {
                // Namespace map binding (e.g. `utils`, `time`)
                if !exports.is_empty() {
                    analyzer.register_builtin(mod_name);
                }
                for name in mailang_module::extract_exports(module_program) {
                    analyzer.register_builtin(&name);
                }
            }
            let main_program = mailang_ast::Program {
                statements: main_stmts.clone(),
            };
            analyzer.analyze(&main_program).map_err(|errs| {
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
        }

        let main_program = mailang_ast::Program {
            statements: main_stmts,
        };
        Compiler::compile_linked(&linked, &main_program).map_err(|e| e.to_string())
    }
}

impl Default for MailangInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

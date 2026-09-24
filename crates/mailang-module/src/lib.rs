use mailang_ast::Program;
use mailang_bytecode::Bytecode;
use mailang_compiler::Compiler;
use mailang_parser::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod deps;
pub mod registry;
pub use deps::{
    content_hash, content_hash_file, dependency_map, ensure_lock, find_offline_package,
    find_project_root, format_lockfile, global_pkg_dir, home_dir, load_manifest,
    lock_entries_from_resolved, lock_path_string, parse_add_spec, parse_lockfile, parse_manifest,
    parse_semver_lite, plan_add, read_lockfile, resolve_dependencies, resolve_dependencies_locked,
    resolve_dependencies_raw, semver_lite_cmp, semver_lite_eq, upsert_dependency, vendor_dir,
    verify_lock, write_lockfile, AddPlan, AddSpec, DepSource, Dependency, EntryLockStatus,
    LockEntry, LockEntryStatus, LockOutcome, LockReport, ProjectManifest, ResolvedDependency,
    SemverLite,
};
pub use registry::{
    format_index, format_index_line, index_file_path, index_rel, install, load_publish_meta,
    mpkg_path, name_prefix, pack_mpkg, parse_index, parse_pkg_spec, pkg_dir_path, publish,
    read_index, registry_init, resolve_version, search, set_yanked, unpack_mpkg,
    upsert_index_entry, write_index, IndexEntry, InstallResult, PublishMeta, PublishResult,
    RegistryError, RegistrySource,
};

/// Module information from mailib.ini
#[derive(Debug, Clone, Default)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry: String, // entry file, default "lib.mai"
}

/// Errors that can occur during module loading
#[derive(Debug, Clone)]
pub enum ModuleError {
    NotFound(String),
    ParseError(String),
    CompileError(String),
    IoError(String),
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::NotFound(path) => write!(f, "Module not found: {}", path),
            ModuleError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ModuleError::CompileError(msg) => write!(f, "Compile error: {}", msg),
            ModuleError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

/// Collect top-level exportable names from a parsed program.
/// Exports are `fn` / `let` / `const` bindings at module scope.
pub fn extract_exports(program: &Program) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in &program.statements {
        match stmt {
            mailang_ast::Stmt::FunctionDef { name, .. }
            | mailang_ast::Stmt::Let { name, .. }
            | mailang_ast::Stmt::Const { name, .. }
                if !name.starts_with('_') =>
            {
                names.push(name.clone());
            }
            _ => {}
        }
    }
    names
}

/// A compiled module with its bytecode and exports
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub name: String,
    pub info: ModuleInfo,
    pub bytecode: Bytecode,
    pub exports: HashMap<String, u32>,
}

/// Trait for loading modules
pub trait ModuleLoader {
    /// Load and compile a module by path or name
    fn load(&mut self, path: &str) -> Result<&CompiledModule, ModuleError>;

    /// Check if a module is already loaded
    fn is_loaded(&self, path: &str) -> bool;

    /// Get a loaded module by path
    fn get(&self, path: &str) -> Option<&CompiledModule>;
}

/// File-based module loader
pub struct FileModuleLoader {
    /// Base directory for resolving relative paths
    base_dir: PathBuf,
    /// Library directory (e.g., next to CLI executable)
    lib_dir: Option<PathBuf>,
    /// Project root containing `mailang.toml` (if discovered)
    project_root: Option<PathBuf>,
    /// Explicit named deps from `mailang.toml` `[dependencies]`
    deps: HashMap<String, PathBuf>,
    /// Loaded modules cache
    modules: HashMap<String, CompiledModule>,
    /// Built-in modules
    builtins: HashMap<String, CompiledModule>,
}

impl FileModuleLoader {
    /// Create a new file module loader
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        let mut loader = Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            lib_dir: None,
            project_root: None,
            deps: HashMap::new(),
            modules: HashMap::new(),
            builtins: HashMap::new(),
        };
        loader.load_project_deps();
        loader
    }

    /// Discover `mailang.toml` (from `base_dir` upward) and load `[dependencies]`.
    pub fn load_project_deps(&mut self) {
        if let Some(root) = deps::find_project_root(&self.base_dir) {
            self.deps = deps::dependency_map(&root);
            self.project_root = Some(root);
        }
    }

    /// Project root that provided `mailang.toml`, if any.
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    /// Explicit dependencies from `mailang.toml`.
    pub fn dependencies(&self) -> &HashMap<String, PathBuf> {
        &self.deps
    }

    /// Set the library directory (for named imports like `import "sys"`)
    pub fn set_lib_dir(&mut self, path: impl AsRef<Path>) {
        self.lib_dir = Some(path.as_ref().to_path_buf());
    }

    /// Register a built-in module
    pub fn register_builtin(&mut self, name: &str, module: CompiledModule) {
        self.builtins.insert(name.to_string(), module);
    }

    /// Parse mailib.ini file
    fn parse_module_info(dir: &Path) -> ModuleInfo {
        let ini_path = dir.join("mailib.ini");
        let mut info = ModuleInfo {
            entry: "lib.mai".to_string(),
            ..Default::default()
        };

        if let Ok(content) = std::fs::read_to_string(&ini_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim().trim_matches('"');
                    match key {
                        "name" => info.name = value.to_string(),
                        "version" => info.version = value.to_string(),
                        "description" => info.description = value.to_string(),
                        "author" => info.author = value.to_string(),
                        "entry" => info.entry = value.to_string(),
                        _ => {}
                    }
                }
            }
        }

        info
    }

    /// Resolve a module path to a directory or file
    /// Supports two modes:
    /// 1. Named module: `import "sys"` -> looks in lib_dir/sys/lib.mai
    /// 2. Relative path: `import "./utils"` or `import "../lib/helper"`
    pub fn resolve_path(&self, path: &str) -> Option<PathBuf> {
        // Check built-ins first
        if self.builtins.contains_key(path) {
            return None; // Built-in, no file path
        }

        // Mode 1: Relative path (starts with ./ or ../)
        if path.starts_with("./") || path.starts_with("../") {
            return self.resolve_relative(path);
        }

        // Mode 2: Absolute path
        let abs_path = Path::new(path);
        if abs_path.is_absolute() {
            return self.resolve_absolute(abs_path);
        }

        // Mode 3: Named module (e.g., "sys", "time")
        // Resolution order: (1) mailang.toml deps, (2) local libs/, (3) CWD/libs.
        self.resolve_named(path)
    }

    /// Resolve a relative path
    fn resolve_relative(&self, path: &str) -> Option<PathBuf> {
        let full_path = self.base_dir.join(path);

        // Try as direct file
        if full_path.exists() && full_path.is_file() {
            return Some(full_path);
        }

        // Try with .mai extension
        let mai_path = full_path.with_extension("mai");
        if mai_path.exists() {
            return Some(mai_path);
        }

        // Try as directory with lib.mai
        if full_path.is_dir() {
            let lib_path = full_path.join("lib.mai");
            if lib_path.exists() {
                return Some(lib_path);
            }
        }

        None
    }

    /// Resolve an absolute path
    fn resolve_absolute(&self, path: &Path) -> Option<PathBuf> {
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }

        let mai_path = path.with_extension("mai");
        if mai_path.exists() {
            return Some(mai_path);
        }

        if path.is_dir() {
            let lib_path = path.join("lib.mai");
            if lib_path.exists() {
                return Some(lib_path);
            }
        }

        None
    }

    /// Resolve a named module (e.g., "sys" -> libs/sys/lib.mai)
    ///
    /// Search order:
    /// 1. Explicit `[dependencies]` entries from `mailang.toml`
    /// 2. `lib_dir` / `base_dir/libs` / `CWD/libs`
    fn resolve_named(&self, name: &str) -> Option<PathBuf> {
        // 1. Explicit dependency from mailang.toml
        if let Some(dep) = self.deps.get(name) {
            if dep.is_file() {
                return Some(dep.clone());
            }
            if let Some(parent) = dep.parent() {
                if parent.is_dir() {
                    let info = Self::parse_module_info(parent);
                    let lib_path = parent.join(&info.entry);
                    if lib_path.exists() {
                        return Some(lib_path);
                    }
                }
            }
            // dep may be a directory path stored as resolved entry file already
            if dep.is_dir() {
                let info = Self::parse_module_info(dep);
                let lib_path = dep.join(&info.entry);
                if lib_path.exists() {
                    return Some(lib_path);
                }
            }
        }

        // 2. Try lib_dir first (e.g., next to CLI executable)
        if let Some(ref lib_dir) = self.lib_dir {
            let module_dir = lib_dir.join(name);

            // Check if module directory exists
            if module_dir.is_dir() {
                let info = Self::parse_module_info(&module_dir);
                let lib_path = module_dir.join(&info.entry);
                if lib_path.exists() {
                    return Some(lib_path);
                }
            }

            // Try as direct file
            let mai_path = lib_dir.join(format!("{}.mai", name));
            if mai_path.exists() {
                return Some(mai_path);
            }
        }

        // 3. Try base_dir/libs
        let module_dir = self.base_dir.join("libs").join(name);
        if module_dir.is_dir() {
            let info = Self::parse_module_info(&module_dir);
            let lib_path = module_dir.join(&info.entry);
            if lib_path.exists() {
                return Some(lib_path);
            }
        }

        // 4. Try CWD/libs
        if let Ok(cwd) = std::env::current_dir() {
            let module_dir = cwd.join("libs").join(name);
            if module_dir.is_dir() {
                let info = Self::parse_module_info(&module_dir);
                let lib_path = module_dir.join(&info.entry);
                if lib_path.exists() {
                    return Some(lib_path);
                }
            }
        }

        None
    }

    /// Load a module from a file path
    fn load_from_path(&self, path: &Path) -> Result<Program, ModuleError> {
        let code = std::fs::read_to_string(path).map_err(|e| {
            ModuleError::IoError(format!("Failed to read '{}': {}", path.display(), e))
        })?;

        let mut parser = Parser::new(&code).map_err(|e| ModuleError::ParseError(e.to_string()))?;

        parser
            .parse_program()
            .map_err(|e| ModuleError::ParseError(e.to_string()))
    }

    /// Compile a program to bytecode
    fn compile_program(&self, program: &Program) -> Result<Bytecode, ModuleError> {
        let compiler = Compiler::new();
        compiler
            .compile(program)
            .map_err(|e| ModuleError::CompileError(e.to_string()))
    }
}

impl ModuleLoader for FileModuleLoader {
    fn load(&mut self, path: &str) -> Result<&CompiledModule, ModuleError> {
        // Check if already loaded
        if self.modules.contains_key(path) {
            return Ok(self.modules.get(path).unwrap());
        }

        // Check built-ins
        if self.builtins.contains_key(path) {
            return Ok(self.builtins.get(path).unwrap());
        }

        // Resolve the module path
        let file_path = self
            .resolve_path(path)
            .ok_or_else(|| ModuleError::NotFound(path.to_string()))?;

        // Parse module info if it's in a directory
        let info = if let Some(parent) = file_path.parent() {
            if parent.join("mailib.ini").exists() {
                Self::parse_module_info(parent)
            } else {
                ModuleInfo {
                    name: path.to_string(),
                    ..Default::default()
                }
            }
        } else {
            ModuleInfo {
                name: path.to_string(),
                ..Default::default()
            }
        };

        let program = self.load_from_path(&file_path)?;
        let export_names = extract_exports(&program);
        let bytecode = self.compile_program(&program)?;

        let mut exports = HashMap::new();
        for (i, name) in export_names.iter().enumerate() {
            exports.insert(name.clone(), i as u32);
        }

        let module = CompiledModule {
            name: path.to_string(),
            info,
            bytecode,
            exports,
        };

        self.modules.insert(path.to_string(), module);
        Ok(self.modules.get(path).unwrap())
    }

    fn is_loaded(&self, path: &str) -> bool {
        self.modules.contains_key(path) || self.builtins.contains_key(path)
    }

    fn get(&self, path: &str) -> Option<&CompiledModule> {
        self.modules.get(path).or(self.builtins.get(path))
    }
}

/// Create a pre-configured module loader with default settings
pub fn create_loader(base_dir: impl AsRef<Path>) -> FileModuleLoader {
    let base = base_dir.as_ref().to_path_buf();
    let mut loader = FileModuleLoader::new(&base); // loads mailang.toml deps

    // Try to find lib directory next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let lib_dir = exe_dir.join("libs");
            if lib_dir.is_dir() {
                loader.set_lib_dir(&lib_dir);
            }
        }
    }

    loader
}

#![allow(
    clippy::print_literal,
    clippy::manual_strip,
    clippy::useless_format,
    clippy::uninlined_format_args
)]
use clap::{Parser, Subcommand, ValueEnum};
use mailang_core::{MailangInterpreter, VmBackend};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

mod repl;

#[derive(Parser)]
#[command(
    name = "mailang",
    about = "Ma矛Lang interpreter",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// CLI-facing VM backend selector (`--vm=stack|register`).
#[derive(Clone, Copy, Debug, ValueEnum)]
enum VmCli {
    /// Classic stack-based bytecode VM (default)
    Stack,
    /// Register (three-address) VM 鈥?experimental, keep stack as default
    Register,
}

impl From<VmCli> for VmBackend {
    fn from(v: VmCli) -> Self {
        match v {
            VmCli::Stack => VmBackend::Stack,
            VmCli::Register => VmBackend::Register,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Run a .mai source file or a .mailangbc bytecode file
    Run {
        file: String,
        /// Interpret `file` as compiled bytecode (`.mailangbc`)
        #[arg(long)]
        bytecode: bool,
        /// VM backend: stack (default) or register (experimental)
        #[arg(long, value_enum, default_value_t = VmCli::Stack)]
        vm: VmCli,
    },
    /// Evaluate a code snippet
    Eval {
        code: String,
        /// VM backend: stack (default) or register (experimental)
        #[arg(long, value_enum, default_value_t = VmCli::Stack)]
        vm: VmCli,
    },
    /// Compile a .mai file to .mailangbc bytecode
    Build {
        file: String,
        /// Output path (default: <file> with .mailangbc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Format a .mai source file in place (or print with --check)
    Fmt {
        file: String,
        /// Check formatting without writing; exit 1 if reformatting needed
        #[arg(long)]
        check: bool,
    },
    /// List resolved dependencies from mailang.toml
    Deps {
        /// Project root containing mailang.toml (default: cwd, then walk up)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Print / verify `mailang.lock` status (exit 1 if stale or mismatched)
        #[arg(long)]
        lock: bool,
    },
    /// Plan a dependency into mailang.toml (fetch is host-op; offline plan only)
    Add {
        /// Spec: name=path | github:org/repo | name=github:org/repo | org/repo | name=git:url | git:url
        spec: String,
        /// Project root containing mailang.toml (default: cwd, then walk up)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Publish the current package to a registry
    Publish {
        /// Registry path or http(s) base URL (default: MAILANG_REGISTRY / ./.mailang-registry / ~/.mailang/registry)
        #[arg(long)]
        registry: Option<String>,
        /// Overwrite if name@version is already published
        #[arg(long)]
        allow_republish: bool,
        /// Project root containing mailang.toml (default: cwd, then walk up)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Install a package from a registry into vendor/ (or ~/.mailang/pkg with --global)
    Install {
        /// Package spec: name or name@ver
        spec: String,
        /// Registry path or http(s) base URL
        #[arg(long)]
        registry: Option<String>,
        /// Install into ~/.mailang/pkg/<name> instead of vendor/<name>
        #[arg(long)]
        global: bool,
        /// Allow installing a yanked version
        #[arg(long)]
        allow_yanked: bool,
        /// Do not rewrite mailang.toml [dependencies]
        #[arg(long)]
        no_manifest: bool,
        /// Project root containing mailang.toml (default: cwd, then walk up)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Search the registry index by name / description
    Search {
        /// Substring to match against package names and descriptions
        query: String,
        /// Registry path (filesystem only; http registries cannot walk index/)
        #[arg(long)]
        registry: Option<String>,
    },
    /// Mark a published version as yanked (install refuses unless --allow-yanked)
    Yank {
        /// Package name
        name: String,
        /// Version to yank
        vers: String,
        /// Registry path
        #[arg(long)]
        registry: Option<String>,
        /// Un-yank (restore installability)
        #[arg(long)]
        undo: bool,
    },
    /// Registry utilities
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },
    /// Interactive REPL (multi-line continuation on `{` or `\`)
    Repl,
    /// Run the language server on stdin/stdout
    Lsp,
}

#[derive(Subcommand)]
enum RegistryAction {
    /// Create an empty registry skeleton at <path>
    Init {
        /// Directory (or leave default registry location when omitted)
        path: Option<String>,
    },
}

fn print_result(output: &str) {
    if !output.is_empty() && output != "null" {
        println!("{}", output);
    }
}

fn resolve_root(start: &Path) -> Option<PathBuf> {
    mailang_module::find_project_root(start).or_else(|| {
        if start.join("mailang.toml").is_file() {
            Some(start.to_path_buf())
        } else {
            None
        }
    })
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Lsp) => {
            tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(mailang_lsp::run_lsp());
        }
        Some(Commands::Deps { path, lock }) => {
            let start = path
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            match resolve_root(&start) {
                Some(root) => {
                    if lock {
                        std::process::exit(cmd_deps_lock(&root));
                    }
                    let (resolved, outcome) = mailang_module::resolve_dependencies_locked(&root);
                    if resolved.is_empty() {
                        println!(
                            "mailang.toml found at {} but [dependencies] is empty (or nothing resolved).",
                            root.display()
                        );
                    } else {
                        println!("{:<20} {:<12} {:<18} {}", "NAME", "VERSION", "HASH", "PATH");
                        for dep in &resolved {
                            let version = if dep.version.is_empty() {
                                "-"
                            } else {
                                dep.version.as_str()
                            };
                            let hash = if dep.hash.is_empty() {
                                "-"
                            } else {
                                dep.hash.as_str()
                            };
                            println!(
                                "{:<20} {:<12} {:<18} {}",
                                dep.name,
                                version,
                                hash,
                                dep.path.display()
                            );
                        }
                    }
                    match outcome {
                        mailang_module::LockOutcome::Created => {
                            println!(
                                "mailang.lock: created at {}",
                                root.join("mailang.lock").display()
                            );
                        }
                        mailang_module::LockOutcome::Matched => {
                            println!("mailang.lock: up to date (reused)");
                        }
                        mailang_module::LockOutcome::Refreshed => {
                            println!("mailang.lock: refreshed (content or version changed)");
                        }
                    }
                }
                None => {
                    println!("No mailang.toml found near {}.", start.display());
                    println!(
                        "Hint: create mailang.toml with an optional [dependencies] section mapping name -> path,"
                    );
                    println!("for example:");
                    println!("  [dependencies]");
                    println!("  json = \"libs/json\"");
                    println!("  cool = {{ github = \"org/repo\" }}");
                    println!("See docs/MODULE_SPEC.md for the format.");
                }
            }
        }
        Some(Commands::Add { spec, path }) => {
            let start = path
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            // Prefer existing project root; otherwise use `start` (create mailang.toml there).
            let root = resolve_root(&start).unwrap_or_else(|| start.clone());
            match mailang_module::plan_add(&root, &spec) {
                Ok(plan) => {
                    let kind = match &plan.spec.source {
                        mailang_module::DepSource::Path => "path",
                        mailang_module::DepSource::Github(_) => "github (fetch is host-op)",
                        mailang_module::DepSource::Git(_) => "git (fetch is host-op)",
                    };
                    println!(
                        "planned {} [{}] into {}",
                        plan.spec.toml_line(),
                        kind,
                        plan.manifest_path.display()
                    );
                    if plan.created_manifest {
                        println!("created mailang.toml");
                    }
                    if matches!(
                        plan.spec.source,
                        mailang_module::DepSource::Github(_) | mailang_module::DepSource::Git(_)
                    ) {
                        println!("note: network fetch is left to the host/CI (curl/git).");
                        println!(
                            "offline resolve order: vendor/{}/ then ~/.mailang/pkg/{}/",
                            plan.spec.name, plan.spec.name
                        );
                    }
                    // Refresh lock after planning so path deps settle immediately.
                    let _ = mailang_module::resolve_dependencies_locked(&root);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Publish {
            registry,
            allow_republish,
            path,
        }) => {
            let start = path
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let root = resolve_root(&start).unwrap_or_else(|| start.clone());
            let src = mailang_module::RegistrySource::resolve(registry.as_deref());
            match mailang_module::publish(&src, &root, allow_republish) {
                Ok(res) => {
                    println!(
                        "published {}@{} -> {}",
                        res.entry.name,
                        res.entry.vers,
                        src.display()
                    );
                    println!("  cksum: {}", res.entry.cksum);
                    if !res.entry.description.is_empty() {
                        println!("  desc:  {}", res.entry.description);
                    }
                    println!("  dir:   {}", res.pkg_dir.display());
                    println!("  mpkg:  {}", res.mpkg_path.display());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Install {
            spec,
            registry,
            global,
            allow_yanked,
            no_manifest,
            path,
        }) => {
            let start = path
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let root = resolve_root(&start).or_else(|| {
                if start.is_dir() {
                    Some(start.clone())
                } else {
                    start.parent().map(|p| p.to_path_buf())
                }
            });
            let src = mailang_module::RegistrySource::resolve(registry.as_deref());
            let project = if global { None } else { root.as_deref() };
            if !global && project.is_none() {
                eprintln!(
                    "Error: install needs a project directory (use --path or run inside a project)"
                );
                std::process::exit(1);
            }
            match mailang_module::install(
                &src,
                &spec,
                project,
                global,
                allow_yanked,
                !no_manifest && !global,
            ) {
                Ok(res) => {
                    println!(
                        "installed {}@{} from {}",
                        res.entry.name,
                        res.entry.vers,
                        src.display()
                    );
                    println!("  dest: {}", res.dest.display());
                    if let Some(m) = res.manifest_path {
                        println!("  manifest: {}", m.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Search { query, registry }) => {
            let src = mailang_module::RegistrySource::resolve(registry.as_deref());
            match mailang_module::search(&src, &query) {
                Ok(hits) => {
                    if hits.is_empty() {
                        println!("no packages matching '{}'", query);
                    } else {
                        println!(
                            "{:<20} {:<12} {:<8} {}",
                            "NAME", "VERSION", "YANKED", "DESCRIPTION"
                        );
                        for e in hits {
                            println!(
                                "{:<20} {:<12} {:<8} {}",
                                e.name,
                                e.vers,
                                if e.yanked { "yes" } else { "no" },
                                e.description
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Yank {
            name,
            vers,
            registry,
            undo,
        }) => {
            let src = mailang_module::RegistrySource::resolve(registry.as_deref());
            match mailang_module::set_yanked(&src, &name, &vers, !undo) {
                Ok(entry) => {
                    if undo {
                        println!("un-yanked {}@{}", entry.name, entry.vers);
                    } else {
                        println!(
                            "yanked {}@{} (install refuses unless --allow-yanked)",
                            entry.name, entry.vers
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Registry { action }) => match action {
            RegistryAction::Init { path } => {
                let target = path.unwrap_or_else(|| {
                    // Default skeleton location when not specified.
                    std::env::var("MAILANG_REGISTRY").unwrap_or_else(|_| {
                        let local = PathBuf::from(".mailang-registry");
                        if local.is_dir() {
                            local.display().to_string()
                        } else if let Some(home) = mailang_module::home_dir() {
                            home.join(".mailang").join("registry").display().to_string()
                        } else {
                            ".mailang-registry".to_string()
                        }
                    })
                });
                match mailang_module::registry_init(&target) {
                    Ok(root) => {
                        println!("registry initialized at {}", root.display());
                        println!("  index/  package index (JSON lines)");
                        println!("  pkgs/   package artifacts (dir + .mpkg)");
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(Commands::Fmt { file, check }) => {
            let src = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: failed to read '{}': {}", file, e);
                    std::process::exit(1);
                }
            };
            match mailang_core::format_source(&src) {
                Ok(formatted) => {
                    if check {
                        if formatted != src {
                            eprintln!("{}: needs formatting", file);
                            std::process::exit(1);
                        }
                    } else if let Err(e) = std::fs::write(&file, &formatted) {
                        eprintln!("Error: failed to write '{}': {}", file, e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: cannot format '{}': {}", file, e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Run {
            file,
            bytecode: use_bytecode,
            vm,
        }) => {
            let base_dir = Path::new(&file).parent().unwrap_or(Path::new("."));
            let mut interp = MailangInterpreter::with_modules(base_dir);
            interp.set_vm_backend(vm.into());

            let is_bc = use_bytecode
                || Path::new(&file)
                    .extension()
                    .map(|e| e == "mailangbc")
                    .unwrap_or(false);

            let result = if is_bc {
                let bytes = std::fs::read(&file)
                    .map_err(|e| format!("Failed to read bytecode file '{}': {}", file, e));
                match bytes {
                    Ok(bytes) => match mailang_core::bytecode::decode(&bytes) {
                        Ok(bc) => interp.run_bytecode(bc),
                        Err(e) => Err(format!("Invalid bytecode file '{}': {:?}", file, e)),
                    },
                    Err(e) => Err(e),
                }
            } else {
                interp.eval_file(&file)
            };

            match result {
                Ok(output) => print_result(&output),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Eval { code, vm }) => {
            let mut interp = MailangInterpreter::new();
            interp.set_vm_backend(vm.into());
            match interp.eval(&code) {
                Ok(output) => print_result(&output),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Build { file, output }) => {
            let base_dir = Path::new(&file).parent().unwrap_or(Path::new("."));
            let mut interp = MailangInterpreter::with_modules(base_dir);

            let out_path = output.unwrap_or_else(|| {
                let mut p = PathBuf::from(&file);
                p.set_extension("mailangbc");
                p
            });

            match interp.compile_file(&file) {
                Ok(bc) => {
                    let bytes = mailang_core::bytecode::encode(&bc);
                    if let Err(e) = std::fs::write(&out_path, &bytes) {
                        eprintln!("Error: failed to write '{}': {}", out_path.display(), e);
                        std::process::exit(1);
                    }
                    println!(
                        "Wrote {} ({} bytes, {} chunks)",
                        out_path.display(),
                        bytes.len(),
                        bc.chunks.len()
                    );
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Repl) | None => {
            let mut interp = MailangInterpreter::new();
            run_repl(&mut interp);
        }
    }
}

/// `mailang deps --lock` 鈥?print/verify lock status. Exit 1 when unhealthy.
fn cmd_deps_lock(root: &Path) -> i32 {
    let resolved = mailang_module::resolve_dependencies_raw(root);
    let report = mailang_module::verify_lock(root, &resolved);
    println!("lock: {}", report.lock_path.display());
    if !report.exists {
        println!("status: missing (run `mailang deps` to create)");
    }
    println!(
        "{:<20} {:<12} {:<18} {:<16} {}",
        "NAME", "VERSION", "HASH", "STATUS", "PATH"
    );
    for e in &report.entries {
        let version = if e.version.is_empty() {
            "-"
        } else {
            e.version.as_str()
        };
        let hash = if e.hash.is_empty() {
            "-"
        } else {
            e.hash.as_str()
        };
        let status = match e.status {
            mailang_module::EntryLockStatus::Ok => "ok",
            mailang_module::EntryLockStatus::Planned => "planned",
            mailang_module::EntryLockStatus::VersionMismatch => "version-mismatch",
            mailang_module::EntryLockStatus::PathMismatch => "path-mismatch",
            mailang_module::EntryLockStatus::HashMismatch => "hash-mismatch",
            mailang_module::EntryLockStatus::MissingInLock => "missing-in-lock",
            mailang_module::EntryLockStatus::MissingOnDisk => "missing-on-disk",
            mailang_module::EntryLockStatus::ExtraInLock => "extra-in-lock",
        };
        println!(
            "{:<20} {:<12} {:<18} {:<16} {}",
            e.name, version, hash, status, e.path
        );
    }
    if report.is_healthy() {
        println!("mailang.lock: verified ({} entries)", report.entries.len());
        0
    } else {
        println!("mailang.lock: STALE or mismatched (run `mailang deps` to refresh)");
        1
    }
}

fn run_repl(interp: &mut MailangInterpreter) {
    println!("Ma矛Lang REPL v{}", env!("CARGO_PKG_VERSION"));
    println!("Type 'exit' or 'quit' to exit. Multi-line: end a line with `{{` or `\\`.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::new();

    loop {
        let prompt = if buf.is_empty() { "> " } else { "... " };
        print!("{}", prompt);
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let line = line.trim_end_matches(['\r', '\n']);
                if buf.is_empty() {
                    let t = line.trim();
                    if t == "exit" || t == "quit" {
                        break;
                    }
                    if t.is_empty() {
                        continue;
                    }
                }
                buf = repl::append_line(&buf, line);
                if repl::needs_continuation(&buf) {
                    continue;
                }
                let code = std::mem::take(&mut buf);
                let code = code.trim();
                if code.is_empty() {
                    continue;
                }
                match interp.eval(code) {
                    Ok(output) => print_result(&output),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}

use clap::{Parser, Subcommand};
use mailang_core::MailangInterpreter;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "mailang", about = "MaìLang interpreter")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a .mai source file or a .mailangbc bytecode file
    Run {
        file: String,
        /// Interpret `file` as compiled bytecode (`.mailangbc`)
        #[arg(long)]
        bytecode: bool,
    },
    /// Evaluate a code snippet
    Eval { code: String },
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
    /// Run the language server on stdin/stdout
    Lsp,
}

fn print_result(output: &str) {
    if !output.is_empty() && output != "null" {
        println!("{}", output);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Lsp) => {
            tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(mailang_lsp::run_lsp());
        }
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
        }) => {
            let base_dir = Path::new(&file)
                .parent()
                .unwrap_or(Path::new("."));
            let mut interp = MailangInterpreter::with_modules(base_dir);

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
        Some(Commands::Eval { code }) => {
            let mut interp = MailangInterpreter::new();
            match interp.eval(&code) {
                Ok(output) => print_result(&output),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Build { file, output }) => {
            let base_dir = Path::new(&file)
                .parent()
                .unwrap_or(Path::new("."));
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
        None => {
            let mut interp = MailangInterpreter::new();
            run_repl(&mut interp);
        }
    }
}

fn run_repl(interp: &mut MailangInterpreter) {
    println!("MaìLang REPL v0.1.0");
    println!("Type 'exit' or 'quit' to exit.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let line = line.trim();
                if line == "exit" || line == "quit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                match interp.eval(line) {
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

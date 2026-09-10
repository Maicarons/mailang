use clap::{Parser, Subcommand};
use mailang_core::MailangInterpreter;
use std::io::{self, BufRead, Write};

#[derive(Parser)]
#[command(name = "mailang", about = "MaìLang interpreter")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Run { file: String },
    Eval { code: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => {
            // Use module-aware interpreter for file execution
            let base_dir = std::path::Path::new(&file)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            let mut interp = MailangInterpreter::with_modules(base_dir);

            match interp.eval_file(&file) {
                Ok(output) => {
                    if !output.is_empty() && output != "null" {
                        println!("{}", output);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Eval { code }) => {
            let mut interp = MailangInterpreter::new();
            match interp.eval(&code) {
                Ok(output) => {
                    if !output.is_empty() && output != "null" {
                        println!("{}", output);
                    }
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
                    Ok(output) => {
                        if !output.is_empty() && output != "null" {
                            println!("{}", output);
                        }
                    }
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

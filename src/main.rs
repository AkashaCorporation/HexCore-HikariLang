use std::fs;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use hikari_lang::lexer::tokenize;
use hikari_lang::parser::parse_program;
use hikari_lang::types::typechecker::TypeChecker;
use hikari_lang::engine::interpreter::Interpreter;

#[derive(Parser)]
#[command(name = "hkl")]
#[command(about = "HikariScript — binary analysis workflow language for HexCore")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an HKL pipeline
    Run {
        /// Path to the .hkl file
        file: PathBuf,

        /// Path to the binary to analyze (reserved for IDE/Decompiler plug-in)
        #[arg(short, long)]
        binary: Option<PathBuf>,

        /// Session name override
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Type-check an HKL file without running it
    Check {
        /// Path to the .hkl file
        file: PathBuf,
    },

    /// Print the AST of an HKL file
    Ast {
        /// Path to the .hkl file
        file: PathBuf,
    },

    /// Format an HKL file
    Fmt {
        /// Path to the .hkl file
        file: PathBuf,
    },

    /// Export HKL patterns to other formats
    Export {
        /// Path to the .hkl file
        file: PathBuf,

        /// Export format (yara, sigma)
        #[arg(short, long, default_value = "yara")]
        format: String,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            binary,
            session,
        } => {
            run_pipeline(&file, binary, session)?;
        }
        Commands::Check { file } => {
            check_file(&file)?;
        }
        Commands::Ast { file } => {
            print_ast(&file)?;
        }
        Commands::Fmt { file } => {
            format_file(&file)?;
        }
        Commands::Export { file, format } => {
            export_file(&file, &format)?;
        }
    }

    Ok(())
}

fn load_program(file: &PathBuf) -> Result<hikari_lang::parser::ast::Program, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(file)?;
    let tokens = tokenize(&source)?;
    let program = parse_program(&tokens)?;
    Ok(program)
}

fn run_pipeline(
    file: &PathBuf,
    binary: Option<PathBuf>,
    session: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(b) = &binary {
        println!("note: binary path {:?} reserved for future plug-in", b);
    }
    if let Some(s) = &session {
        println!("note: session override {:?} (applied when runtime supports it)", s);
    }

    let program = load_program(file)?;
    println!("Running pipeline from {}...", file.display());

    let mut interpreter = Interpreter::new();
    let result = interpreter.execute_program(&program)?;

    println!("Pipeline completed successfully.");
    println!("Result: {}", result);
    Ok(())
}

fn check_file(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let program = load_program(file)?;

    let mut checker = TypeChecker::new();
    match checker.check_program(&program) {
        Ok(()) => {
            println!("Type check passed for {}", file.display());
            Ok(())
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("Error: {}", error);
            }
            Err("type check failed".into())
        }
    }
}

fn print_ast(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let program = load_program(file)?;
    println!("{:#?}", program);
    Ok(())
}

fn format_file(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Load to validate; pretty-printer still TODO for IDE team
    let _program = load_program(file)?;
    eprintln!("Format not yet implemented (file parses OK)");
    Ok(())
}

fn export_file(file: &PathBuf, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _program = load_program(file)?;
    eprintln!("Export to {} not yet implemented (file parses OK)", format);
    Ok(())
}

use clap::{CommandFactory, Parser, Subcommand};
use skills_primer::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "skill-primer")]
#[command(about = "Print skill loading instructions and skill catalog")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Include skills from directory (repeatable)
    #[arg(long = "include", short = 'i', global = true)]
    include_dirs: Vec<PathBuf>,
}

#[derive(Subcommand, PartialEq)]
enum Command {
    /// Print skill loading instructions and skill catalog. Use to 'prime' a coding agent.
    Prime,
}

fn main() {
    let cli = Cli::parse();
    match (&cli.command, cli.include_dirs.is_empty()) {
        (Some(Command::Prime), _) | (None, false) => handle_prime(&cli.include_dirs),
        _ => {
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!();
        }
    }
}

fn handle_prime(include_dirs: &[PathBuf]) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match generate_prime_output(include_dirs, &cwd) {
        Ok(output) => {
            print!("{}", output.instructions);
            if !output.warnings.is_empty() {
                eprintln!("{}", output.warnings.join("\n"));
            }
        }
        Err(errors) => {
            for line in &errors {
                eprintln!("{}", line);
            }
            std::process::exit(1);
        }
    }
}

use clap::{CommandFactory, Parser, Subcommand};
use skills_primer::*;
use std::path::{Path, PathBuf};

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
    /// Show the CLI configuration
    ShowConfig,
    /// List available skills
    Ls,
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match (&cli.command, cli.include_dirs.is_empty()) {
        (Some(Command::Prime), _) => handle_prime(&cli.include_dirs),
        (Some(Command::ShowConfig), _) => handle_show_config(&cli.include_dirs, &cwd),
        (Some(Command::Ls), _) => handle_ls(&cli.include_dirs),
        (None, false) => {
            eprintln!("error: a subcommand is required when using --include");
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!();
            std::process::exit(1);
        }
        _ => {
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!();
        }
    }
}

fn handle_show_config(include_dirs: &[PathBuf], cwd: &Path) {
    match generate_show_config_response(include_dirs, cwd) {
        Ok(output) => {
            for line in &output.search_paths {
                println!("{}", line);
            }
        }
        Err(errors) => {
            for line in errors {
                eprintln!("{}", line);
            }
            std::process::exit(1);
        }
    }
}

fn handle_ls(include_dirs: &[PathBuf]) {
    match generate_ls_output(include_dirs) {
        Ok(output) => {
            for line in &output.skill_paths {
                println!("{}", line);
            }
            for line in &output.stderr {
                eprintln!("{}", line);
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

fn handle_prime(include_dirs: &[PathBuf]) {
    match generate_prime_output(include_dirs) {
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

use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use skills_primer::*;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "skill-primer")]
#[command(about = "Print skill loading instructions and skill catalog")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Relative skill directory path to scan while walking upward
    #[arg(long = "path", value_name = "PATH", global = true)]
    paths: Vec<PathBuf>,

    /// Enable warnings, including for commands that suppress them by default.
    #[arg(
        long = "warnings",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "no_warnings"
    )]
    warnings: bool,

    /// Suppress warnings, including for commands that show them by default.
    #[arg(
        long = "no-warnings",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "warnings"
    )]
    no_warnings: bool,
}

#[derive(Subcommand, Copy, Clone, PartialEq, Eq)]
enum Command {
    /// Print skill loading instructions and skill catalog. Use to 'prime' a coding agent.
    Prime,
    /// Show the CLI configuration
    Config,
    /// List available skills
    Ls,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum WarningMode {
    Show,
    Suppress,
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let warning_mode = cli.warning_mode();
    match (&cli.command, cli.paths.is_empty()) {
        (Some(Command::Prime), _) => handle_prime(&cli.paths, &cwd, warning_mode),
        (Some(Command::Config), _) => handle_config(&cli.paths, &cwd),
        (Some(Command::Ls), _) => handle_ls(&cli.paths, &cwd, warning_mode),
        (None, false) => {
            eprintln!("error: a subcommand is required when using --path");
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

impl Cli {
    fn warning_mode(&self) -> WarningMode {
        if self.warnings {
            WarningMode::Show
        } else if self.no_warnings {
            WarningMode::Suppress
        } else {
            match self.command {
                Some(Command::Prime) | None => WarningMode::Suppress,
                Some(Command::Config) | Some(Command::Ls) => WarningMode::Show,
            }
        }
    }
}

fn handle_config(paths: &[PathBuf], cwd: &Path) {
    match generate_config_output(paths, cwd) {
        Ok(output) => {
            for line in &output.lines {
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

fn handle_ls(paths: &[PathBuf], cwd: &Path, warning_mode: WarningMode) {
    match generate_ls_output(paths, cwd) {
        Ok(output) => {
            for line in &output.skill_paths {
                println!("{}", line);
            }
            emit_stderr_lines(&output.stderr, warning_mode);
        }
        Err(errors) => {
            for line in &errors {
                eprintln!("{}", line);
            }
            std::process::exit(1);
        }
    }
}

fn handle_prime(paths: &[PathBuf], cwd: &Path, warning_mode: WarningMode) {
    match generate_prime_output(paths, cwd) {
        Ok(output) => {
            print!("{}", output.instructions);
            emit_stderr_lines(&output.warnings, warning_mode);
        }
        Err(errors) => {
            for line in &errors {
                eprintln!("{}", line);
            }
            std::process::exit(1);
        }
    }
}

fn emit_stderr_lines(lines: &[String], warning_mode: WarningMode) {
    if warning_mode == WarningMode::Suppress {
        return;
    }
    for line in lines {
        eprintln!("{}", line);
    }
}

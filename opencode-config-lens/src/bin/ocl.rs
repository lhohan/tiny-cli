use std::path::PathBuf;

use opencode_config_lens::runtime::{run, Cli};

const NAME: &str = "ocl";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = "OpenCode Config Lens TUI";

fn print_help() {
    println!("{}", ABOUT);
    println!();
    println!("Usage: {} [OPTIONS]", NAME);
    println!();
    println!("Options:");
    println!("      --home-dir <PATH>  Override config home directory");
    println!("  -h, --help             Print help");
    println!("  -V, --version          Print version");
}

fn print_version() {
    println!("{} {}", NAME, VERSION);
}

fn parse_args() -> Result<Option<PathBuf>, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut home_dir: Option<PathBuf> = None;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                print_version();
                std::process::exit(0);
            }
            "--home-dir" => {
                i += 1;
                if i >= args.len() {
                    return Err("--home-dir requires a value".to_string());
                }
                home_dir = Some(PathBuf::from(&args[i]));
            }
            arg if arg.starts_with("--home-dir=") => {
                let value = &arg[11..]; // len("--home-dir=") == 11
                if value.is_empty() {
                    return Err("--home-dir requires a value".to_string());
                }
                home_dir = Some(PathBuf::from(value));
            }
            arg => {
                return Err(format!("unknown option: {}", arg));
            }
        }
        i += 1;
    }

    Ok(home_dir)
}

fn main() {
    let home_dir = match parse_args() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("ERROR: {}", err);
            std::process::exit(1);
        }
    };

    if let Err(err) = run(Cli { home_dir }) {
        eprintln!("ERROR: {}", err);
        std::process::exit(err.exit_code());
    }
}

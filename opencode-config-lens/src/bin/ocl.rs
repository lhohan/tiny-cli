use std::path::PathBuf;

use clap::Parser;

use opencode_config_lens::runtime::{run, Cli};

#[derive(Debug, Parser)]
#[command(name = "ocl")]
#[command(about = "OpenCode Config Lens TUI")]
#[command(version)]
struct Args {
    /// Override config home directory
    #[arg(long = "home-dir")]
    home_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(Cli {
        home_dir: args.home_dir,
    }) {
        eprintln!("ERROR: {}", err);
        std::process::exit(err.exit_code());
    }
}

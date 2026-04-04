use std::path::PathBuf;

use clap::Parser;

use opencode_model_report::v2::runtime::{run, Cli};

#[derive(Debug, Parser)]
#[command(name = "opencode-model-report")]
#[command(about = "Report OpenCode model usage and costs in a fullscreen TUI")]
#[command(version)]
struct Args {
    /// Disable color output
    #[arg(long)]
    no_color: bool,

    /// Override config home directory
    #[arg(long = "home-dir")]
    home_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(Cli {
        no_color: args.no_color,
        home_dir: args.home_dir,
    }) {
        eprintln!("ERROR: {}", err);
        std::process::exit(err.exit_code());
    }
}

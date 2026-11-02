use std::process::ExitCode;
use std::time::Duration;

use usage_watch::model::Report;
use usage_watch::{args, http::Http, providers, render};

fn main() -> ExitCode {
    match args::parse(std::env::args().skip(1)) {
        args::Parsed::Help => {
            print!("{}", help_text());
            ExitCode::SUCCESS
        }
        args::Parsed::Version => {
            println!("usage-watch {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        args::Parsed::Error(e) => {
            eprintln!("error: {e}\n\n{}", help_text());
            ExitCode::from(2)
        }
        args::Parsed::Run(a) => run(a),
    }
}

fn help_text() -> String {
    let mut s = String::from(
        "usage-watch — show remaining usage for AI subscriptions\n\n\
         Usage: usage-watch [OPTIONS]\n\n\
         Options:\n\
         \x20 -p, --provider <ID>   only this provider (repeatable)\n\
         \x20     --json            machine-readable JSON output\n\
         \x20 -t, --timeout <SECS> per-request timeout in seconds (default 15)\n\
         \x20 -h, --help            show this help\n\
         \x20 -V, --version         show version\n\n\
         Providers are enabled by environment variables:\n",
    );
    for spec in providers::all() {
        s.push_str(&format!("  {:<12} {}\n", spec.id, spec.env_hint));
    }
    s.push_str(
        "\nExit codes: 0 ok, 1 a provider request failed, 2 bad arguments or nothing configured.\n",
    );
    s
}

fn run(a: args::Args) -> ExitCode {
    let specs = providers::all();
    let selected: Vec<providers::ProviderSpec> = if a.providers.is_empty() {
        specs.into_iter().filter(|s| (s.is_configured)()).collect()
    } else {
        let mut out = Vec::new();
        for name in &a.providers {
            match specs.iter().find(|s| s.id == name) {
                Some(s) => out.push(*s),
                None => {
                    eprintln!("error: unknown provider '{name}'");
                    eprintln!("\n{}", help_text());
                    return ExitCode::from(2);
                }
            }
        }
        out
    };

    if selected.is_empty() {
        eprintln!("no providers configured. Set one of:");
        for s in providers::all() {
            eprintln!("  {:<12} {}", s.id, s.env_hint);
        }
        return ExitCode::from(2);
    }

    let http = Http::new(Duration::from_secs(a.timeout_secs));
    let mut reports: Vec<Report> = Vec::new();
    let mut failed = false;
    for s in selected {
        if !a.providers.is_empty() && !(s.is_configured)() {
            reports.push(Report::err(
                s.id,
                s.title,
                format!("not configured (set {})", s.env_hint),
            ));
            failed = true;
            continue;
        }
        match (s.fetch)(&http) {
            Ok(r) => reports.push(r),
            Err(e) => {
                reports.push(Report::err(s.id, s.title, e));
                failed = true;
            }
        }
    }

    if a.json {
        println!("{}", render::render_json(&reports));
    } else {
        print!("{}", render::render_text(&reports));
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

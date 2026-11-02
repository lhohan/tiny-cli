//! Command-line argument parsing (hand-rolled to keep dependencies minimal).

pub struct Args {
    pub providers: Vec<String>,
    pub json: bool,
    pub timeout_secs: u64,
}

pub enum Parsed {
    Run(Args),
    Help,
    Version,
    Error(String),
}

pub const DEFAULT_TIMEOUT_SECS: u64 = 15;

pub fn parse<I: Iterator<Item = String>>(mut it: I) -> Parsed {
    let mut providers = Vec::new();
    let mut json = false;
    let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => return Parsed::Help,
            "-V" | "--version" => return Parsed::Version,
            "--json" => json = true,
            "-p" | "--provider" => match it.next() {
                Some(v) if !v.is_empty() => providers.push(v),
                _ => return Parsed::Error("--provider needs a value".to_string()),
            },
            "-t" | "--timeout" => match it.next().and_then(|v| v.parse::<u64>().ok()) {
                Some(v) if v > 0 => timeout_secs = v,
                _ => {
                    return Parsed::Error(
                        "--timeout needs a positive number of seconds".to_string(),
                    )
                }
            },
            _ if arg.starts_with("--provider=") => {
                let v = &arg["--provider=".len()..];
                if v.is_empty() {
                    return Parsed::Error("--provider needs a value".to_string());
                }
                providers.push(v.to_string());
            }
            _ if arg.starts_with("--timeout=") => {
                let v = &arg["--timeout=".len()..];
                match v.parse::<u64>() {
                    Ok(n) if n > 0 => timeout_secs = n,
                    _ => {
                        return Parsed::Error(
                            "--timeout needs a positive number of seconds".to_string(),
                        )
                    }
                }
            }
            _ => return Parsed::Error(format!("unknown argument: {arg}")),
        }
    }
    Parsed::Run(Args {
        providers,
        json,
        timeout_secs,
    })
}

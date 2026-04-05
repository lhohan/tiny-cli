use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::LoadError;

pub fn fetch_available_models() -> Result<Vec<String>, LoadError> {
    let output = Command::new("opencode")
        .args(["models", "--refresh"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                LoadError::OpenCodeNotFound
            } else {
                LoadError::RefreshFailed {
                    stderr: err.to_string(),
                    code: 4,
                }
            }
        })?;

    if !output.status.success() {
        return Err(LoadError::RefreshFailed {
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            code: output.status.code().unwrap_or(4),
        });
    }

    Ok(extract_available_models(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub fn extract_available_models(stdout: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for raw_line in stdout.lines() {
        let line = crate::report::strip_ansi(raw_line).trim().to_string();
        if line.is_empty() || line == "Models cache refreshed" {
            continue;
        }
        if line.contains(' ') || !line.contains('/') {
            continue;
        }
        if seen.insert(line.clone()) {
            models.push(line);
        }
    }

    models
}

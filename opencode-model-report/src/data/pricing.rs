use std::collections::HashMap;
use std::process::{Command, Stdio};

use crate::LoadError;

pub fn fetch_costs() -> Result<Vec<(String, Option<f64>, Option<f64>)>, LoadError> {
    let output = Command::new("curl")
        .args(["-fsSL", "https://models.dev/api.json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                LoadError::CurlNotFound
            } else {
                LoadError::FetchFailed(err.to_string())
            }
        })?;

    if !output.status.success() {
        return Err(LoadError::FetchFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let costs = parse_costs_from_api_json(&String::from_utf8_lossy(&output.stdout))
        .map_err(|err| LoadError::FetchFailed(err.to_string()))?;

    Ok(costs
        .into_iter()
        .map(|(model, (input_cost, output_cost))| (model, input_cost, output_cost))
        .collect())
}

pub fn parse_costs_from_api_json(
    text: &str,
) -> serde_json::Result<HashMap<String, (Option<f64>, Option<f64>)>> {
    #[derive(Debug, serde::Deserialize)]
    struct Provider {
        #[serde(default)]
        models: HashMap<String, serde_json::Value>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct CostWire {
        #[serde(default)]
        input: Option<f64>,
        #[serde(default)]
        output: Option<f64>,
    }

    let api: HashMap<String, Provider> = serde_json::from_str(text)?;
    let mut costs = HashMap::new();

    for (provider_id, provider) in api {
        for (model_id, model_data) in provider.models {
            let key = format!("{}/{}", provider_id, model_id);
            let cost = model_data
                .get("cost")
                .and_then(|value| serde_json::from_value::<CostWire>(value.clone()).ok())
                .map(|cost| (cost.input, cost.output))
                .unwrap_or((None, None));
            costs.insert(key, cost);
        }
    }

    Ok(costs)
}

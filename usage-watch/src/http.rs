//! Minimal blocking HTTP GET client over ureq with a global timeout.

use std::time::Duration;

pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    pub fn new(timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .user_agent(concat!("usage-watch/", env!("CARGO_PKG_VERSION")))
            .build();
        Http {
            agent: config.new_agent(),
        }
    }

    /// GET `url` with extra headers. Every HTTP response is returned as
    /// `(status, body)`; only transport failures are errors.
    pub fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<(u16, String), String> {
        let mut req = self.agent.get(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        match req.call() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                match resp.into_body().read_to_string() {
                    Ok(body) => Ok((status, body)),
                    Err(e) => Err(format!("read response body: {e}")),
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// GET and parse the body as JSON. Non-2xx responses include a body snippet.
    pub fn get_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<serde_json::Value, String> {
        let (status, body) = self.get(url, headers)?;
        if !(200..300).contains(&status) {
            return Err(format!("HTTP {status}: {}", truncate(&body, 400)));
        }
        serde_json::from_str(&body).map_err(|e| format!("invalid JSON from {url}: {e}"))
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

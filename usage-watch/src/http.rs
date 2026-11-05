//! Minimal blocking HTTP GET client over ureq with a global timeout.

use std::time::Duration;

pub struct Http {
    agent: ureq::Agent,
}

/// One HTTP response.
///
/// Redirects are not followed. None of these endpoints redirects
/// legitimately, and a redirect means the request was bounced to the login
/// page, which is worth reporting as-is instead of chasing the auth loop.
pub struct Response {
    pub status: u16,
    pub location: Option<String>,
    pub body: String,
}

impl Http {
    pub fn new(timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .max_redirects(0)
            .user_agent(concat!("usage-watch/", env!("CARGO_PKG_VERSION")))
            .build();
        Http {
            agent: config.new_agent(),
        }
    }

    /// GET `url` with extra headers. Every HTTP response is returned; only
    /// transport failures are errors.
    pub fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
        let mut req = self.agent.get(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        match req.call() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let location = resp
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                match resp.into_body().read_to_string() {
                    Ok(body) => Ok(Response {
                        status,
                        location,
                        body,
                    }),
                    Err(e) => Err(format!("read response body: {e}")),
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// GET and parse the body as JSON. Non-2xx responses are errors; a
    /// redirect reports its target, which is usually the login page.
    pub fn get_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<serde_json::Value, String> {
        let resp = self.get(url, headers)?;
        if !(200..300).contains(&resp.status) {
            return Err(match &resp.location {
                Some(loc) => format!(
                    "HTTP {} → {} (session cookie not accepted)",
                    resp.status,
                    redirect_target(loc)
                ),
                None => format!("HTTP {}: {}", resp.status, truncate(&resp.body, 400)),
            });
        }
        serde_json::from_str(&resp.body).map_err(|e| format!("invalid JSON from {url}: {e}"))
    }
}

/// Drop the query and fragment from a redirect target, which for these
/// endpoints is a login URL carrying a long `return_to` parameter.
fn redirect_target(location: &str) -> &str {
    location.split(['?', '#']).next().unwrap_or(location)
}

pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

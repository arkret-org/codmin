/// Shape of `/config.json` served by the deployment. Only the coauth
/// public URL is currently consumed; `pages::login` writes it into
/// storage on app boot so `utils::net::session::coauth_public_url` can read
/// it back from any layer.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub coauth_public_url: String,
    /// Optional opt-in browser-error telemetry endpoint. When set
    /// AND the operator has flipped `codmin_telemetry_opt_in` in
    /// localStorage, `utils::net::telemetry::report_http_error` posts
    /// structured (no-PII) error events here.
    #[serde(default)]
    pub telemetry_endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_config_rejects_misspelled_security_relevant_keys() {
        let error = serde_json::from_value::<RuntimeConfig>(serde_json::json!({
            "coauth_public_url": "https://auth.example",
            "telemetery_endpoint": "https://collector.example"
        }))
        .expect_err("unknown deployment keys must not be silently ignored");
        assert!(error.to_string().contains("telemetery_endpoint"));
    }
}

/// Why `/config.json` could not be turned into a usable runtime config.
/// Surfaced verbatim by the login page so deployment operators see the
/// concrete failure (HTTP, JSON shape, missing field) instead of an
/// empty form that silently fails on the OAuth redirect.
#[derive(Debug, Clone)]
pub enum ConfigLoadError {
    /// `/config.json` did not respond (network error, 404, 5xx).
    Fetch(String),
    /// The response body was not valid JSON or missed required fields.
    Parse(String),
    /// The required `coauth_public_url` field was absent or empty.
    MissingCoauthUrl,
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch(msg) => write!(f, "Failed to fetch /config.json: {msg}"),
            Self::Parse(msg) => write!(f, "Failed to parse /config.json: {msg}"),
            Self::MissingCoauthUrl => {
                f.write_str("/config.json is missing the required `coauth_public_url` field")
            }
        }
    }
}

pub async fn load_runtime_config() -> Result<RuntimeConfig, ConfigLoadError> {
    let response = gloo_net::http::Request::get("/config.json")
        .send()
        .await
        .map_err(|e| ConfigLoadError::Fetch(e.to_string()))?;

    if !response.ok() {
        return Err(ConfigLoadError::Fetch(format!(
            "HTTP {}",
            response.status()
        )));
    }

    let cfg = response
        .json::<RuntimeConfig>()
        .await
        .map_err(|e| ConfigLoadError::Parse(e.to_string()))?;

    if cfg.coauth_public_url.trim().is_empty() {
        return Err(ConfigLoadError::MissingCoauthUrl);
    }

    Ok(cfg)
}

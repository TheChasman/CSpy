use serde::Deserialize;
use std::process::Command;

/// Credential blob stored by Claude Code in macOS Keychain.
#[derive(Debug, Deserialize)]
pub struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<OAuthCreds>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCreds {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,
}

/// Read the Claude Code OAuth token from macOS Keychain.
///
/// Claude Code stores credentials under the service name
/// "Claude Code-credentials" via `security` / Keychain API.
pub fn get_oauth_token() -> Result<String, String> {
    // Use macOS `security` CLI — most reliable for generic-password items
    // where the account field may be empty or unconventional.
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .map_err(|e| format!("Failed to run `security`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Keychain lookup failed. Is Claude Code installed and logged in? ({stderr})"
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let creds: ClaudeCredentials =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse credentials: {e}"))?;

    let oauth = creds
        .claude_ai_oauth
        .ok_or("No claudeAiOauth field in credentials")?;

    if oauth.access_token.is_empty() {
        return Err("OAuth access token is empty".into());
    }

    log::info!(
        "Keychain: got token for subscription type {:?}",
        oauth.subscription_type.as_deref().unwrap_or("unknown")
    );

    Ok(oauth.access_token)
}

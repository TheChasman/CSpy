use serde::Deserialize;
use std::process::Command;
use std::path::{Path, PathBuf};

/// Credential blob stored by Claude Code in macOS Keychain.
#[derive(Deserialize)]
pub struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: Option<OAuthCreds>,
}

#[derive(Deserialize)]
pub struct OAuthCreds {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<i64>, // millisecond Unix timestamp
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,
}

// Redacted Debug — never print tokens to logs
impl std::fmt::Debug for OAuthCreds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthCreds")
            .field("access_token", &"[REDACTED]")
            .field("subscription_type", &self.subscription_type)
            .finish()
    }
}

impl std::fmt::Debug for ClaudeCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCredentials")
            .field("claude_ai_oauth", &self.claude_ai_oauth)
            .finish()
    }
}

/// Token with optional expiry (millisecond Unix timestamp).
pub struct TokenInfo {
    pub token: String,
    /// `None` for token-file tokens (no expiry known).
    pub expires_at_ms: Option<i64>,
}

#[derive(Deserialize)]
struct KeysFile {
    anthropic: Option<AnthropicKeys>,
}

#[derive(Deserialize)]
struct AnthropicKeys {
    oauth_token: Option<String>,
}

/// Read the Claude OAuth token.
///
/// Sources checked in order:
/// 1. Named key in `~/.config/cspy/keys.json` (`anthropic.oauth_token`)
/// 2. Legacy token file at `~/.config/cspy/token`
/// 3. macOS Keychain — "Claude Code-credentials" (automatic if Claude Code is installed)
pub fn get_oauth_token() -> Result<TokenInfo, String> {
    // Source 1: named keys file (no expiry info available)
    if let Some(token) = read_anthropic_token_from_keys_file() {
        log::info!("Token loaded from ~/.config/cspy/keys.json (anthropic.oauth_token)");
        return Ok(TokenInfo { token, expires_at_ms: None });
    }

    // Source 2: legacy token file (no expiry info available)
    if let Some(token) = read_token_file() {
        log::info!("Token loaded from ~/.config/cspy/token");
        return Ok(TokenInfo { token, expires_at_ms: None });
    }

    // Source 3: macOS Keychain (Claude Code stores credentials here)
    read_keychain_token()
}

/// Read token from ~/.config/cspy/token if it exists.
fn read_token_file() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    read_token_file_from(std::path::Path::new(&home))
}

fn read_anthropic_token_from_keys_file() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    read_anthropic_token_from_keys_file_from(Path::new(&home))
}

/// Read token from `<home>/.config/cspy/token`. Testable with a temp directory.
fn read_token_file_from(home: &std::path::Path) -> Option<String> {
    let path = home.join(".config/cspy/token");
    let contents = std::fs::read_to_string(&path).ok()?;
    normalize_str(&contents)
}

fn read_anthropic_token_from_keys_file_from(home: &Path) -> Option<String> {
    let keys = read_keys_file_from(home)?;
    normalize_opt(keys.anthropic?.oauth_token)
}

fn keys_file_path(home: &Path) -> PathBuf {
    home.join(".config/cspy/keys.json")
}

fn read_keys_file_from(home: &Path) -> Option<KeysFile> {
    let path = keys_file_path(home);
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn normalize_str(v: &str) -> Option<String> {
    let s = v.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn normalize_opt(v: Option<String>) -> Option<String> {
    v.and_then(|s| normalize_str(&s))
}

/// Read token from macOS Keychain via the `security` CLI.
fn read_keychain_token() -> Result<TokenInfo, String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .map_err(|e| format!("Failed to run `security`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "No token found. Either:\n  \
             • Save named keys to ~/.config/cspy/keys.json, or\n  \
             • Install Claude Code and log in (automatic), or\n  \
             • Save your token to ~/.config/cspy/token\n\n\
             Keychain error: {stderr}"
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
        "Keychain: got token for subscription type {:?}, expires_at: {:?}",
        oauth.subscription_type.as_deref().unwrap_or("unknown"),
        oauth.expires_at,
    );

    Ok(TokenInfo {
        token: oauth.access_token,
        expires_at_ms: oauth.expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_valid_token_file() {
        let tmp = tempfile::tempdir().unwrap();
        let token_dir = tmp.path().join(".config/cspy");
        std::fs::create_dir_all(&token_dir).unwrap();
        std::fs::write(token_dir.join("token"), "  sk-ant-oat01-test-token  \n").unwrap();
        let result = read_token_file_from(tmp.path());
        assert_eq!(result, Some("sk-ant-oat01-test-token".to_string()));
    }

    #[test]
    fn empty_token_file_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let token_dir = tmp.path().join(".config/cspy");
        std::fs::create_dir_all(&token_dir).unwrap();
        std::fs::write(token_dir.join("token"), "  \n").unwrap();
        assert_eq!(read_token_file_from(tmp.path()), None);
    }

    #[test]
    fn missing_token_file_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read_token_file_from(tmp.path()), None);
    }

    #[test]
    fn parses_valid_credentials() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat01-abc123",
                "expiresAt": 1700000000000,
                "subscriptionType": "pro"
            }
        }"#;
        let creds: ClaudeCredentials = serde_json::from_str(json).unwrap();
        let oauth = creds.claude_ai_oauth.unwrap();
        assert_eq!(oauth.access_token, "sk-ant-oat01-abc123");
        assert_eq!(oauth.expires_at, Some(1700000000000));
        assert_eq!(oauth.subscription_type, Some("pro".to_string()));
    }

    #[test]
    fn parses_credentials_without_oauth_field() {
        let json = r#"{}"#;
        let creds: ClaudeCredentials = serde_json::from_str(json).unwrap();
        assert!(creds.claude_ai_oauth.is_none());
    }

    #[test]
    fn parses_credentials_with_null_optional_fields() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat01-abc123",
                "expiresAt": null,
                "subscriptionType": null
            }
        }"#;
        let creds: ClaudeCredentials = serde_json::from_str(json).unwrap();
        let oauth = creds.claude_ai_oauth.unwrap();
        assert_eq!(oauth.access_token, "sk-ant-oat01-abc123");
        assert!(oauth.expires_at.is_none());
        assert!(oauth.subscription_type.is_none());
    }

    #[test]
    fn debug_output_redacts_token() {
        let oauth = OAuthCreds {
            access_token: "sk-ant-oat01-super-secret".to_string(),
            expires_at: None,
            subscription_type: None,
        };
        let debug_str = format!("{:?}", oauth);
        assert!(debug_str.contains("[REDACTED]"), "debug should redact token");
        assert!(!debug_str.contains("super-secret"), "debug must NOT contain the actual token");
    }

    #[test]
    fn reads_anthropic_token_from_keys_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".config/cspy");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("keys.json"),
            r#"{"anthropic":{"oauth_token":" sk-ant-oat01-from-keys "}}"#,
        )
        .unwrap();

        let token = read_anthropic_token_from_keys_file_from(tmp.path());
        assert_eq!(token, Some("sk-ant-oat01-from-keys".to_string()));
    }
}

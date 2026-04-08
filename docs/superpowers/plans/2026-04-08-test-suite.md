# CSpy Test Suite & Linting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add comprehensive unit tests, integration tests, linting, and CI for CSpy's Rust and Svelte code.

**Architecture:** Inline `#[cfg(test)]` for Rust unit tests, `src-tauri/tests/` for HTTP integration tests with wiremock, co-located `*.test.ts` for Svelte/Vitest. Clippy and ESLint for linting. New CI workflow for lint+test gating.

**Tech Stack:** Rust (`#[cfg(test)]`, `wiremock`, `tempfile`), TypeScript (Vitest, `@testing-library/svelte`), Clippy, ESLint

**Spec:** `docs/superpowers/specs/2026-04-08-test-suite-design.md`

---

## File Map

**Create:**
- `src-tauri/tests/fetch_usage.rs` — wiremock integration tests for HTTP layer
- `src/lib/types.test.ts` — unit tests for TypeScript utility functions
- `src/routes/+page.test.ts` — component tests for the popover UI
- `eslint.config.js` — ESLint flat config for Svelte + TS
- `.github/workflows/test.yml` — CI workflow for lint + test

**Modify:**
- `src-tauri/Cargo.toml` — add `[dev-dependencies]`
- `src-tauri/src/lib.rs` — refactor `is_quiet_hours`, add `#[cfg(test)]` block
- `src-tauri/src/icon.rs` — extract `render_icon_rgba`, add `#[cfg(test)]` block
- `src-tauri/src/keychain.rs` — refactor `read_token_file`, add `#[cfg(test)]` block
- `src-tauri/src/usage.rs` — add `fetch_usage_from`, make module pub, add `#[cfg(test)]` block
- `package.json` — add dev-dependencies and `lint`/`test` scripts
- `vite.config.ts` — add Vitest config

---

### Task 1: Add Rust dev-dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dev-dependencies to Cargo.toml**

Append after the `[dependencies]` section:

```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1.0", features = ["rt", "time", "sync", "macros"] }
```

Note: tokio is already in `[dependencies]` but the dev-dep adds `macros` feature for `#[tokio::test]`.

- [ ] **Step 2: Make `usage` module public for integration tests**

In `src-tauri/src/lib.rs` line 2, change:

```rust
mod usage;
```

to:

```rust
pub mod usage;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "chore: add test dev-dependencies and make usage module pub"
```

---

### Task 2: Refactor `is_quiet_hours` for testability

**Files:**
- Modify: `src-tauri/src/lib.rs:151-155`

- [ ] **Step 1: Extract `is_quiet_hours_at(hour: u32)` and update caller**

Replace the existing `is_quiet_hours` function at line 151-155 with:

```rust
/// Returns true if the given hour (0–23) falls within quiet hours (23:00–08:00).
fn is_quiet_hours_at(hour: u32) -> bool {
    hour >= 23 || hour < 8
}

/// Returns true if current local time is within quiet hours (23:00–08:00).
fn is_quiet_hours() -> bool {
    is_quiet_hours_at(chrono::Local::now().hour())
}
```

No callers change — `is_quiet_hours()` still works identically.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor: extract is_quiet_hours_at for testability"
```

---

### Task 3: Refactor `read_token_file` for testability

**Files:**
- Modify: `src-tauri/src/keychain.rs:62-72`

- [ ] **Step 1: Extract `read_token_file_from(home: &Path)` and keep original as wrapper**

Replace the `read_token_file` function at line 62-72 with:

```rust
/// Read token from ~/.config/cspy/token if it exists.
fn read_token_file() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    read_token_file_from(std::path::Path::new(&home))
}

/// Read token from `<home>/.config/cspy/token`. Testable with a temp directory.
fn read_token_file_from(home: &std::path::Path) -> Option<String> {
    let path = home.join(".config/cspy/token");
    let contents = std::fs::read_to_string(&path).ok()?;
    let token = contents.trim().to_string();
    if token.is_empty() {
        return None;
    }
    Some(token)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/keychain.rs
git commit -m "refactor: extract read_token_file_from for testability"
```

---

### Task 4: Refactor `fetch_usage` for URL injection

**Files:**
- Modify: `src-tauri/src/usage.rs:3,43-61`

- [ ] **Step 1: Make `USAGE_URL` public and add `fetch_usage_from`**

Change line 3 from `const` to `pub const`:

```rust
pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
```

Then rename the existing `fetch_usage` function to `fetch_usage_from` with a `url` parameter, and add a convenience wrapper:

```rust
/// Fetch current usage from the Anthropic OAuth endpoint.
pub async fn fetch_usage(client: &reqwest::Client, token: &str) -> Result<UsageData, String> {
    fetch_usage_from(client, token, USAGE_URL).await
}

/// Fetch usage from a given URL. Used by integration tests with a mock server.
pub async fn fetch_usage_from(client: &reqwest::Client, token: &str, url: &str) -> Result<UsageData, String> {
    // Small random jitter (50–250ms) to avoid clustering with other callers sharing this token
    let jitter = std::time::Duration::from_millis(50 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64 % 200));
    tokio::time::sleep(jitter).await;

    log::info!("API request -> {} (after {}ms jitter)", url, jitter.as_millis());

    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", BETA_HEADER)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = resp.text().await.unwrap_or_default();
        log::warn!("API returned 401 — token expired or invalid: {body}");
        return Err("token_expired".into());
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        log::warn!("API rate limited (429) — Retry-After: {retry_after}s");
        return Err(format!("rate_limited:{retry_after}"));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API returned {status}: {body}"));
    }

    let api: ApiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let now = chrono::Utc::now().to_rfc3339();

    Ok(UsageData {
        five_hour: api.five_hour.map(|b| UsageBucket {
            utilisation: b.utilization / 100.0,
            resets_at: b.resets_at,
        }),
        seven_day: api.seven_day.map(|b| UsageBucket {
            utilisation: b.utilization / 100.0,
            resets_at: b.resets_at,
        }),
        fetched_at: now,
    })
}
```

No callers in `lib.rs` need updating — they all use `usage::fetch_usage(client, token)` which still works.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/usage.rs
git commit -m "refactor: add fetch_usage_from with injectable URL for testing"
```

---

### Task 5: Extract icon rendering for testability

**Files:**
- Modify: `src-tauri/src/icon.rs`

- [ ] **Step 1: Extract `render_icon_rgba` as a pure function**

Add this function before `generate_usage_icon`, and refactor `generate_usage_icon` to call it:

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::image::Image;

pub(crate) const ICON_WIDTH: u32 = 32;
pub(crate) const ICON_HEIGHT: u32 = 32;

/// Cache of rendered icon buffers, keyed by quantised utilisation (0-20 = 5% steps).
/// Maximum 21 entries x 4 KiB = 84 KiB total — bounded, no unbounded leak.
static ICON_CACHE: Mutex<Option<HashMap<u8, &'static [u8]>>> = Mutex::new(None);

/// Render raw RGBA bytes for a 32x32 usage icon at the given utilisation level.
/// Pure function — no caching, no tauri dependency. Used directly by tests.
pub(crate) fn render_icon_rgba(quantised_util: f64) -> Vec<u8> {
    const BORDER: u32 = 2;
    const PADDING: u32 = 4;

    let fill_color: (u8, u8, u8) = if quantised_util >= 0.90 {
        (248, 113, 113) // Red: #f87171
    } else if quantised_util >= 0.70 {
        (251, 191, 36)  // Amber: #fbbf24
    } else {
        (74, 222, 128)  // Green: #4ade80
    };

    let outline_color: (u8, u8, u8) = (60, 60, 60);

    let inner_left = BORDER;
    let inner_right = ICON_WIDTH - BORDER;
    let inner_top = PADDING;
    let inner_bottom = ICON_HEIGHT - PADDING;
    let inner_width = inner_right - inner_left - 2 * BORDER;
    let fill_width = ((inner_width as f64 * quantised_util) as u32).min(inner_width);

    let mut rgba = vec![0u8; (ICON_WIDTH * ICON_HEIGHT * 4) as usize];

    for y in 0..ICON_HEIGHT {
        for x in 0..ICON_WIDTH {
            let pixel_idx = ((y * ICON_WIDTH + x) * 4) as usize;

            let (r, g, b, a) = if y < inner_top || y >= inner_bottom {
                (0, 0, 0, 0)
            } else if x < inner_left + BORDER || x >= inner_right - BORDER
                || y < inner_top + BORDER || y >= inner_bottom - BORDER {
                (outline_color.0, outline_color.1, outline_color.2, 255)
            } else {
                let inner_x = x - inner_left - BORDER;
                if inner_x < fill_width {
                    (fill_color.0, fill_color.1, fill_color.2, 255)
                } else {
                    (180, 180, 180, 80)
                }
            };

            rgba[pixel_idx] = r;
            rgba[pixel_idx + 1] = g;
            rgba[pixel_idx + 2] = b;
            rgba[pixel_idx + 3] = a;
        }
    }

    rgba
}

/// Generate a dynamic usage icon: hollow rectangle with coloured fill based on utilisation.
/// Renders at 32x32 for Retina crispness. macOS menu bar expects @2x icons.
///
/// Icons are cached by quantised utilisation (5% steps) so each unique level
/// is only rendered once. The leaked buffers are bounded to ~84 KiB total.
pub fn generate_usage_icon(utilisation: f64) -> Image<'static> {
    let util = utilisation.max(0.0).min(1.0);
    let key = (util * 20.0).round() as u8;

    {
        let mut guard = ICON_CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(rgba_ref) = cache.get(&key) {
            return Image::new(rgba_ref, ICON_WIDTH, ICON_HEIGHT);
        }
    }

    let quantised_util = key as f64 / 20.0;
    let rgba = render_icon_rgba(quantised_util);

    let rgba_static: &'static [u8] = Box::leak(rgba.into_boxed_slice());

    let mut guard = ICON_CACHE.lock().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    cache.insert(key, rgba_static);

    Image::new(rgba_static, ICON_WIDTH, ICON_HEIGHT)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/icon.rs
git commit -m "refactor: extract render_icon_rgba for testability"
```

---

### Task 6: Unit tests for `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs` — append `#[cfg(test)]` module

- [ ] **Step 1: Write unit tests**

Append at the bottom of `src-tauri/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ── format_countdown ────────────────────────────

    #[test]
    fn countdown_future_hours_and_mins() {
        let reset = (chrono::Utc::now() + chrono::Duration::minutes(150)).to_rfc3339();
        let result = format_countdown(&reset);
        assert!(result.starts_with("2h "), "expected '2h Xm', got '{result}'");
    }

    #[test]
    fn countdown_future_mins_only() {
        let reset = (chrono::Utc::now() + chrono::Duration::minutes(42)).to_rfc3339();
        let result = format_countdown(&reset);
        assert_eq!(result, "42m");
    }

    #[test]
    fn countdown_past_returns_dash() {
        let reset = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assert_eq!(format_countdown(&reset), "\u{2014}"); // em dash
    }

    #[test]
    fn countdown_unparseable_returns_dash() {
        assert_eq!(format_countdown("not-a-date"), "\u{2014}");
    }

    // ── is_window_expired ───────────────────────────

    #[test]
    fn window_expired_past_resets_at() {
        let bucket = usage::UsageBucket {
            utilisation: 0.5,
            resets_at: Some((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
        };
        assert!(is_window_expired(&bucket));
    }

    #[test]
    fn window_not_expired_future_resets_at() {
        let bucket = usage::UsageBucket {
            utilisation: 0.5,
            resets_at: Some((chrono::Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
        };
        assert!(!is_window_expired(&bucket));
    }

    #[test]
    fn window_not_expired_none_resets_at() {
        let bucket = usage::UsageBucket {
            utilisation: 0.5,
            resets_at: None,
        };
        assert!(!is_window_expired(&bucket));
    }

    // ── is_quiet_hours_at ───────────────────────────

    #[test]
    fn quiet_hours_boundaries() {
        assert!(!is_quiet_hours_at(22), "22:00 should NOT be quiet");
        assert!(is_quiet_hours_at(23),  "23:00 should be quiet");
        assert!(is_quiet_hours_at(0),   "00:00 should be quiet");
        assert!(is_quiet_hours_at(7),   "07:00 should be quiet");
        assert!(!is_quiet_hours_at(8),  "08:00 should NOT be quiet");
        assert!(!is_quiet_hours_at(12), "12:00 should NOT be quiet");
    }

    // ── backoff_sleep ───────────────────────────────

    #[test]
    fn backoff_zero_errors_returns_poll_secs() {
        assert_eq!(backoff_sleep(0), POLL_SECS);
    }

    #[test]
    fn backoff_one_error_returns_poll_secs() {
        assert_eq!(backoff_sleep(1), POLL_SECS);
    }

    #[test]
    fn backoff_two_errors_doubles() {
        assert_eq!(backoff_sleep(2), POLL_SECS * 2); // 360
    }

    #[test]
    fn backoff_three_errors_quadruples() {
        assert_eq!(backoff_sleep(3), POLL_SECS * 4); // 720
    }

    #[test]
    fn backoff_capped_at_max() {
        assert_eq!(backoff_sleep(10), MAX_BACKOFF_SECS); // capped at 1800
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "test: add unit tests for lib.rs pure functions"
```

---

### Task 7: Unit tests for `icon.rs`

**Files:**
- Modify: `src-tauri/src/icon.rs` — append `#[cfg(test)]` module

- [ ] **Step 1: Write unit tests**

Append at the bottom of `src-tauri/src/icon.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: get pixel RGBA at (x, y) from a flat RGBA buffer.
    fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
        let idx = ((y * ICON_WIDTH + x) * 4) as usize;
        (rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3])
    }

    /// Count interior pixels (not border, not padding) that match a given RGB.
    fn count_interior_pixels_with_rgb(rgba: &[u8], rgb: (u8, u8, u8)) -> u32 {
        let mut count = 0;
        // Interior region: x in 4..28, y in 6..26 (inside border + padding)
        for y in 6..26 {
            for x in 4..28 {
                let (r, g, b, _) = pixel_at(rgba, x, y);
                if (r, g, b) == rgb {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn dimensions_are_32x32() {
        let rgba = render_icon_rgba(0.5);
        assert_eq!(rgba.len(), (32 * 32 * 4) as usize);
    }

    #[test]
    fn zero_percent_has_no_fill_pixels() {
        let rgba = render_icon_rgba(0.0);
        let green = count_interior_pixels_with_rgb(&rgba, (74, 222, 128));
        assert_eq!(green, 0, "0% should have no green fill pixels");
    }

    #[test]
    fn fifty_percent_uses_green() {
        let rgba = render_icon_rgba(0.5);
        let green = count_interior_pixels_with_rgb(&rgba, (74, 222, 128));
        assert!(green > 0, "50% should have green fill pixels");
        // Should be roughly half the interior
        let grey = count_interior_pixels_with_rgb(&rgba, (180, 180, 180));
        assert!(green > 0 && grey > 0, "50% should have both fill and empty regions");
    }

    #[test]
    fn seventy_percent_uses_amber() {
        let rgba = render_icon_rgba(0.70);
        let amber = count_interior_pixels_with_rgb(&rgba, (251, 191, 36));
        assert!(amber > 0, "70% should use amber fill");
    }

    #[test]
    fn ninety_percent_uses_red() {
        let rgba = render_icon_rgba(0.90);
        let red = count_interior_pixels_with_rgb(&rgba, (248, 113, 113));
        assert!(red > 0, "90% should use red fill");
    }

    #[test]
    fn hundred_percent_fills_entire_interior() {
        let rgba = render_icon_rgba(1.0);
        let grey = count_interior_pixels_with_rgb(&rgba, (180, 180, 180));
        assert_eq!(grey, 0, "100% should have no empty grey pixels in interior");
    }

    #[test]
    fn padding_rows_are_transparent() {
        let rgba = render_icon_rgba(0.5);
        // Top padding: y 0..4, all pixels should be fully transparent
        for y in 0..4 {
            for x in 0..ICON_WIDTH {
                let (_, _, _, a) = pixel_at(&rgba, x, y);
                assert_eq!(a, 0, "pixel ({x},{y}) in top padding should be transparent");
            }
        }
        // Bottom padding: y 28..32
        for y in 28..ICON_HEIGHT {
            for x in 0..ICON_WIDTH {
                let (_, _, _, a) = pixel_at(&rgba, x, y);
                assert_eq!(a, 0, "pixel ({x},{y}) in bottom padding should be transparent");
            }
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib icon`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/icon.rs
git commit -m "test: add unit tests for icon rendering"
```

---

### Task 8: Unit tests for `keychain.rs`

**Files:**
- Modify: `src-tauri/src/keychain.rs` — append `#[cfg(test)]` module

- [ ] **Step 1: Write unit tests**

Append at the bottom of `src-tauri/src/keychain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ── read_token_file_from ────────────────────────

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

    // ── ClaudeCredentials deserialization ────────────

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

    // ── Redacted Debug ──────────────────────────────

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
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib keychain`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/keychain.rs
git commit -m "test: add unit tests for keychain deserialization and token file"
```

---

### Task 9: Unit tests for `usage.rs` response parsing

**Files:**
- Modify: `src-tauri/src/usage.rs` — append `#[cfg(test)]` module

- [ ] **Step 1: Write unit tests**

These test the `ApiResponse` → `UsageData` mapping logic. Since `ApiResponse` is private, we test via JSON deserialization of the full struct to verify the mapping.

Append at the bottom of `src-tauri/src/usage.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: deserialize raw API JSON and apply the same normalisation as fetch_usage.
    fn parse_api_response(json: &str) -> UsageData {
        let api: ApiResponse = serde_json::from_str(json).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        UsageData {
            five_hour: api.five_hour.map(|b| UsageBucket {
                utilisation: b.utilization / 100.0,
                resets_at: b.resets_at,
            }),
            seven_day: api.seven_day.map(|b| UsageBucket {
                utilisation: b.utilization / 100.0,
                resets_at: b.resets_at,
            }),
            fetched_at: now,
        }
    }

    #[test]
    fn normalises_utilization_50_to_0_5() {
        let data = parse_api_response(r#"{
            "five_hour": { "utilization": 50.0, "resets_at": "2026-04-08T12:00:00Z" },
            "seven_day": null
        }"#);
        let bucket = data.five_hour.unwrap();
        assert!((bucket.utilisation - 0.5).abs() < f64::EPSILON);
        assert_eq!(bucket.resets_at, Some("2026-04-08T12:00:00Z".to_string()));
    }

    #[test]
    fn normalises_utilization_0_to_0() {
        let data = parse_api_response(r#"{
            "five_hour": { "utilization": 0.0, "resets_at": null },
            "seven_day": null
        }"#);
        let bucket = data.five_hour.unwrap();
        assert!((bucket.utilisation - 0.0).abs() < f64::EPSILON);
        assert!(bucket.resets_at.is_none());
    }

    #[test]
    fn normalises_utilization_100_to_1() {
        let data = parse_api_response(r#"{
            "five_hour": { "utilization": 100.0, "resets_at": null },
            "seven_day": null
        }"#);
        let bucket = data.five_hour.unwrap();
        assert!((bucket.utilisation - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_five_hour_is_none() {
        let data = parse_api_response(r#"{
            "five_hour": null,
            "seven_day": { "utilization": 10.0, "resets_at": null }
        }"#);
        assert!(data.five_hour.is_none());
        assert!(data.seven_day.is_some());
    }

    #[test]
    fn both_buckets_present() {
        let data = parse_api_response(r#"{
            "five_hour": { "utilization": 25.0, "resets_at": "2026-04-08T15:00:00Z" },
            "seven_day": { "utilization": 10.0, "resets_at": "2026-04-12T00:00:00Z" }
        }"#);
        assert!((data.five_hour.unwrap().utilisation - 0.25).abs() < f64::EPSILON);
        assert!((data.seven_day.unwrap().utilisation - 0.10).abs() < f64::EPSILON);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib usage`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/usage.rs
git commit -m "test: add unit tests for usage response parsing and normalisation"
```

---

### Task 10: Integration tests for `fetch_usage` with wiremock

**Files:**
- Create: `src-tauri/tests/fetch_usage.rs`

- [ ] **Step 1: Write integration tests**

Create `src-tauri/tests/fetch_usage.rs`:

```rust
use cspy_lib::usage::{build_client, fetch_usage_from, UsageData};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn success_returns_normalised_data() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "five_hour": { "utilization": 42.0, "resets_at": "2026-04-08T15:00:00Z" },
            "seven_day": { "utilization": 8.5, "resets_at": "2026-04-12T00:00:00Z" }
        })))
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let data = fetch_usage_from(&client, "test-token", &url).await.unwrap();

    let five = data.five_hour.unwrap();
    assert!((five.utilisation - 0.42).abs() < f64::EPSILON);
    assert_eq!(five.resets_at, Some("2026-04-08T15:00:00Z".to_string()));

    let seven = data.seven_day.unwrap();
    assert!((seven.utilisation - 0.085).abs() < f64::EPSILON);
}

#[tokio::test]
async fn unauthorized_returns_token_expired() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid token"))
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let err = fetch_usage_from(&client, "bad-token", &url).await.unwrap_err();

    assert_eq!(err, "token_expired");
}

#[tokio::test]
async fn rate_limited_with_retry_after() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
        )
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let err = fetch_usage_from(&client, "test-token", &url).await.unwrap_err();

    assert_eq!(err, "rate_limited:60");
}

#[tokio::test]
async fn rate_limited_without_retry_after() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let err = fetch_usage_from(&client, "test-token", &url).await.unwrap_err();

    assert_eq!(err, "rate_limited:0");
}

#[tokio::test]
async fn server_error_returns_status_in_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let err = fetch_usage_from(&client, "test-token", &url).await.unwrap_err();

    assert!(err.contains("500"), "error should contain status code: {err}");
    assert!(err.contains("internal error"), "error should contain body: {err}");
}

#[tokio::test]
async fn malformed_json_returns_parse_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("not json at all")
                .insert_header("Content-Type", "application/json")
        )
        .mount(&mock_server)
        .await;

    let client = build_client().unwrap();
    let url = format!("{}/api/oauth/usage", mock_server.uri());
    let err = fetch_usage_from(&client, "test-token", &url).await.unwrap_err();

    assert!(err.contains("parse"), "error should mention parsing: {err}");
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test fetch_usage`
Expected: All 6 tests pass. Each test takes ~200-300ms (jitter + mock server).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/fetch_usage.rs
git commit -m "test: add wiremock integration tests for fetch_usage"
```

---

### Task 11: Run Clippy and fix warnings

**Files:**
- Modify: any files flagged by clippy

- [ ] **Step 1: Run clippy**

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings 2>&1`
Expected: May produce warnings. Fix any that appear.

- [ ] **Step 2: Fix any warnings**

Apply clippy's suggestions. Common issues in this codebase might include:
- `needless_borrow` on string references
- `redundant_clone` on owned values
- `map_err` patterns that could be simplified

- [ ] **Step 3: Run clippy again to confirm clean**

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings 2>&1`
Expected: No warnings or errors.

- [ ] **Step 4: Run all Rust tests to confirm nothing broke**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A src-tauri/
git commit -m "fix: resolve clippy warnings"
```

---

### Task 12: Add Svelte test dependencies and Vitest config

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`

- [ ] **Step 1: Install dev-dependencies**

Run:

```bash
npm install -D vitest @testing-library/svelte @testing-library/jest-dom jsdom
```

- [ ] **Step 2: Add scripts to `package.json`**

Add to the `"scripts"` section:

```json
"test": "vitest",
"test:run": "vitest run"
```

- [ ] **Step 3: Configure Vitest in `vite.config.ts`**

Replace `vite.config.ts` with:

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true
	},
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'jsdom',
		globals: true,
	}
});
```

- [ ] **Step 4: Verify Vitest runs (no tests yet)**

Run: `npx vitest run`
Expected: "No test files found" or similar — no errors.

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json vite.config.ts
git commit -m "chore: add Vitest and testing-library dependencies"
```

---

### Task 13: Unit tests for `src/lib/types.ts`

**Files:**
- Create: `src/lib/types.test.ts`

- [ ] **Step 1: Write unit tests**

Create `src/lib/types.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { tierFor, burnRateTier, calculateBurnRate, formatCountdown } from './types';

describe('tierFor', () => {
	it('returns green below 70%', () => {
		expect(tierFor(0)).toBe('green');
		expect(tierFor(0.5)).toBe('green');
		expect(tierFor(0.69)).toBe('green');
	});

	it('returns amber at 70-89%', () => {
		expect(tierFor(0.70)).toBe('amber');
		expect(tierFor(0.80)).toBe('amber');
		expect(tierFor(0.89)).toBe('amber');
	});

	it('returns red at 90%+', () => {
		expect(tierFor(0.90)).toBe('red');
		expect(tierFor(0.95)).toBe('red');
		expect(tierFor(1.0)).toBe('red');
	});
});

describe('burnRateTier', () => {
	it('returns green below 16%/hr', () => {
		expect(burnRateTier(0)).toBe('green');
		expect(burnRateTier(15.9)).toBe('green');
	});

	it('returns amber at 16-19%/hr', () => {
		expect(burnRateTier(16)).toBe('amber');
		expect(burnRateTier(19.9)).toBe('amber');
	});

	it('returns red at 20%/hr+', () => {
		expect(burnRateTier(20)).toBe('red');
		expect(burnRateTier(30)).toBe('red');
	});
});

describe('calculateBurnRate', () => {
	const WINDOW = 5 * 3600; // 5 hours in seconds

	it('returns 0 when not enough elapsed time', () => {
		// 30 seconds elapsed (WINDOW - 30s remaining)
		expect(calculateBurnRate(0.5, WINDOW - 30)).toBe(0);
	});

	it('calculates correctly for 50% over 2.5 hours', () => {
		// 2.5h elapsed = 9000s, remaining = 18000 - 9000 = 9000s
		const rate = calculateBurnRate(0.5, 9000);
		expect(rate).toBeCloseTo(20.0, 1); // 50% / 2.5h = 20%/hr
	});

	it('calculates correctly for 10% over 1 hour', () => {
		// 1h elapsed = 3600s, remaining = 18000 - 3600 = 14400s
		const rate = calculateBurnRate(0.1, 14400);
		expect(rate).toBeCloseTo(10.0, 1); // 10% / 1h = 10%/hr
	});
});

describe('formatCountdown', () => {
	beforeEach(() => {
		vi.useFakeTimers();
		vi.setSystemTime(new Date('2026-04-08T12:00:00Z'));
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('returns dash for null', () => {
		expect(formatCountdown(null)).toBe('\u2014'); // em dash
	});

	it('returns "resetting..." for past timestamp', () => {
		expect(formatCountdown('2026-04-08T11:00:00Z')).toBe('resetting\u2026');
	});

	it('formats hours and minutes', () => {
		expect(formatCountdown('2026-04-08T13:30:00Z')).toBe('1h 30m');
	});

	it('formats minutes only when under an hour', () => {
		expect(formatCountdown('2026-04-08T12:45:00Z')).toBe('45m');
	});
});
```

- [ ] **Step 2: Run tests**

Run: `npx vitest run src/lib/types.test.ts`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.test.ts
git commit -m "test: add unit tests for TypeScript utility functions"
```

---

### Task 14: Component tests for `+page.svelte`

**Files:**
- Create: `src/routes/+page.test.ts`

- [ ] **Step 1: Write component tests**

Create `src/routes/+page.test.ts`:

```typescript
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Capture event listener callbacks so tests can simulate Tauri events
const listeners: Record<string, (event: { payload: unknown }) => void> = {};

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
	invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((eventName: string, callback: Function) => {
		listeners[eventName] = callback as (event: { payload: unknown }) => void;
		return Promise.resolve(() => {});
	}),
}));

vi.mock('@tauri-apps/api/app', () => ({
	getVersion: vi.fn(() => Promise.resolve('0.4.0')),
}));

// Must import AFTER vi.mock calls
import Page from './+page.svelte';

const futureReset = new Date(Date.now() + 3 * 3600_000).toISOString(); // 3h from now

const mockUsage = {
	five_hour: { utilisation: 0.42, resets_at: futureReset },
	seven_day: null,
	fetched_at: new Date().toISOString(),
};

describe('+page.svelte', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		Object.keys(listeners).forEach(k => delete listeners[k]);
		// Default: get_usage rejects (no cached data)
		mockInvoke.mockRejectedValue('No cached data yet');
	});

	it('shows loading state initially', async () => {
		render(Page);
		await waitFor(() => {
			expect(screen.getByText(/Reading Keychain/)).toBeTruthy();
		});
	});

	it('renders usage data when usage-updated fires', async () => {
		render(Page);

		// Wait for onMount to set up listeners
		await waitFor(() => expect(listeners['usage-updated']).toBeDefined());

		// Simulate the usage-updated event
		listeners['usage-updated']({ payload: mockUsage });

		await waitFor(() => {
			expect(screen.getByText('42%')).toBeTruthy();
		});
	});

	it('renders error box when usage-error fires with no data', async () => {
		render(Page);

		await waitFor(() => expect(listeners['usage-error']).toBeDefined());

		listeners['usage-error']({ payload: 'Token expired' });

		await waitFor(() => {
			expect(screen.getByText('Token expired')).toBeTruthy();
		});
	});

	it('renders stale warning when error occurs but data exists', async () => {
		render(Page);

		await waitFor(() => expect(listeners['usage-updated']).toBeDefined());

		// First: provide data
		listeners['usage-updated']({ payload: mockUsage });
		await waitFor(() => expect(screen.getByText('42%')).toBeTruthy());

		// Then: error arrives
		listeners['usage-error']({ payload: 'Network error' });

		await waitFor(() => {
			expect(screen.getByText(/showing cached data/i)).toBeTruthy();
		});
	});

	it('calls refresh_usage when refresh button is clicked', async () => {
		mockInvoke.mockImplementation((cmd: string) => {
			if (cmd === 'get_usage') return Promise.reject('no data');
			if (cmd === 'refresh_usage') return Promise.resolve(mockUsage);
			return Promise.reject('unknown command');
		});

		render(Page);

		// Wait for data to load via listener so button is visible
		await waitFor(() => expect(listeners['usage-updated']).toBeDefined());
		listeners['usage-updated']({ payload: mockUsage });
		await waitFor(() => expect(screen.getByText('42%')).toBeTruthy());

		const refreshBtn = screen.getByTitle('Refresh now');
		await fireEvent.click(refreshBtn);

		expect(mockInvoke).toHaveBeenCalledWith('refresh_usage');
	});
});
```

- [ ] **Step 2: Run tests**

Run: `npx vitest run src/routes/+page.test.ts`
Expected: All tests pass. Some tests may need minor adjustments depending on exact `@testing-library/svelte` Svelte 5 behavior — fix as needed.

- [ ] **Step 3: Run all Vitest tests together**

Run: `npx vitest run`
Expected: All Svelte tests pass (types.test.ts + +page.test.ts).

- [ ] **Step 4: Commit**

```bash
git add src/routes/+page.test.ts
git commit -m "test: add component tests for popover UI"
```

---

### Task 15: ESLint configuration

**Files:**
- Create: `eslint.config.js`
- Modify: `package.json` — add `lint` script

- [ ] **Step 1: Install ESLint dependencies**

Run:

```bash
npm install -D eslint @eslint/js typescript-eslint eslint-plugin-svelte
```

- [ ] **Step 2: Create `eslint.config.js`**

Create `eslint.config.js` in the project root:

```javascript
import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

export default [
	eslint.configs.recommended,
	...tseslint.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tseslint.parser,
			},
		},
	},
	{
		ignores: [
			'.svelte-kit/',
			'build/',
			'src-tauri/',
			'node_modules/',
		],
	},
];
```

- [ ] **Step 3: Add lint script to `package.json`**

Add to `"scripts"`:

```json
"lint": "eslint src"
```

- [ ] **Step 4: Run ESLint**

Run: `npm run lint`
Expected: May produce warnings. Fix any errors.

- [ ] **Step 5: Fix any lint errors**

Apply ESLint's suggestions. Re-run `npm run lint` until clean.

- [ ] **Step 6: Commit**

```bash
git add eslint.config.js package.json package-lock.json src/
git commit -m "chore: add ESLint with Svelte + TypeScript support"
```

---

### Task 16: CI workflow for lint + test

**Files:**
- Create: `.github/workflows/test.yml`

- [ ] **Step 1: Create the workflow**

Create `.github/workflows/test.yml`:

```yaml
name: Lint & Test

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  lint-and-test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Cache Rust artefacts
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            src-tauri/target
          key: ${{ runner.os }}-cargo-test-${{ hashFiles('src-tauri/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-test-

      - run: npm ci

      - name: Clippy
        run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

      - name: Rust tests
        run: cargo test --manifest-path src-tauri/Cargo.toml

      - name: ESLint
        run: npm run lint

      - name: Vitest
        run: npx vitest run
```

- [ ] **Step 2: Verify the workflow YAML is valid**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/test.yml'))"`
Expected: No errors (valid YAML).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: add lint and test workflow"
```

---

## Final Verification

After all tasks are complete:

- [ ] Run: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — no warnings
- [ ] Run: `cargo test --manifest-path src-tauri/Cargo.toml` — all Rust tests pass
- [ ] Run: `npm run lint` — no ESLint errors
- [ ] Run: `npx vitest run` — all Svelte tests pass
- [ ] Run: `cargo check --manifest-path src-tauri/Cargo.toml` — compiles cleanly

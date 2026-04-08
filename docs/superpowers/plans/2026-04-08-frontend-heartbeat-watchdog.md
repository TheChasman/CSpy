# Frontend Heartbeat Watchdog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a frontend health watchdog that detects when the Svelte popover stops sending heartbeats and auto-recovers (restart Vite in dev, reload WebView in all environments).

**Architecture:** The Svelte app calls `invoke('heartbeat')` every 30s; Rust stores `last_heartbeat: Instant` in `AppState`. A 60s watchdog task calls `is_frontend_healthy()` (a pure, testable function) and triggers recovery on failure. Dev builds probe port 1420 and spawn `npm run dev` if Vite is down; all builds reload the WebView. After two failed reloads in production, a native notification is shown.

**Tech Stack:** Rust (tokio, std::process::Command, std::net::TcpStream), Svelte 5 runes, Tauri 2 (commands, WebviewWindow::eval, RunEvent::Exit)

---

## File Structure

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs` | Add `AppState` fields, `heartbeat` command, `is_frontend_healthy()`, `start_watchdog()`, dev-only `ensure_vite_running()`, exit handler |
| `src/routes/+page.svelte` | Add 30s heartbeat `invoke` interval in `onMount`, cleanup in `onDestroy` |

---

### Task 1: `is_frontend_healthy` — TDD

**Files:**
- Modify: `src-tauri/src/lib.rs` (add pure function + 4 unit tests)

The health check is a pure function — no I/O, easy to test. Write the tests first.

- [ ] **Step 1: Write the 4 failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `src-tauri/src/lib.rs`:

```rust
#[test]
fn heartbeat_healthy_within_threshold() {
    let now = std::time::Instant::now();
    let startup = now - std::time::Duration::from_secs(60);
    let last_beat = Some(now - std::time::Duration::from_secs(60));
    assert!(is_frontend_healthy(last_beat, startup, now, 15, 90));
}

#[test]
fn heartbeat_unhealthy_beyond_threshold() {
    let now = std::time::Instant::now();
    let startup = now - std::time::Duration::from_secs(200);
    let last_beat = Some(now - std::time::Duration::from_secs(100));
    assert!(!is_frontend_healthy(last_beat, startup, now, 15, 90));
}

#[test]
fn heartbeat_healthy_during_grace_period() {
    let now = std::time::Instant::now();
    let startup = now - std::time::Duration::from_secs(10); // 10s < 15s grace
    let last_beat = None; // no heartbeat yet
    assert!(is_frontend_healthy(last_beat, startup, now, 15, 90));
}

#[test]
fn heartbeat_none_after_grace_period() {
    let now = std::time::Instant::now();
    let startup = now - std::time::Duration::from_secs(20); // 20s > 15s grace
    let last_beat = None; // no heartbeat ever received
    assert!(!is_frontend_healthy(last_beat, startup, now, 15, 90));
}
```

- [ ] **Step 2: Run to confirm 4 failures**

```bash
cargo test --manifest-path src-tauri/Cargo.toml heartbeat 2>&1 | tail -15
```

Expected: 4 test failures with `error[E0425]: cannot find function 'is_frontend_healthy'`

- [ ] **Step 3: Add constants and implement `is_frontend_healthy`**

After the existing constants block (after line 31, before `// ── Tauri commands`), add:

```rust
/// Seconds after startup before the watchdog begins checking heartbeats.
const HEARTBEAT_GRACE_SECS: u64 = 15;

/// Seconds without a heartbeat before the frontend is considered unhealthy.
const HEARTBEAT_THRESHOLD_SECS: u64 = 90;

/// Watchdog tick interval in seconds.
const WATCHDOG_TICK_SECS: u64 = 60;
```

After the `backoff_sleep` function (around line 169), add:

```rust
/// Returns true if the frontend is healthy.
/// Pure function — no I/O, easy to test.
///
/// `last_heartbeat`: when the last heartbeat was received (`None` = never)
/// `startup`: when the app started
/// `now`: current time (pass `Instant::now()` in production)
/// `grace_secs`: seconds after startup before checking
/// `threshold_secs`: seconds without a heartbeat before unhealthy
fn is_frontend_healthy(
    last_heartbeat: Option<std::time::Instant>,
    startup: std::time::Instant,
    now: std::time::Instant,
    grace_secs: u64,
    threshold_secs: u64,
) -> bool {
    // Still in startup grace period — don't alert on slow WebView loads
    if now.duration_since(startup).as_secs() < grace_secs {
        return true;
    }
    match last_heartbeat {
        None => false, // past grace period, no heartbeat ever received
        Some(last) => now.duration_since(last).as_secs() < threshold_secs,
    }
}
```

- [ ] **Step 4: Run tests — expect 4 passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml heartbeat 2>&1 | tail -10
```

Expected:
```
test tests::heartbeat_healthy_during_grace_period ... ok
test tests::heartbeat_healthy_within_threshold ... ok
test tests::heartbeat_none_after_grace_period ... ok
test tests::heartbeat_unhealthy_beyond_threshold ... ok
test result: ok. 4 passed; 0 failed
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add is_frontend_healthy with TDD

Pure function checks heartbeat age against configurable grace
and threshold windows. Four unit tests cover all branches."
```

---

### Task 2: `AppState` fields + `heartbeat` command

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add new fields to `AppState`**

Replace the `AppState` struct (lines 14–22) with:

```rust
/// Shared app state: cached usage, OAuth token, HTTP client, and heartbeat tracking.
pub struct AppState {
    pub token: RwLock<Option<String>>,
    /// Token expiry as millisecond Unix timestamp (None = unknown / token-file source).
    pub token_expires_at_ms: RwLock<Option<i64>>,
    pub cached: RwLock<Option<UsageData>>,
    pub client: reqwest::Client,
    /// Set to true when an update has been downloaded and is ready to install on restart.
    pub update_pending: RwLock<bool>,
    /// Last time the frontend sent a heartbeat. None = not yet received.
    pub last_heartbeat: RwLock<Option<std::time::Instant>>,
    /// When the app started — used for the heartbeat grace period.
    pub startup_time: std::time::Instant,
    /// Vite child process (dev builds only). Killed on app exit.
    pub vite_child: std::sync::Mutex<Option<std::process::Child>>,
}
```

- [ ] **Step 2: Add the `heartbeat` Tauri command**

After the `refresh_usage` command (after line 69), add:

```rust
/// Called by the frontend every 30s to signal it is alive.
#[tauri::command]
async fn heartbeat(state: State<'_, Arc<AppState>>) {
    *state.last_heartbeat.write().await = Some(std::time::Instant::now());
}
```

- [ ] **Step 3: Register `heartbeat` in `invoke_handler`**

In `run()`, change:

```rust
.invoke_handler(tauri::generate_handler![get_usage, refresh_usage])
```

to:

```rust
.invoke_handler(tauri::generate_handler![get_usage, refresh_usage, heartbeat])
```

- [ ] **Step 4: Initialise new fields in `AppState` construction**

In `run()`, replace the `AppState` initialisation:

```rust
let state = Arc::new(AppState {
    token: RwLock::new(None),
    token_expires_at_ms: RwLock::new(None),
    cached: RwLock::new(None),
    client,
    update_pending: RwLock::new(false),
    last_heartbeat: RwLock::new(None),
    startup_time: std::time::Instant::now(),
    vite_child: std::sync::Mutex::new(None),
});
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error"
```

Expected: no output (clean compile).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add heartbeat AppState fields and command

AppState gains last_heartbeat, startup_time, and vite_child.
The heartbeat command updates last_heartbeat on each call."
```

---

### Task 3: Watchdog task + dev Vite recovery

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `start_watchdog` after `start_countdown_ticker`**

After the closing `}` of `start_countdown_ticker` (around line 284), add:

```rust
fn start_watchdog(app_handle: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        // Wait for the grace period before starting health checks
        tokio::time::sleep(std::time::Duration::from_secs(HEARTBEAT_GRACE_SECS + 5)).await;

        let mut reload_count: u32 = 0;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(WATCHDOG_TICK_SECS)).await;

            let last = *state.last_heartbeat.read().await;
            let healthy = is_frontend_healthy(
                last,
                state.startup_time,
                std::time::Instant::now(),
                HEARTBEAT_GRACE_SECS,
                HEARTBEAT_THRESHOLD_SECS,
            );

            if healthy {
                if reload_count > 0 {
                    log::info!("Watchdog: frontend recovered after {} reload(s)", reload_count);
                    reload_count = 0;
                }
                continue;
            }

            reload_count += 1;
            log::warn!("Watchdog: frontend heartbeat timeout — recovery attempt {}", reload_count);

            // Dev: ensure Vite is running before attempting WebView reload
            #[cfg(debug_assertions)]
            ensure_vite_running(&state);

            // Reload the WebView
            if let Some(win) = app_handle.get_webview_window("popover") {
                let _ = win.eval("window.location.reload()");
                log::info!("Watchdog: triggered WebView reload");
            }

            // After two failed reloads, emit a notification in production
            if reload_count >= 2 {
                log::error!("Watchdog: frontend still unresponsive after {} reloads", reload_count);
                #[cfg(not(debug_assertions))]
                {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app_handle
                        .notification()
                        .builder()
                        .title("CSpy")
                        .body("Popover unresponsive — restart CSpy to recover")
                        .show();
                }
                reload_count = 0;
            }

            // Clear heartbeat and sleep for threshold duration so next check
            // uses only real post-recovery heartbeats
            *state.last_heartbeat.write().await = None;
            tokio::time::sleep(std::time::Duration::from_secs(HEARTBEAT_THRESHOLD_SECS)).await;
        }
    });
}
```

- [ ] **Step 2: Add `ensure_vite_running` (dev-only) immediately after `start_watchdog`**

```rust
/// Ensures the Vite dev server is running on port 1420.
/// If it is not responding, spawns `npm run dev` from the project root
/// and waits up to 15 seconds for the port to open.
/// No-op in release builds.
#[cfg(debug_assertions)]
fn ensure_vite_running(state: &Arc<AppState>) {
    use std::net::TcpStream;
    use std::time::Duration;

    const PROJECT_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let addr = "127.0.0.1:1420";

    // Check if Vite is already up
    if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(300)).is_ok() {
        return; // already running
    }

    log::warn!("Watchdog: Vite not responding on :1420 — spawning npm run dev");

    match std::process::Command::new("npm")
        .args(["run", "dev"])
        .current_dir(PROJECT_ROOT)
        .spawn()
    {
        Ok(child) => {
            if let Ok(mut guard) = state.vite_child.lock() {
                *guard = Some(child);
            }
        }
        Err(e) => {
            log::error!("Watchdog: failed to spawn npm run dev: {e}");
            return;
        }
    }

    // Poll until port 1420 responds, timeout 15s
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(200)).is_ok() {
            log::info!("Watchdog: Vite is up on :1420");
            break;
        }
        if std::time::Instant::now() >= deadline {
            log::error!("Watchdog: Vite did not come up within 15s");
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Confirm all tests still pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add watchdog task and dev Vite recovery

60s watchdog calls is_frontend_healthy(); on failure it reloads
the WebView. Dev builds also probe :1420 and spawn npm run dev
if Vite is not running. After 2 consecutive failures in production,
emits a native notification."
```

---

### Task 4: Exit handler + full wiring

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Wire `start_watchdog` into `setup`**

In `run()`, after the `start_update_checker` call, add:

```rust
// Start frontend heartbeat watchdog
start_watchdog(app.handle().clone(), state.clone());
```

- [ ] **Step 2: Replace `.run()` with `.build()`/`.run()` to handle exit**

Change the final two lines of the `tauri::Builder` chain:

```rust
        .run(tauri::generate_context!())
        .expect("error while running CSpy");
```

to:

```rust
        .build(tauri::generate_context!())
        .expect("error while building CSpy")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill Vite child process if we spawned it
                let state = app_handle.state::<Arc<AppState>>();
                if let Ok(mut guard) = state.vite_child.lock() {
                    if let Some(ref mut child) = *guard {
                        let _ = child.kill();
                        log::info!("Watchdog: killed Vite child on exit");
                    }
                }
            }
        });
```

- [ ] **Step 3: Verify it compiles cleanly**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Run all tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire watchdog and add exit handler for Vite cleanup

start_watchdog() registered in setup. RunEvent::Exit handler
kills any Vite child process spawned by the watchdog."
```

---

### Task 5: Frontend heartbeat emitter

**Files:**
- Modify: `src/routes/+page.svelte`

The Svelte app calls `invoke('heartbeat')` every 30 seconds. This pings the Rust command which updates `last_heartbeat`. Errors are silently swallowed — a failed heartbeat just means the watchdog will eventually trigger.

- [ ] **Step 1: Add the heartbeat interval to `onMount` and cleanup to `onDestroy`**

In `src/routes/+page.svelte`, make the following changes:

**Add `heartbeat` variable declaration** alongside the existing interval variable (`ticker`):

```typescript
let heartbeatTicker: ReturnType<typeof setInterval> | undefined;
```

**Add the heartbeat interval inside `onMount`** (after the countdown ticker setup, before the `try` block for `get_usage`):

```typescript
// Heartbeat — tells Rust the frontend is alive every 30s
heartbeatTicker = setInterval(() => {
    invoke('heartbeat').catch(() => {/* swallow — watchdog handles recovery */});
}, 30_000);
```

**Add cleanup in `onDestroy`**:

```typescript
if (heartbeatTicker) clearInterval(heartbeatTicker);
```

The full `onDestroy` block becomes:

```typescript
onDestroy(() => {
    unlistenUsage?.();
    unlistenError?.();
    if (ticker) clearInterval(ticker);
    if (heartbeatTicker) clearInterval(heartbeatTicker);
});
```

- [ ] **Step 2: Verify the frontend builds**

```bash
npm run build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 3: Run frontend tests**

```bash
npx vitest run 2>&1 | tail -5
```

Expected: `Tests 18 passed (18)`

- [ ] **Step 4: Run all Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: emit heartbeat from frontend every 30s

invoke('heartbeat') keeps the Rust watchdog satisfied while the
popover is healthy. Errors are swallowed — the watchdog handles
recovery if the frontend goes silent."
```

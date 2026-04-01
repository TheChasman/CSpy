# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What CSpy Is

A macOS menu bar app that monitors Claude AI subscription usage (5-hour and 7-day quotas) in real time. Built with Tauri 2 + Rust + SvelteKit + Svelte 5.

**Not** an API developer cost tracker. This reads the **subscriber** usage data that Claude.ai itself uses to show the usage bar.

## Commands

```bash
npm install              # Install JS deps (required after clone or lockfile change)
cargo tauri dev          # Dev mode — hot-reload Svelte + Rust rebuilds on change
cargo tauri build        # Production build → src-tauri/target/release/bundle/
npm run check            # SvelteKit sync + TypeScript type checking
cargo tauri icon <img>   # Generate all icon sizes from a source image
```

No test runner is configured yet. No linter is configured.

## Architecture

```
macOS Keychain ("Claude Code-credentials")
    │
    ▼  reads OAuth token via `security` CLI
┌──────────────────────┐
│  Rust backend         │
│  ├─ keychain.rs       │  Reads token from Keychain
│  ├─ usage.rs          │  GET api.anthropic.com/api/oauth/usage
│  └─ lib.rs            │  Tray icon, polling loop, Tauri commands
└──────┬───────────────┘
       │  events (usage-updated, usage-error) + invoke (get_usage, refresh_usage)
┌──────▼───────────────┐
│  Svelte popover       │  290×240 borderless window
│  └─ +page.svelte      │  Progress bars, countdowns, refresh button
└──────────────────────┘
```

### Rust ↔ Svelte communication

Two mechanisms:

1. **Tauri Commands (RPC)** — Svelte calls `invoke('get_usage')` or `invoke('refresh_usage')`. Defined in `lib.rs` with `#[tauri::command]`.
2. **Tauri Events (push)** — Rust emits `usage-updated` (with `UsageData` payload) and `usage-error` (with error string) from the background polling loop. Svelte listens via `listen()` in `onMount`.

### Polling loop

- Runs in a `tokio::spawn` task in `lib.rs`.
- Interval: `POLL_SECS = 180` (3 minutes).
- On success: caches `UsageData` in `AppState`, updates tray tooltip, emits `usage-updated`.
- On error: logs, emits `usage-error`, continues polling.

### Shared state

`AppState` holds `RwLock<Option<String>>` (cached OAuth token) and `RwLock<Option<UsageData>>` (last fetched data). Passed as Tauri managed state.

## Key Data Flow

1. On startup, Rust reads `"Claude Code-credentials"` from macOS Keychain via the `security` CLI binary (not the `keyring` crate — see Design Decisions)
2. Extracts `claudeAiOauth.accessToken` (sk-ant-oat01-...)
3. Polls `GET https://api.anthropic.com/api/oauth/usage` with header `anthropic-beta: oauth-2025-04-20`
4. API returns `{ five_hour: { utilization, resets_at }, seven_day: { ... } }`
5. Rust normalises to `UsageData` struct, updates tray tooltip, emits event to frontend
6. Tray left-click toggles the popover, positioned centred below the icon

## Design Decisions

1. **No Dock icon** — `LSUIElement=true`; menu bar only.
2. **Keychain via `security` CLI** — More reliable than `keyring` crate for generic-password items with unconventional account fields. See `keychain.rs`.
3. **3-minute poll interval** — Balances freshness with being a good API citizen.
4. **Colour tiers** — green (<60%), amber (60–84%), red (≥85%). Defined in both `usage.rs` (`worst_tier`) and `types.ts` (`tierFor`). Keep these in sync.
5. **Popover, not window** — Borderless, always-on-top, `skipTaskbar`. Positioned at `(trayX - width/2, trayY + 4)`.
6. **No persistence** — Stateless; re-reads Keychain on each launch. No database.
7. **Countdown refresh** — Frontend re-renders reset countdowns every 30s via `setInterval`.

## Conventions

- Rust structs use `snake_case` fields; Svelte types mirror them exactly (`five_hour`, `resets_at`).
- `UsageData` and `UsageBucket` are defined in both `usage.rs` (Rust) and `types.ts` (TypeScript) — changes must be kept in sync.
- Tauri capabilities in `src-tauri/capabilities/default.json` — any new window or plugin permission must be added there.
- Frontend uses Svelte 5 runes (`$state`, `$effect`, `$derived`) — not Svelte 4 stores.

## Prerequisites

- Claude Code installed and logged in (credentials must exist in macOS Keychain)
- Node.js (v20+)
- Rust stable toolchain + `cargo-tauri` CLI
- macOS 14+ (Sonoma)

## Toolchain Note

Rust toolchain is currently `stable-x86_64-apple-darwin` (Rosetta on M1).
For native ARM builds: `rustup target add aarch64-apple-darwin`

## What's NOT Done Yet

- [ ] Custom tray icon (currently uses default app icon as template)
- [ ] Notification on threshold crossing
- [ ] Click-away-to-dismiss popover behaviour
- [ ] Settings UI (poll interval, thresholds)
- [ ] Token refresh handling (if OAuth token expires)

# AGENTS.md

Instructions for AI coding assistants working in this repository.

## Project

CSpy is a macOS menu bar app (Tauri 2 + Rust + SvelteKit + Svelte 5) that monitors Claude AI subscription usage quotas. It reads an OAuth token from the macOS Keychain and polls the Anthropic usage API every 3 minutes.

## Commands

```bash
npm install              # Install frontend dependencies
cargo tauri dev          # Dev mode (hot-reload Svelte + Rust rebuild on change)
cargo tauri build        # Production build
npm run check            # TypeScript type checking
```

No test runner or linter is configured.

## Architecture

- **Rust backend** (`src-tauri/src/`): Keychain access, HTTP polling, tray icon, Tauri commands.
- **Svelte frontend** (`src/`): 290x240 borderless popover with progress bars and countdown timers.
- **Communication**: Rust exposes `get_usage` and `refresh_usage` as Tauri commands (invoked via `invoke()`). The background polling loop pushes `usage-updated` and `usage-error` events that Svelte listens to via `listen()`.

## Dual type definitions

`UsageData` and `UsageBucket` are defined in both Rust (`src-tauri/src/usage.rs`) and TypeScript (`src/lib/types.ts`). When modifying these structs, update both files to keep them in sync.

Colour tier thresholds (green < 60%, amber 60-84%, red >= 85%) are defined in both `usage.rs` (`worst_tier`) and `types.ts` (`tierFor`). Keep these in sync.

## Conventions

- **Svelte 5 runes** — use `$state`, `$effect`, `$derived`. Do not use Svelte 4 stores.
- **Tauri capabilities** — any new window operation or plugin permission must be added to `src-tauri/capabilities/default.json`.
- **Keychain access** — uses the macOS `security` CLI binary, not the `keyring` crate. See `keychain.rs` for why.
- **No Dock icon** — the app sets `ActivationPolicy::Accessory`. Do not add a main application window.
- **No persistence** — stateless by design. No database, no config files. Re-reads Keychain on each launch.
- **Polling interval** — `POLL_SECS = 180` in `lib.rs`. Do not reduce below 60 seconds.

## Spelling

Use UK English (e.g. `utilisation`, `colour`). Field names in Rust structs and TypeScript interfaces already follow this.

## What not to do

- Do not add a Dock icon or main window — this is a menu bar-only app.
- Do not store the OAuth token on disk — it comes from the Keychain at runtime.
- Do not change the `security` CLI approach to Keychain access without good reason; the `keyring` crate was tried and was less reliable for this use case.

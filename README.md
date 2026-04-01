# CSpy

A macOS menu bar app that shows your Claude AI subscription usage in real time — the same 5-hour and 7-day quotas that Claude.ai displays, pulled straight from the API.

CSpy is **not** an API developer cost tracker. It reads the **subscriber** usage data that the Claude.ai web app itself uses.

## How it works

CSpy reads the OAuth token that Claude Code stores in the macOS Keychain, then polls the Anthropic usage endpoint every 3 minutes. A small popover (toggled by clicking the tray icon) shows two progress bars with countdown timers until each quota resets.

```
macOS Keychain → OAuth token → GET /api/oauth/usage → tray icon + popover
```

### Colour tiers

| Utilisation | Colour |
|-------------|--------|
| < 60%       | Green  |
| 60–84%      | Amber  |
| ≥ 85%       | Red    |

## Prerequisites

- **macOS 14+** (Sonoma)
- **Claude Code** installed and logged in (its credentials must be in the macOS Keychain)
- **Node.js** v20+
- **Rust** stable toolchain with the `cargo-tauri` CLI (`cargo install tauri-cli`)

## Building from source

```bash
git clone git@github.com:TheChasman/CSpy.git
cd CSpy
npm install
cargo tauri build
```

The bundled `.app` and `.dmg` appear in `src-tauri/target/release/bundle/`.

## Development

```bash
npm install
cargo tauri dev
```

This starts the Vite dev server (port 1420) with hot-reload for the Svelte frontend, and recompiles the Rust backend on file changes.

## Stack

| Layer    | Technology                  |
|----------|-----------------------------|
| Backend  | Rust + Tauri 2              |
| Frontend | SvelteKit + Svelte 5        |
| HTTP     | reqwest                     |
| Keychain | macOS `security` CLI        |

## Licence

MIT

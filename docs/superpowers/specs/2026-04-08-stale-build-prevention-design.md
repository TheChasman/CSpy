# Stale Frontend Build Prevention

**Date:** 2026-04-08
**Status:** Approved
**Problem:** Running `target/debug/cspy` directly loads frontend from `build/` which can be stale or broken, causing a White Screen of Death (WSOD).

## Root Cause

1. Commit `5e5697e` added `src/routes/+page.test.ts` -- SvelteKit reserves `+`-prefixed files in routes, breaking `npm run build`.
2. `build/` froze at the last successful build (2026-04-03).
3. No safeguard detects a missing or stale `build/` directory during `cargo build`.
4. CI test workflow never runs `npm run build`, so the breakage went undetected.

## Changes

### 1. Rename test file

Rename `src/routes/+page.test.ts` to `src/routes/page.test.ts`. Drops the reserved `+` prefix while keeping the test colocated with the component. No import changes needed -- the test imports `./+page.svelte` which is unaffected. Vitest include pattern `src/**/*.test.ts` still matches.

### 2. `build.rs` auto-builds frontend

Enhance `src-tauri/build.rs` to check for `../build/index.html` relative to `CARGO_MANIFEST_DIR`. If missing, run `npm run build` from the project root.

Logic:
- Resolve project root from `CARGO_MANIFEST_DIR`
- Check if `build/index.html` exists
- If missing: print `cargo:warning`, run `npm run build`, assert exit success
- Emit `cargo:rerun-if-changed=<path to build/index.html>` so deletion triggers a rerun
- Call `tauri_build::build()` as before

Only fires when `build/` is absent (fresh clone, deleted directory). Does not re-run `npm run build` on every compile.

### 3. CI test workflow

Add `npm run build` step to `.github/workflows/test.yml` after `npm ci` and before lint/test steps. Catches frontend build failures on every PR and push to main.

Step order:
1. `npm ci`
2. `npm run build` (new)
3. Clippy
4. Rust tests
5. ESLint
6. Vitest

## Files Modified

| File | Change |
|------|--------|
| `src/routes/+page.test.ts` | Renamed to `src/routes/page.test.ts` |
| `src-tauri/build.rs` | Add frontend existence check and auto-build |
| `.github/workflows/test.yml` | Add `npm run build` step |

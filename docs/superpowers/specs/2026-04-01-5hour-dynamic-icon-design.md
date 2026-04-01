# CSpy Redesign: 5-Hour Rolling Window with Dynamic Tray Icon

**Date:** 2026-04-01  
**Scope:** MVP redesign focusing on 5-hour quota only, with colour-coded dynamic tray icon  
**Status:** Design approved

---

## Overview

CSpy currently shows both 5-hour and 7-day quotas in a popover window. This redesign removes the 7-day quota entirely and moves the primary usage feedback into the **tray icon itself**, which updates dynamically to show fill level and colour warning.

### Problem Statement
Current UX requires clicking through windows (Shift+Tab to Chrome, click window, find Claude tab, click it) to see usage. Too much friction for a quick status check.

### Solution
Show usage progress **visually in the menu bar icon** with colour feedback (green → amber → red), eliminating the need to open anything just to check status.

---

## Design

### 1. Tray Icon (Dynamic)

The tray icon is a **hollow rectangle** that fills from left to right, with colour-coded fill based on 5-hour utilisation:

- **0–69% utilisation:** Green fill
- **70–89% utilisation:** Amber fill
- **≥90% utilisation:** Red fill

**Properties:**
- Monochrome outline (black on light mode, white on dark mode)
- Coloured fill (green/amber/red)
- Regenerated on each usage update
- Hollow when 0% (empty rectangle with just outline)
- Fully filled when 100%

**Update Frequency:**
- Regenerated every time the backend polls the API (every 3 minutes by default)
- Also regenerated on manual refresh via popover

### 2. Popover Window (On Click)

Clicking the tray icon toggles a small popover showing:

```
┌──────────────────────────────┐
│ 5-hour quota                 │
│ [████████░░░░░░░░░░░░░]     │
│ 62% used · Resets in 2h 15m  │
│                              │
│ Burn rate: 12.4%/hr [●]      │
└──────────────────────────────┘
```

**Content:**
- Label: "5-hour quota"
- Progress bar (visual fill matching icon fill colour)
- Text: "XX% used · Resets in Xh Ym"
- Burn rate indicator:
  - Text: "Burn rate: XX.X%/hr" 
  - Colour dot (● in green/amber/red matching burn rate thresholds)
  - Colour logic:
    - Green: < 16%/hr (on track to ~80% in 5h)
    - Amber: 16% ≤ rate < 20%/hr (accelerating, watch it)
    - Red: ≥ 20%/hr (will exceed 80% buffer)

**Popover Behaviour:**
- 290×240 size (same as current, can be reduced if we remove extras)
- Borderless, transparent background
- Always-on-top, positioned below tray icon
- Toggle on click (click again to close)
- Hide on click outside (standard macOS popover behaviour)

**Error States:**
- If API fetch fails, show error banner (same as current)
- Display cached data with "⚠ Last refresh failed" warning

### 3. Data Flow

**Rust Backend:**
- Poll API every 3 minutes → `fetch_usage()` returns `UsageData`
- On success:
  1. Calculate fill level from `five_hour.utilisation` (0.0–1.0)
  2. Determine colour (green/amber/red based on thresholds: 0–69%, 70–89%, ≥90%)
  3. Render new tray icon with bar + colour
  4. Update `AppState.cached` with `UsageData`
  5. Emit `usage-updated` event to Svelte (include calculated burn rate)
  6. Update tray tooltip (optional: just the percentage, or keep full detail)

**Burn Rate Calculation:**
- `five_hour.resets_at` is an ISO 8601 timestamp (when the 5-hour window closes)
- Calculate time remaining: `secs_until_reset = resets_at.timestamp() - now().timestamp()`
- Calculate hourly burn: `burn_rate_pct = (utilisation * 100) / (secs_until_reset / 3600)`
- Example: 62% used with 2h 15m remaining = 62 / 2.25 = ~27.6%/hr

**Svelte Frontend:**
- Listen for `usage-updated` events (includes burn rate)
- Render popover content:
  - Bar + percentage + countdown (existing)
  - Burn rate + colour indicator (new)
- Update countdown every 30 seconds
- Recalculate burn rate on each countdown tick (percentage stays same, but hours remaining decreases)

### 4. Removed Content

- **7-day quota display** — entirely removed from both icon and popover
- **Popover complexity** — no manual refresh button, no error details (MVP)

### 5. Future (Out of Scope)

- **TODO:** Popover becomes full Claude usage dashboard (all quotas, history, billing button)
- **TODO:** Click-away-to-dismiss popover (currently toggle)
- **TODO:** Custom tray icon instead of template-based

---

## Technical Changes

### Rust Backend (`src-tauri/src/`)

**`icon.rs` (refactor):**
- Current `load_owl_icon()` loads a static PNG and converts to grayscale
- New `generate_usage_icon()` will:
  1. Take utilisation (0.0–1.0) and calculate fill width as percentage
  2. Determine colour (green/amber/red)
  3. Render RGBA bytes for a hollow rectangle with coloured fill
  4. Return `Image<'static>` for tray

**`lib.rs` (main loop):**
- In `start_polling()`, after `fetch_usage()` succeeds:
  - Call `icon::generate_usage_icon()` with utilisation value
  - Update tray icon with `tray.set_icon()`
  - Emit `usage-updated` event (unchanged)

**`usage.rs` (unchanged):**
- Still fetches 5-hour and 7-day data
- Filters to only use `five_hour` in rendering (ignore `seven_day`)

### Svelte Frontend (`src/`)

**`routes/+page.svelte`:**
- Remove 7-day section entirely
- Keep 5-hour bar + percentage + countdown
- Simplify layout to single bar (not two)

**`lib/types.ts`:**
- Keep `UsageData` and `UsageBucket` interfaces (both still have five_hour and seven_day fields to match Rust, but frontend only uses five_hour)
- Update `tierFor()` function (usage % colour):
  - Green: < 70%
  - Amber: ≥ 70% and < 90%
  - Red: ≥ 90%
- Add `burnRateTier()` function (burn rate colour):
  - Green: < 16%/hr
  - Amber: ≥ 16% and < 20%/hr
  - Red: ≥ 20%/hr
- Add `calculateBurnRate(utilisation, secondsUntilReset)` function:
  - Returns percentage per hour as a float
  - Example: `calculateBurnRate(0.62, 8100)` = ~27.6%/hr (2h 15m remaining)

**`src/app.css`:**
- Update bar colours to match icon thresholds

---

## Icon Rendering Details

**Size:** 16×16 pixels (standard macOS menu bar icon size)

**Structure:**
```
[X] = 16×16 canvas
Outline: 1px dark/light border around edges
Fill: Left-to-right bar proportional to utilisation
Example at 62%: 10 pixels filled, 6 pixels hollow
```

**Colour Mapping:**
- **Green:** `#4ade80` (used when utilisation < 70%)
- **Amber:** `#fbbf24` (used when utilisation 70–89%)
- **Red:** `#f87171` (used when utilisation ≥ 90%)
- **Outline/hollow:** Black (light mode) or white (dark mode)

**Implementation:**
- Use `image` crate (already in Cargo.toml) to render RGBA bytes
- Generate on startup and on every poll
- No file I/O — entirely in-memory RGBA generation

---

## Success Criteria

1. ✓ Tray icon shows coloured fill that updates every 3 minutes
2. ✓ Icon colour changes appropriately (green → amber → red based on usage %)
3. ✓ Clicking icon opens popover with:
   - Bar + percentage + reset time
   - Burn rate (XX%/hr) with colour indicator
4. ✓ Burn rate colour changes appropriately (green → amber → red based on %/hr)
5. ✓ 7-day quota removed from display entirely
6. ✓ Error handling preserved (API failures show warning banner)
7. ✓ Single instance enforcement maintained

---

## Testing (Not Implemented in MVP)

- Manual: Check icon colour at various utilisation levels (0%, 50%, 75%, 95%)
- Manual: Verify popover displays correct reset time countdown
- Manual: Verify API rate-limit errors show warning banner
- Integration: Confirm icon updates after each poll

---

## Notes

- **Burn rate updates:** Calculated fresh on each countdown tick (every 30s in the popover). As time passes, even with fixed usage %, burn rate decreases (e.g., 62% with 2h left = 27.6%/hr, but 5 mins later it's 23.5%/hr). This is correct behaviour — shows acceleration/deceleration relative to reset time.
- **Popover size:** Can reduce from 290×240 to ~220×120 since we're only showing one bar + burn rate. Keep flexible for future dashboard expansion.
- **Tooltip support:** macOS doesn't support native tooltips on tray icons; that's why we use a popover instead.
- **Dynamic icons:** Already proven concept (Battery icon, others). Regenerating on each poll is acceptable at 3-minute intervals (minimal CPU).

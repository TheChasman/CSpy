/** Matches the Rust UsageData struct sent via Tauri events */
export interface UsageData {
	five_hour: UsageBucket | null;
	seven_day: UsageBucket | null;
	fetched_at: string;
}

export interface UsageBucket {
	/** 0.0 to 1.0 */
	utilisation: number;
	/** ISO 8601 reset timestamp, or null if not rate-limited */
	resets_at: string | null;
}

/** Colour tier thresholds */
export type Tier = 'green' | 'amber' | 'red';

export function tierFor(utilisation: number): Tier {
	if (utilisation >= 0.85) return 'red';
	if (utilisation >= 0.60) return 'amber';
	return 'green';
}

/** Format seconds remaining as "Xh Ym" */
export function formatCountdown(resetIso: string | null): string {
	if (!resetIso) return '—';
	const diff = new Date(resetIso).getTime() - Date.now();
	if (diff <= 0) return 'resetting…';
	const h = Math.floor(diff / 3_600_000);
	const m = Math.floor((diff % 3_600_000) / 60_000);
	return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

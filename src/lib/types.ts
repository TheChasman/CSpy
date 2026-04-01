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
	if (utilisation >= 0.90) return 'red';
	if (utilisation >= 0.70) return 'amber';
	return 'green';
}

/** Colour tier for burn rate (%/hr) */
export function burnRateTier(burnRatePercent: number): Tier {
	if (burnRatePercent >= 20) return 'red';
	if (burnRatePercent >= 16) return 'amber';
	return 'green';
}

/** Calculate burn rate in percentage per hour */
export function calculateBurnRate(utilisation: number, secondsUntilReset: number): number {
	if (secondsUntilReset <= 0) return 0;
	const hoursRemaining = secondsUntilReset / 3600;
	const usagePercent = utilisation * 100;
	return usagePercent / hoursRemaining;
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

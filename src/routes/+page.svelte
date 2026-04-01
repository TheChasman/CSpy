<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { type UsageData, type Tier, tierFor, formatCountdown } from '$lib/types';

	let usage: UsageData | null = $state(null);
	let error: string | null = $state(null);
	let loading = $state(true);
	let countdownKey = $state(0); // forces countdown re-render

	let unlisten: UnlistenFn | undefined;
	let ticker: ReturnType<typeof setInterval> | undefined;

	onMount(async () => {
		// Listen for Rust-side usage updates
		unlisten = await listen<UsageData>('usage-updated', (event) => {
			usage = event.payload;
			error = null;
			loading = false;
		});

		// Listen for errors
		await listen<string>('usage-error', (event) => {
			error = event.payload;
			loading = false;
		});

		// Tick countdowns every 30s
		ticker = setInterval(() => { countdownKey++; }, 30_000);

		// Request immediate fetch
		try {
			usage = await invoke<UsageData>('get_usage');
			loading = false;
		} catch (e) {
			error = String(e);
			loading = false;
		}
	});

	onDestroy(() => {
		unlisten?.();
		if (ticker) clearInterval(ticker);
	});

	async function refresh() {
		loading = true;
		try {
			usage = await invoke<UsageData>('refresh_usage');
			error = null;
		} catch (e) {
			error = String(e);
		}
		loading = false;
	}

	function pct(n: number): string {
		return `${Math.round(n * 100)}%`;
	}

	function tier(n: number): Tier {
		return tierFor(n);
	}
</script>

<div class="popover">
	<header>
		<span class="title">CSpy</span>
		<button class="refresh" onclick={refresh} disabled={loading} title="Refresh now">
			<svg class:spinning={loading} width="14" height="14" viewBox="0 0 24 24"
				fill="none" stroke="currentColor" stroke-width="2.5"
				stroke-linecap="round" stroke-linejoin="round">
				<path d="M21 12a9 9 0 1 1-6.22-8.56" />
				<polyline points="21 3 21 9 15 9" />
			</svg>
		</button>
	</header>

	{#if error && !usage}
		<div class="error-box">
			<span class="error-icon">⚠</span>
			<span>{error}</span>
		</div>
	{:else if usage}
		{#key countdownKey}
			<!-- 5-hour window -->
			<section class="bucket">
				<div class="bucket-header">
					<span class="bucket-label">5-hour</span>
					<span class="mono {tier(usage.five_hour?.utilisation ?? 0)}">
						{pct(usage.five_hour?.utilisation ?? 0)}
					</span>
				</div>
				<div class="bar-track">
					<div
						class="bar-fill {tier(usage.five_hour?.utilisation ?? 0)}"
						style="width: {pct(usage.five_hour?.utilisation ?? 0)}"
					></div>
				</div>
				<div class="bucket-footer dim mono">
					Resets in {formatCountdown(usage.five_hour?.resets_at ?? null)}
				</div>
			</section>

			<!-- 7-day window -->
			<section class="bucket">
				<div class="bucket-header">
					<span class="bucket-label">Weekly</span>
					<span class="mono {tier(usage.seven_day?.utilisation ?? 0)}">
						{pct(usage.seven_day?.utilisation ?? 0)}
					</span>
				</div>
				<div class="bar-track">
					<div
						class="bar-fill {tier(usage.seven_day?.utilisation ?? 0)}"
						style="width: {pct(usage.seven_day?.utilisation ?? 0)}"
					></div>
				</div>
				<div class="bucket-footer dim mono">
					Resets in {formatCountdown(usage.seven_day?.resets_at ?? null)}
				</div>
			</section>
		{/key}

		{#if error}
			<div class="stale-warning dim mono">⚠ Last refresh failed — showing cached data</div>
		{/if}
	{:else}
		<div class="loading">Reading Keychain…</div>
	{/if}

	<footer class="dim mono">
		{#if usage}
			Updated {new Date(usage.fetched_at).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })}
		{/if}
	</footer>
</div>

<style>
	.popover {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 260px;
	}

	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.title {
		font-weight: 700;
		font-size: 15px;
		letter-spacing: -0.3px;
	}

	.refresh {
		background: none;
		border: 1px solid var(--bar-bg);
		border-radius: 6px;
		color: var(--text-dim);
		cursor: pointer;
		padding: 4px 6px;
		display: flex;
		align-items: center;
		transition: color 0.2s, border-color 0.2s;
	}
	.refresh:hover { color: var(--text); border-color: var(--text-dim); }
	.refresh:disabled { opacity: 0.4; cursor: default; }

	.spinning {
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.bucket {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.bucket-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}

	.bucket-label {
		font-weight: 600;
		font-size: 12px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
	}

	.bucket-footer {
		font-size: 11px;
	}

	.error-box {
		background: rgba(248, 113, 113, 0.12);
		border: 1px solid rgba(248, 113, 113, 0.3);
		border-radius: var(--radius);
		padding: 8px 10px;
		font-size: 12px;
		display: flex;
		gap: 6px;
		align-items: flex-start;
	}
	.error-icon { flex-shrink: 0; }

	.stale-warning {
		font-size: 11px;
		color: var(--amber);
	}

	.loading {
		text-align: center;
		padding: 20px 0;
		color: var(--text-dim);
	}

	footer {
		text-align: right;
		font-size: 10px;
	}

	/* Colour utility classes for text */
	.green { color: var(--green); }
	.amber { color: var(--amber); }
	.red   { color: var(--red); }
</style>

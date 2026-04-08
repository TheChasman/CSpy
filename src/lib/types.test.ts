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
	const WINDOW = 5 * 3600;

	it('returns 0 when not enough elapsed time', () => {
		expect(calculateBurnRate(0.5, WINDOW - 30)).toBe(0);
	});

	it('calculates correctly for 50% over 2.5 hours', () => {
		const rate = calculateBurnRate(0.5, 9000);
		expect(rate).toBeCloseTo(20.0, 1);
	});

	it('calculates correctly for 10% over 1 hour', () => {
		const rate = calculateBurnRate(0.1, 14400);
		expect(rate).toBeCloseTo(10.0, 1);
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
		expect(formatCountdown(null)).toBe('\u2014');
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

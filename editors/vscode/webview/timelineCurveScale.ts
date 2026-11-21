import type { CurvePoint } from '../src/timelineProtocol';

const top = 8;
const bottom = 92;

export class CurveScale {
    private readonly factor: number;
    private readonly start: number;
    private readonly span: number;

    constructor(points: readonly CurvePoint[], keys: readonly number[]) {
        let low = Infinity;
        let high = -Infinity;
        for (const { value } of points) {
            low = Math.min(low, value);
            high = Math.max(high, value);
        }
        for (const value of keys) {
            low = Math.min(low, value);
            high = Math.max(high, value);
        }
        this.factor = Math.max(Math.abs(low), Math.abs(high), low === high ? 1 : Number.MIN_VALUE);
        const lower = low / this.factor;
        const upper = high / this.factor;
        const half = lower === upper ? .1 : (upper - lower) / 2;
        this.start = (upper + lower) / 2 - half;
        this.span = 2 * half;
    }

    percent(value: number): number {
        return bottom - (bottom - top) * (value / this.factor - this.start) / this.span;
    }

    fromOffset(value: number, offsetPercent: number): number {
        const next = (value / this.factor - offsetPercent / (bottom - top) * this.span) * this.factor;
        // Keep narrow ranges editable even when the values are large and nearly equal.
        const digits = Math.max(7, Math.min(17, Math.ceil(4 - Math.log10(this.span))));
        return Number(next.toPrecision(digits));
    }

    get bounds(): [number, number] {
        return [this.start * this.factor, (this.start + this.span) * this.factor];
    }
}

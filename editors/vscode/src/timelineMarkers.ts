import type { TimelineMarker, TimelineMarkerEdit } from './timelineProtocol';

/** Markers belong to the current design and authored clip, outside the source transaction. */
export class TimelineMarkers {
    private readonly clocks = new Map<string, Map<string, number>>();

    list(clock: string, duration: number): TimelineMarker[] {
        const markers = this.clocks.get(clock);
        if (!markers) return [];
        return [...markers].filter(([, time]) => time <= duration).map(([name, time]) => ({ name, time }))
            .sort((left, right) => left.time - right.time || left.name.localeCompare(right.name));
    }

    time(clock: string, duration: number, name: string): number {
        const time = typeof name === 'string' ? this.clocks.get(clock)?.get(name.trim()) : undefined;
        if (time === undefined || time > duration) throw new Error('Choose a marker within this preview clock.');
        return time;
    }

    edit(clock: string, duration: number, edit: TimelineMarkerEdit): void {
        const name = typeof edit.name === 'string' ? edit.name.trim() : '';
        if (!name || name.length > 48 || /[\u0000-\u001f\u007f]/.test(name)) throw new Error('Enter a marker name of up to 48 characters.');
        if (edit.action === 'set' && (!Number.isFinite(edit.time) || edit.time < 0 || edit.time > duration || edit.time * 10_000_000 >= 2 ** 63)) {
            throw new Error('Choose a marker time within the preview clock.');
        }
        const markers = this.clocks.get(clock) ?? new Map<string, number>();
        if (edit.action === 'remove') {
            if (!markers.delete(name)) throw new Error('Choose a marker from this preview clock.');
        } else if (edit.action === 'set') {
            if (markers.size >= 64 && !markers.has(name)) throw new Error('Remove a marker before adding another.');
            markers.set(name, edit.time);
        } else throw new Error('Choose a marker action.');
        if (markers.size) this.clocks.set(clock, markers);
        else this.clocks.delete(clock);
    }
}

import type { KeyframeLocation, TimelineTrack } from '../src/timelineProtocol';

export const keyframeId = (key: KeyframeLocation): string => JSON.stringify([key.target, key.property, key.time]);

export class TimelineSelection {
    private readonly selected = new Map<string, KeyframeLocation>();
    private anchor?: string;

    get keys(): KeyframeLocation[] { return [...this.selected.values()]; }
    get size(): number { return this.selected.size; }
    has(key: KeyframeLocation): boolean { return this.selected.has(keyframeId(key)); }

    replace(keys: readonly KeyframeLocation[]): void {
        this.selected.clear();
        for (const key of keys) this.selected.set(keyframeId(key), key);
        const last = keys.at(-1);
        this.anchor = last ? keyframeId(last) : undefined;
    }

    pick(key: KeyframeLocation, ordered: readonly KeyframeLocation[], extend: boolean, toggle: boolean): void {
        const id = keyframeId(key);
        if (extend && this.anchor) {
            const start = ordered.findIndex(candidate => keyframeId(candidate) === this.anchor);
            const end = ordered.findIndex(candidate => keyframeId(candidate) === id);
            if (start >= 0 && end >= 0) {
                if (!toggle) this.selected.clear();
                for (const candidate of ordered.slice(Math.min(start, end), Math.max(start, end) + 1)) {
                    this.selected.set(keyframeId(candidate), candidate);
                }
                return;
            }
        }
        if (!toggle) this.selected.clear();
        if (toggle && this.selected.has(id)) this.selected.delete(id);
        else this.selected.set(id, key);
        this.anchor = id;
    }

    reconcile(tracks: readonly TimelineTrack[]): void {
        const exact = new Map<string, TimelineTrack>();
        const unambiguous = new Map<string, TimelineTrack | null>();
        for (const track of tracks) {
            exact.set(JSON.stringify([track.target, track.property]), track);
            unambiguous.set(track.property, unambiguous.has(track.property) ? null : track);
        }
        const accepted: KeyframeLocation[] = [];
        for (const key of this.selected.values()) {
            const track = exact.get(JSON.stringify([key.target, key.property])) ?? unambiguous.get(key.property);
            if (!track) continue;
            const time = acceptedTime(track, key.time);
            if (time !== undefined) accepted.push({ target: track.target, property: track.property, time });
        }
        this.replace(accepted);
    }
}

function acceptedTime(track: TimelineTrack, wanted: number): number | undefined {
    let low = 0;
    let high = track.keyframes.length;
    while (low < high) {
        const middle = Math.floor((low + high) / 2);
        if (track.keyframes[middle].time < wanted) low = middle + 1;
        else high = middle;
    }
    const before = track.keyframes[low - 1]?.time;
    const after = track.keyframes[low]?.time;
    const nearest = before !== undefined && (after === undefined || wanted - before < after - wanted) ? before : after;
    // Source round-trips through native XAML KeyTime at 100 ns precision.
    if (nearest !== undefined && Math.abs(nearest - wanted) <= 0.0000001) return nearest;
}

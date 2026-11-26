import type { KeyframeLocation, TimelineTrack } from '../src/timelineProtocol';
import { acceptedKeyframe } from '../src/timelineKeyframes';

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
            const match = acceptedKeyframe(track, key.time);
            if (match) accepted.push({ target: track.target, property: track.property, time: match.time });
        }
        this.replace(accepted);
    }
}

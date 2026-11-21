import type { TimelineKeyframe, TimelineTrack } from './timelineProtocol';

export function acceptedKeyframe(track: TimelineTrack, wanted: number): TimelineKeyframe | undefined {
    let low = 0;
    let high = track.keyframes.length;
    while (low < high) {
        const middle = Math.floor((low + high) / 2);
        if (track.keyframes[middle].time < wanted) low = middle + 1;
        else high = middle;
    }
    const before = track.keyframes[low - 1];
    const after = track.keyframes[low];
    const nearest = before && (!after || wanted - before.time < after.time - wanted) ? before : after;
    // Source round-trips through native XAML KeyTime at 100 ns precision.
    if (nearest && Math.abs(nearest.time - wanted) <= 0.0000001) return nearest;
}

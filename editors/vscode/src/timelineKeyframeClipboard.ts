import * as vscode from 'vscode';
import { choose, editTime, type StudioContext } from './studioContext';
import type { AuthoredTimeline, KeyframeClipboard, KeyframeDestination, KeyframeLocation, TimelineProgram, TimelineTrack } from './timelineProtocol';

const maxKeys = 4096;
const maxBytes = 1024 * 1024;

export async function copyTimelineKeys(context: StudioContext, program: number, keyframes: KeyframeLocation[]): Promise<void> {
    const clipboard = await context.read<KeyframeClipboard>('designer_keyframe_clipboard', { program, keyframes });
    await writeTimelineClipboard(JSON.stringify(clipboard), keyframes.length);
}

export async function writeTimelineClipboard(text: string, count: number): Promise<void> {
    await vscode.env.clipboard.writeText(text);
    void vscode.window.setStatusBarMessage(`Copied ${count} FerroUI keyframe${count === 1 ? '' : 's'}`, 3000);
}

export async function timelineEditTime(context: StudioContext, program: TimelineProgram, duration: number, operation: string): Promise<number | undefined> {
    const pose = context.panel.authoredTimelinePose;
    if (!program.preview?.native) return pose?.program === program.id ? pose.time : 0;
    const time = await editTime(`${operation} time on the authored timeline in seconds`, duration, 0);
    context.ensure();
    return time;
}

export async function prepareTimelinePaste(context: StudioContext, program: TimelineProgram, timeline: AuthoredTimeline,
    action: { reverse: boolean; track?: number }, available: readonly number[]) {
    const clipboard = await readTimelineClipboard();
    context.ensure();
    const destinations = await chooseKeyframeDestinations(clipboard, timeline.tracks, available, target => {
        const node = context.panel.nodes.find(node => node.id === target);
        return node?.name ?? node?.label;
    }, () => context.ensure(), action.track);
    if (!destinations) return;
    const time = await timelineEditTime(context, program, timeline.duration, 'Paste');
    if (time !== undefined) return { clipboard, destinations, time, reverse: action.reverse };
}

export async function chooseKeyframeDestinations(clipboard: KeyframeClipboard, tracks: TimelineTrack[],
    available: readonly number[], label: (target: number) => string | undefined, ensure: () => void,
    preferred?: number): Promise<KeyframeDestination[] | undefined> {
    if (clipboard.tracks.length > available.length) throw new Error('Show and unlock a compatible destination track for each copied track.');
    const used = new Set<number>();
    const destinations: KeyframeDestination[] = [];
    for (const [offset, copied] of clipboard.tracks.entries()) {
        const kind = copied.keyframes[0].value.kind;
        const choices = [...new Set(available)].flatMap(index => {
            const track = tracks[index];
            if (used.has(index) || track?.keyframes[0]?.value_kind !== kind || (kind === 'Discrete' && track.property !== copied.property)) return [];
            return [{ label: track.property, description: label(track.target) ?? `Target ${track.target}`, index, track }];
        }).sort((a, b) => Number(b.track.property === copied.property) - Number(a.track.property === copied.property));
        if (!choices.length) throw new Error(`Show and unlock a compatible destination for ${copied.property} (${kind}).`);
        const focused = clipboard.tracks.length === 1 ? choices.find(choice => choice.index === preferred) : undefined;
        const selected = focused ?? await choose(`Paste ${copied.property} · Track ${offset + 1} of ${clipboard.tracks.length}`, choices);
        ensure();
        if (!selected) return;
        used.add(selected.index);
        destinations.push({ target: selected.track.target, property: selected.track.property });
    }
    return destinations;
}

export async function readTimelineClipboard(): Promise<KeyframeClipboard> {
    return parseClipboard(await vscode.env.clipboard.readText());
}

function parseClipboard(text: string): KeyframeClipboard {
    if (Buffer.byteLength(text) > maxBytes) throw new Error('The keyframe clipboard exceeds 1 MiB.');
    const invalid = () => new Error('Copy FerroUI keyframes from the Timeline before pasting.');
    let clipboard: Partial<KeyframeClipboard>;
    try {
        clipboard = JSON.parse(text);
    } catch {
        throw invalid();
    }
    if (clipboard?.format !== 'ferroui.keyframes.v1' || !Array.isArray(clipboard.tracks)
        || clipboard.tracks.length === 0 || clipboard.tracks.length > maxKeys) throw invalid();
    let total = 0;
    for (const track of clipboard.tracks) {
        if (typeof track?.property !== 'string' || !Array.isArray(track.keyframes) || !track.keyframes.length) throw invalid();
        total += track.keyframes.length;
        if (total > maxKeys || track.keyframes.some(key => !Number.isFinite(key?.time) || typeof key?.value?.kind !== 'string')) throw invalid();
    }
    return { format: clipboard.format, tracks: clipboard.tracks };
}

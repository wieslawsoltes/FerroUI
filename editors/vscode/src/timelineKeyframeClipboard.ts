import * as vscode from 'vscode';
import { choose, editTime, type StudioContext } from './studioContext';
import type { AuthoredTimeline, KeyframeLocation, TimelineProgram, TimelineTrack } from './timelineProtocol';

interface CopiedKey { time: number; value: { kind: string; value: unknown }; easing: unknown }
interface CopiedTrack { property: string; keyframes: CopiedKey[] }
interface KeyframeClipboard { format: 'ferroui.keyframes.v1'; tracks: CopiedTrack[] }
type Destination = Pick<KeyframeLocation, 'target' | 'property'>;

const maxKeys = 4096;
const maxBytes = 1024 * 1024;

export async function copyTimelineKeys(context: StudioContext, program: number, keyframes: KeyframeLocation[]): Promise<void> {
    const clipboard = await context.read<KeyframeClipboard>('designer_keyframe_clipboard', { program, keyframes });
    await vscode.env.clipboard.writeText(JSON.stringify(clipboard));
    void vscode.window.setStatusBarMessage(`Copied ${keyframes.length} FerroUI keyframe${keyframes.length === 1 ? '' : 's'}`, 3000);
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
    const clipboard = parseClipboard(await vscode.env.clipboard.readText());
    context.ensure();
    const destinations = await chooseDestinations(context, clipboard, timeline.tracks, available, action.track);
    if (!destinations) return;
    const time = await timelineEditTime(context, program, timeline.duration, 'Paste');
    if (time !== undefined) return { clipboard, destinations, time, reverse: action.reverse };
}

async function chooseDestinations(context: StudioContext, clipboard: KeyframeClipboard, tracks: TimelineTrack[],
    available: readonly number[], preferred?: number): Promise<Destination[] | undefined> {
    if (clipboard.tracks.length > available.length) throw new Error('Show and unlock a compatible destination track for each copied track.');
    const used = new Set<number>();
    const destinations: Destination[] = [];
    for (const [offset, copied] of clipboard.tracks.entries()) {
        const kind = copied.keyframes[0].value.kind;
        const choices = [...new Set(available)].flatMap(index => {
            const track = tracks[index];
            if (used.has(index) || track?.keyframes[0]?.value_kind !== kind || (kind === 'Discrete' && track.property !== copied.property)) return [];
            const node = context.panel.nodes.find(node => node.id === track.target);
            return [{ label: track.property, description: node?.name ?? node?.label ?? `Target ${track.target}`, index, track }];
        }).sort((a, b) => Number(b.track.property === copied.property) - Number(a.track.property === copied.property));
        if (!choices.length) throw new Error(`Show and unlock a compatible destination for ${copied.property} (${kind}).`);
        const focused = clipboard.tracks.length === 1 ? choices.find(choice => choice.index === preferred) : undefined;
        const selected = focused ?? await choose(`Paste ${copied.property} · Track ${offset + 1} of ${clipboard.tracks.length}`, choices);
        context.ensure();
        if (!selected) return;
        used.add(selected.index);
        destinations.push({ target: selected.track.target, property: selected.track.property });
    }
    return destinations;
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

import type { HostedAnimationProgram, HostedAnimationTarget, HostedKeyframeCommand, HostedKeyframeGroupAction } from './hostedAnimationProtocol';
import { chooseTimelineKeyAction } from './studioKeyframes';
import { chooseKeyframeDestinations, readTimelineClipboard } from './timelineKeyframeClipboard';

export async function prepareHostedKeyframeGroup(
    { keys, pivot, edit }: HostedKeyframeGroupAction, program: HostedAnimationProgram,
    targets: readonly HostedAnimationTarget[], ensure: () => void,
): Promise<HostedKeyframeCommand | undefined> {
    const tracks = program.authoring?.tracks ?? [];
    if (!tracks.length) throw new Error('Choose an animation with authored keyframes.');
    const picked = edit ?? await chooseTimelineKeyAction(keys.length, true, true, true, pivot, 'Authored Time');
    if (!picked) return;
    ensure();
    if (picked.action === 'paste_keyframes') {
        const clipboard = await readTimelineClipboard();
        ensure();
        const destinations = await chooseKeyframeDestinations(clipboard, tracks, tracks.map((_, index) => index),
            id => targets.find(target => target.id === id)?.label, ensure, picked.track);
        if (destinations) return { action: picked.action, paste: { clipboard, destinations, time: pivot, reverse: picked.reverse } };
        return;
    }
    if (picked.action === 'mirror_keyframes') return { action: picked.action, pivot, keyframes: keys };
    return { ...picked, keyframes: keys };
}

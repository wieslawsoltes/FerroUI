import { strict as assert } from 'node:assert';
import type { HostedKeyframeCommand } from '../src/hostedAnimationProtocol';
import type { HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { KeyframeClipboard } from '../src/timelineProtocol';
import { animationFixture } from './hostedAnimation';
import { applyClipDraft } from './hostedClips';

export async function verifyHostedKeyframeGroups(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const authored = animationFixture(0, `<!-- Preserve the view 🧪 -->
    <Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
            <!-- Keep the animation note -->
            <KeyFrame Cue="0%"><Setter Property="CornerRadius" Value="0"/><Setter Property="Opacity" Value="0.4"/></KeyFrame>
            <KeyFrame Cue="20%"><Setter Property="CornerRadius" Value="12"/><Setter Property="Opacity" Value="0.7"/></KeyFrame>
            <KeyFrame Cue="40%" KeySpline="0.2,0.3,0.8,1.2"><Setter Property="Opacity" Value="0.8"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="32"/><Setter Property="Opacity" Value="1"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`).replaceAll('\n', '\r\n');
    const original = await preview.renderHosted(update(170, authored));
    assert.equal(original.animations?.keyframeSelection, true);
    const [program] = original.animations!.programs;
    const tracks = program.authoring!.tracks;
    assert.deepEqual(tracks.map(track => track.property), ['CornerRadius', 'Opacity']);
    const [corner, opacity] = tracks;
    const location = ({ target, property }: typeof corner, time: number) => ({ target, property, time });
    const keys = [location(corner, .4), location(opacity, .4), location(opacity, .8)];
    const sample = await preview.sampleHostedAnimation({ revision: original.revision, sequence: original.sequence,
        program: program.id, elapsed: 1.25, restart: true });
    const plan = (command: HostedKeyframeCommand) => preview.planHostedKeyframeGroup(sample.frame, { program: program.id, command });
    await assert.rejects(preview.planHostedKeyframeGroup(original, {
        program: program.id, command: { action: 'move_keyframes', keyframes: keys, offset: .2 },
    }), /frame changed/i);
    await assert.rejects(plan({ action: 'move_keyframes', keyframes: [keys[1]], offset: .4 }), /occupies a destination/i);
    assert.deepEqual((await plan({ action: 'move_keyframes', keyframes: keys, offset: 0 })).patches, []);
    const copied = await plan({ action: 'copy_keyframes', keyframes: keys });
    assert.deepEqual(copied.patches, []);
    const clipboard = JSON.parse(copied.clipboard!) as KeyframeClipboard;
    assert.deepEqual(clipboard.tracks.map(track => [track.property, track.keyframes.map(key => key.value.kind)]),
        [['CornerRadius', ['CornerRadius']], ['Opacity', ['Number', 'Number']]]);
    const moved = await plan({ action: 'move_keyframes', keyframes: keys, offset: .2 });
    assert.deepEqual(moved.keys?.map(key => key.time), [.6, .6, 1]);
    const duplicated = await plan({ action: 'duplicate_keyframes', keyframes: keys, offset: .1 });
    assert.deepEqual(duplicated.keys?.map(key => key.time), [.5, .5, .9]);
    const scaled = await plan({ action: 'scale_keyframes', keyframes: keys, pivot: 0, factor: 1.5 });
    assert.deepEqual(scaled.keys?.map(key => key.time), [.6, .6, 1.2]);
    const mirrored = await plan({ action: 'mirror_keyframes', keyframes: keys, pivot: .7 });
    assert.deepEqual(mirrored.keys?.map(key => key.time), [1, 1, .6]);
    const deleted = await plan({ action: 'remove_keyframes', keyframes: [location(corner, 0), location(opacity, 2)] });
    assert.deepEqual(deleted.keys, []);
    assert.doesNotMatch(applyClipDraft(authored, deleted.patches), /Property="CornerRadius" Value="0"|Property="Opacity" Value="1"/);
    const all = tracks.flatMap(track => track.keyframes.map(key => location(track, key.time)));
    const removed = await plan({ action: 'remove_keyframes', keyframes: all });
    assert.doesNotMatch(applyClipDraft(authored, removed.patches), /<KeyFrame/);
    const pasted = await plan({ action: 'paste_keyframes', paste: {
        clipboard, destinations: tracks.map(({ target, property }) => ({ target, property })), time: 1.25, reverse: true,
    } });
    assert.deepEqual(pasted.keys?.map(key => key.time).sort((a, b) => a - b), [1.25, 1.65, 1.65]);
    const changed = applyClipDraft(authored, moved.patches);
    assert.match(changed, /<!-- Preserve the view 🧪 -->/u);
    assert.match(changed, /<!-- Keep the animation note -->/);
    assert.doesNotMatch(changed, /(?<!\r)\n/);
    const position = { program: program.id, elapsed: 1.25, playing: false, completed: false };
    const replacement = await preview.renderHosted({ ...update(171, changed),
        restoreAnimation: { frame: sample.frame, presentation: { selection: program.id, position, overrides: {} } } });
    const current = replacement.animations!.programs[0];
    assert.deepEqual(current.authoring?.tracks.map(track => track.keyframes.map(key => key.time)), [[0, .6, 2], [0, .6, 1, 2]]);
    assert.equal(replacement.animationPresentation?.position?.elapsed, 1.25);
    const restored = await preview.renderHosted({ ...update(172, authored), restoreAnimation: {
        frame: replacement, presentation: { selection: current.id, position: { ...position, program: current.id }, overrides: {} },
    } });
    assert.equal(restored.image.png, sample.frame.image.png, 'Undo reuses the exact paused pose on the restored source');
    console.log('App Preview grouped, transferred, and removed typed keys across tracks and restored the native pose');
}

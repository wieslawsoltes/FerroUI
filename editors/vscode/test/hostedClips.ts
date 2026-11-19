import { strict as assert } from 'node:assert';
import type { HostedAnimationControl, HostedSourcePatch } from '../src/hostedAnimationProtocol';
import { HostedAnimations } from '../src/hostedAnimations';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture, cornerKeys } from './hostedAnimation';
import { until } from './wait';

export function applyClipDraft(source: string, patches: HostedSourcePatch[]): string {
    assert.equal(patches.length, 1, 'Arrange all clips with one checked source patch');
    const [{ start, end, expected, replacement }] = patches;
    assert.equal(source.slice(start, end), expected);
    return source.slice(0, start) + replacement + source.slice(end);
}

const clips = `<Window.Styles>
    <!-- Preserve this 🧭 note and CRLF. -->
    <Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" Delay="0:0:1" SpeedRatio="2" FillMode="Both">${cornerKeys}</Animation>
    </Style.Animations></Style>
    <Style Selector="Border"><Style.Animations>
        <Animation Duration="0:0:2" Delay="0:0:0" FillMode="Both">
            <KeyFrame Cue="0%"><Setter Property="Width" Value="180"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="Width" Value="220"/></KeyFrame>
        </Animation>
    </Style.Animations></Style>
</Window.Styles>`;

export async function verifyHostedClips(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = animationFixture(0, clips).replaceAll('\n', '\r\n');
    const errors: unknown[] = [];
    let displayed: HostedFrame | undefined;
    let previous: HostedFrame | undefined;
    const positions: number[] = [];
    const present = (frame: HostedFrame) => {
        previous = displayed;
        displayed = frame;
        animations.accept(frame);
        if (animations.state.position?.previewRange?.loop) positions.push(animations.state.position.elapsed);
    };
    const animations = new HostedAnimations(request => preview.sampleHostedAnimation(request), present, () => {}, error => errors.push(error));
    const control = (action: HostedAnimationControl) => animations.control(animations.state.frame!, action, AbortSignal.timeout(12_000));
    const first = await preview.renderHosted(update(160, source));
    present(first);
    const [corner, width] = animations.state.programs;
    assert.ok(corner?.customAnimator && width?.clip && first.animations?.clipScope);
    assert.deepEqual(corner.clip, { start: .5, end: 1.5, editable: true });
    assert.deepEqual(width.clip, { start: 0, end: 2, editable: true });
    const programs = [width.id, corner.id];
    await control({ action: 'scrub_all', elapsed: 1 });
    const paused = displayed!;
    const frame = animations.state.frame!;
    assert.deepEqual(animations.state.position?.clips, [corner.id, width.id]);
    await assert.rejects(preview.sampleHostedAnimation({ ...frame, clips: [{ program: corner.id }, { program: corner.id }],
        elapsed: 1, restart: true }), /distinct current native clips/i);
    await assert.rejects(preview.planHostedClipPlacement({ revision: first.revision, sequence: first.sequence }, { programs }), /frame changed/i);
    await assert.rejects(preview.planHostedClipPlacement(frame, { programs: [width.id, width.id] }), /distinct current clips/i);
    const draft = await preview.planHostedClipPlacement(frame, { programs, start: .25, gap: -.75 });
    const edited = applyClipDraft(source, draft.patches);
    assert.equal(edited, source.replace('Delay="0:0:1"', 'Delay="0:0:3"').replace('Delay="0:0:0"', 'Delay="0:0:0.2500000"'));
    animations.beforeSourceUpdate();
    const rebuilt = await preview.renderHosted({ ...update(161, edited), restoreAnimation: animations.restore });
    assert.equal(rebuilt.animationPresentation?.position?.elapsed, 1);
    assert.equal(rebuilt.animationPresentation?.position?.clips?.length, 2);
    assert.deepEqual(rebuilt.animations?.programs.map(program => program.clip?.start), [1.5, .25]);
    present(rebuilt);
    await control({ action: 'play_all' });
    await until(() => (animations.state.position?.elapsed ?? 0) > 1);
    await control({ action: 'pause' });
    assert.equal(animations.state.position?.playing, false);
    await control({ action: 'stop' });
    assert.equal(displayed!.image.png, first.image.png);
    const before = await preview.renderHosted(update(162, animationFixture(8).replace('Width="180"', 'Width="200"')));
    const after = await preview.renderHosted(update(163, animationFixture(0).replace('Width="180"', 'Width="195"')));
    assert.equal(paused.image.png, before.image.png, 'One native frame contains both sampled properties');
    assert.equal(rebuilt.image.png, after.image.png, 'The first replacement frame is already posed at the retained time');

    const rangeFrame = await preview.renderHosted(update(164, edited));
    present(rangeFrame);
    const program = rangeFrame.animations!.programs[1].id;
    const range = { start: .5, end: .75, loop: false };
    await assert.rejects(control({ action: 'play_range', program, range: { ...range, end: 3 } }), /within the preview clock/);
    const started = await control({ action: 'play_range', program, range });
    assert.equal(started.position?.elapsed, .5);
    await until(() => animations.state.position?.completed === true);
    assert.equal(animations.state.position?.elapsed, .75, 'A one-shot retains its exact Out pose');
    const out = displayed!;
    await assert.rejects(animations.control(previous!, { action: 'seek', program, elapsed: .5 }, AbortSignal.timeout(5000)), /current app-hosted preview frame/i);
    await animations.control(previous!, { action: 'pause' }, AbortSignal.timeout(5000));
    const loop = { ...range, loop: true };
    await control({ action: 'play_range', range: loop });
    await until(() => positions.some((elapsed, index) => index > 0 && elapsed < positions[index - 1]));
    await control({ action: 'pause' });
    assert.ok(positions.every(elapsed => elapsed >= loop.start && elapsed <= loop.end));
    const loopPose = displayed!;
    const time = animations.state.position!.elapsed;
    animations.beforeSourceUpdate();
    const replacement = await preview.renderHosted({ ...update(165, edited.replace('Preserve this', 'Retain this')), restoreAnimation: animations.restore });
    assert.deepEqual(replacement.animationPresentation?.position?.previewRange, loop);
    assert.equal(replacement.animationPresentation?.position?.elapsed, time);
    assert.equal(replacement.image.png, loopPose.image.png, 'A replacement shows the retained range pose as its first frame');
    present(replacement);
    const widthId = replacement.animations!.programs[1].id;
    await control({ action: 'play_range', program: widthId, range: loop });
    const shorterSource = edited.replace('Duration="0:0:2" Delay="0:0:0.2500000"', 'Duration="0:0:0.4" Delay="0:0:0.2500000"');
    animations.beforeSourceUpdate();
    const shorter = await preview.renderHosted({ ...update(166, shorterSource), restoreAnimation: animations.restore });
    assert.equal(shorter.animationPresentation?.position?.playing, true);
    assert.deepEqual(shorter.animationPresentation?.position?.previewRange, { ...loop, end: .65 });
    present(shorter);
    await control({ action: 'pause' });
    await control({ action: 'stop' });
    const expected = await preview.renderHosted(update(167, animationFixture(0).replace('Width="180"', 'Width="190"')));
    assert.equal(out.image.png, expected.image.png, 'Out is a rendered app-native pose');
    assert.deepEqual(errors, []);
    console.log('App Preview combined clips, applied native delays, looped and held preview ranges, and matched the first replacement pose');
}

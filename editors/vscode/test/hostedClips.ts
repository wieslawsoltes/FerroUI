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
    const present = (frame: HostedFrame) => { displayed = frame; animations.accept(frame); };
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
    assert.deepEqual(errors, []);
    console.log('App Preview combined native clips matched rendered poses, sequenced overlapping XAML delays, and rejected retired frames');
}

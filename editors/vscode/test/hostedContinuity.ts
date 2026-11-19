import { strict as assert } from 'node:assert';
import type { HostedAnimationControl } from '../src/hostedAnimationProtocol';
import { HostedAnimations } from '../src/hostedAnimations';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture, cornerKeys } from './hostedAnimation';
import { until } from './wait';

const source = animationFixture(0, `<Window.Resources>
    <Animation x:Key="ReviewCorners" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Both">
        <!--ferroui-designer-program-id:2ab63f50-de82-47d1-9c0b-060a7e6610e1-->${cornerKeys}
    </Animation>
    <Animation x:Key="IdleCorners" x:SetterTargetType="Border" Duration="0:0:2">${cornerKeys}</Animation>
</Window.Resources>`);

function clip(frame: HostedFrame, name: string) {
    const program = frame.animations?.programs.find(program => program.name === name);
    assert.ok(program, `Found the native ${name} clip`);
    return program;
}

export async function verifyHostedContinuity(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    let displayed: HostedFrame;
    const errors: unknown[] = [];
    const present = (frame: HostedFrame) => { displayed = frame; animations.accept(frame); };
    const animations = new HostedAnimations(request => preview.sampleHostedAnimation(request), present, () => {}, error => errors.push(error));
    const control = (action: HostedAnimationControl) => animations.control(animations.state.frame!, action, AbortSignal.timeout(12_000));
    const propose = (revision: number, source: string) => preview.renderHosted({ ...update(revision, source), restoreAnimation: animations.restore });
    try {
        const first = await propose(16, source);
        present(first);
        const review = clip(first, 'ReviewCorners');
        const idle = clip(first, 'IdleCorners');
        const card = first.animations?.targets.find(target => target.label === 'ReviewCard');
        assert.ok(card);
        await control({ action: 'targets', program: review.id, targets: [card.id] });
        await control({ action: 'seek', program: review.id, elapsed: 1 });
        await control({ action: 'select', program: idle.id });
        const paused = displayed!;
        assert.notEqual(paused.image.png, first.image.png);

        animations.beforeSourceUpdate();
        assert.equal(animations.state.frame, undefined);
        assert.equal(animations.state.position?.elapsed, 1, 'Keep the paused playhead while replacing the source');
        const late = await preview.sampleHostedAnimation({ revision: paused.revision, sequence: paused.sequence,
            program: review.id, targets: [card.id], elapsed: 1.25, restart: false });
        assert.notEqual(late.frame.image.png, paused.image.png, 'The app finished a newer sample that the editor did not accept');
        await assert.rejects(propose(17, source.replace('</Window>', '')), /root node was opened but never closed/i);
        await assert.rejects(preview.sampleHostedAnimation({ revision: late.frame.revision, sequence: late.frame.sequence,
            program: review.id, targets: [card.id], elapsed: 0, restart: true }), /current application preview|frame changed/i);
        animations.beforeSourceUpdate();
        // This successful revision was overtaken before the editor accepted it.
        const superseded = await propose(18, animationFixture(0));
        assert.equal(superseded.animationPresentation?.position, undefined);
        assert.equal(displayed!, paused);

        const revised = source.replace('ReviewCorners', 'AdjustedCorners').replace('<Window.Resources>',
            '<Window.Resources><SolidColorBrush x:Key="ShiftNativeIds" Color="#203040"/>');
        const restored = await propose(19, revised);
        present(restored);
        const mapped = clip(restored, 'AdjustedCorners');
        const mappedCard = restored.animations?.targets.find(target => target.label === 'ReviewCard');
        assert.ok(mappedCard);
        assert.notEqual(mapped.id, review.id);
        assert.notEqual(mappedCard.id, card.id);
        assert.equal(animations.state.selection, clip(restored, 'IdleCorners').id);
        assert.deepEqual(animations.state.overrides, { [mapped.id]: [mappedCard.id] });
        assert.deepEqual(animations.state.position, { program: mapped.id, elapsed: 1, playing: false, completed: false });
        assert.equal(restored.image.png, paused.image.png, 'The first delivered replacement already holds the exact native pose');

        animations.beforeSourceUpdate();
        const shorter = await propose(20, revised.replace('Duration="0:0:2"', 'Duration="0:0:0.5"'));
        present(shorter);
        assert.equal(animations.state.position?.elapsed, 0.5);
        assert.equal(animations.state.position?.completed, true);

        await control({ action: 'play', program: clip(shorter, 'AdjustedCorners').id, elapsed: 0.15 });
        animations.beforeSourceUpdate();
        const resumed = await propose(21, revised);
        assert.equal(resumed.animationPresentation?.position?.playing, true);
        assert.equal(resumed.animationPresentation?.position?.elapsed, 0.15);
        present(resumed);
        await until(() => (animations.state.position?.elapsed ?? 0) > 0.15);
        await control({ action: 'pause' });
        assert.equal(animations.state.position?.playing, false);

        animations.beforeSourceUpdate();
        const anotherScope = update(22, revised);
        anotherScope.document.uri = new URL('AnotherView.xaml', anotherScope.document.uri).toString();
        const unrelated = await preview.renderHosted({ ...anotherScope, restoreAnimation: animations.restore });
        assert.equal(unrelated.animationPresentation, undefined);
        const removed = await propose(23, revised.replace('Name="ReviewCard"', 'Name="DifferentCard"'));
        present(removed);
        assert.equal(animations.state.position, undefined);
        assert.deepEqual(animations.state.overrides, {}, 'Removing an explicit target never retargets a surviving clip');
        assert.deepEqual(errors, []);
        console.log('App Preview retained the first native pose, remapped clip and targets, resumed playback, and fenced invalid, superseded and unrelated source revisions');
    } finally { animations.invalidate(); }
}

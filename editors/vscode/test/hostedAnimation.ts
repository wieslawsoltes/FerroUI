import { strict as assert } from 'node:assert';
import type { HostedAnimationRequest } from '../src/hostedAnimationProtocol';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

export const cornerKeys = `<KeyFrame Cue="0%"><Setter Property="CornerRadius" Value="0"/></KeyFrame>
  <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="32"/></KeyFrame>`;

export function animationFixture(radius: number, programs = ''): string {
    return `<Window xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Width="380" Height="220" Background="#EDF2FA">
    ${programs}
    <Border Name="ReviewCard" Width="180" Height="100" CornerRadius="${radius}" Background="#305DB6"/>
</Window>`;
}

export async function verifyHostedAnimation(preview: NativePreview, update: (revision: number, source: string) => HostedSource): Promise<void> {
    const programs = `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">${cornerKeys}</Animation>
    </Style.Animations></Style></Window.Styles>
    <Window.Resources><Animation x:Key="ReviewCorners" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Both">${cornerKeys}</Animation></Window.Resources>`;
    const authored = animationFixture(0, programs);
    const first = await preview.renderHosted(update(11, authored));
    assert.ok(first.animations);
    const { programs: clips, targets } = first.animations;
    const style = clips.find(program => program.selector === 'Border#ReviewCard');
    const resource = clips.find(program => program.name === 'ReviewCorners');
    const card = targets.find(target => target.label === 'ReviewCard');
    assert.ok(style?.customAnimator && resource?.customAnimator && card);
    assert.equal(style.sourceOffset, authored.indexOf('<Animation Duration'));
    assert.deepEqual(style.defaultTargets, [card.id]);
    assert.deepEqual(resource.defaultTargets, []);
    assert.deepEqual(resource.compatibleTargets, [card.id]);

    const frame = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });
    const sample = (frame: HostedFrame, request: Omit<HostedAnimationRequest, 'revision' | 'sequence'>) =>
        preview.sampleHostedAnimation({ revision: frame.revision, sequence: frame.sequence, ...request });
    const halfway = await sample(first, { program: style.id, elapsed: 1, restart: true });
    assert.equal(halfway.elapsed, 1);
    assert.ok(halfway.frame.sequence > first.sequence);
    assert.notEqual(halfway.frame.image.png, first.image.png);
    const corner = style.curveTracks?.find(track => track.property === 'CornerRadius');
    assert.deepEqual(corner?.curve_channels, ['top_left', 'top_right', 'bottom_right', 'bottom_left']);
    const query = { program: style.id, target: card.id, property: 'CornerRadius', window: { start: 0, end: 2 } };
    const topLeft = await preview.hostedCurve(frame(halfway.frame), { ...query, channel: 'top_left' });
    assert.equal(topLeft.find(point => point.time === 1)?.value, 8, 'Read the custom native radius animator from a property component');
    await assert.rejects(preview.hostedCurve(frame(halfway.frame), { ...query, channel: 'height' }), /curve channel/i);
    await assert.rejects(sample(first, { program: style.id, elapsed: 0, restart: true }), /frame changed/i);
    const retired = frame(halfway.frame);
    const rewind = await sample(halfway.frame, { program: style.id, elapsed: .5, restart: true });
    assert.notEqual(rewind.frame.image.png, halfway.frame.image.png);
    await assert.rejects(sample(rewind.frame, { program: resource.id, elapsed: 1, restart: true }), /preview targets/i);
    const alternate = await sample(rewind.frame, { program: resource.id, targets: [card.id], elapsed: 1, restart: true });
    assert.equal(alternate.frame.image.png, halfway.frame.image.png);
    const stopped = await sample(alternate.frame, { elapsed: 0, restart: true });
    assert.equal(stopped.frame.image.png, first.image.png);
    const expected = await preview.renderHosted(update(12, animationFixture(8)));
    assert.equal(halfway.frame.image.png, expected.image.png, 'At one second the app-registered animator reaches Radius 8');
    const builtIn = await preview.renderHosted(update(13, animationFixture(16)));
    assert.notEqual(builtIn.image.png, halfway.frame.image.png, 'The built-in linear interpolation would reach Radius 16');
    await assert.rejects(preview.sampleHostedAnimation({ ...retired, program: style.id, elapsed: 1, restart: true }), /frame changed/i);
    console.log('App-hosted Timeline used the registered animator for a style and resource, matched the expected native frame and rejected retired samples');
}

export async function verifyHostedScalarCurve(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const authored = animationFixture(8, `<!-- Keep this preview note 🚀 -->
    <Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
            <KeyFrame Cue="0%"><Setter Property="Width" Value="140"/></KeyFrame>
            <KeyFrame Cue="50%" KeySpline="0.2,0.4,0.5,1.7"><Setter Property="Width" Value="220"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="Width" Value="180"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`).replaceAll('\n', '\r\n');
    const first = await preview.renderHosted(update(30, authored));
    const program = first.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === 'Width'));
    const track = program?.curveTracks?.find(track => track.property === 'Width');
    assert.ok(program && track && program.authoring, 'Publish a scalar graph and the native codec for its exact authored track');
    const active = await preview.sampleHostedAnimation({ revision: first.revision, sequence: first.sequence,
        program: program.id, elapsed: 1, restart: true });
    const frame = { revision: first.revision, sequence: active.frame.sequence };
    const query = { program: program.id, target: program.defaultTargets[0], property: 'Width', window: { start: 0, end: 2 } };
    const trace = await preview.hostedCurve(frame, query);
    assert.deepEqual(trace.find(point => point.time === 1), { time: 1, value: 220 });
    assert.ok(trace.some(point => point.value > 220), 'Use the loaded easing and include values beyond the authored keys');
    const velocity = await preview.hostedCurve(frame, { ...query, metric: 'velocity' });
    assert.ok(velocity.some(point => point.time < 1 && point.value < 0), 'The native easing settles before the authored midpoint');
    assert.ok(velocity.filter(point => point.time > 1).every(point => Math.abs(point.value + 40) < 1e-8));
    await assert.rejects(preview.hostedCurve({ ...frame, sequence: first.sequence }, query), /frame changed/i);
    await assert.rejects(preview.hostedCurve(frame, { ...query, window: { start: 1, end: 3 } }), /range/i);
    const draft = await preview.planHostedKeyframe(frame, { program: program.id, target: track.target, property: 'Width',
        previous: 1, time: 1.25, value: '208', easing: track.keyframes[1].easing_text });
    assert.ok(draft.patch);
    const { start, end, expected, replacement } = draft.patch;
    assert.equal(authored.slice(start, end), expected);
    const changed = authored.slice(0, start) + replacement + authored.slice(end);
    assert.ok(changed.includes('Keep this preview note 🚀'));
    assert.ok(!changed.replaceAll('\r\n', '').includes('\n'));
    assert.match(changed, /Property="Width" Value="208"/);
    const staticPose = await preview.renderHosted(update(31, animationFixture(8).replace('Width="180"', 'Width="220"')));
    assert.equal(active.frame.image.png, staticPose.image.png, 'The graph and native frame reach the same midpoint width');
    const rebuilt = await preview.renderHosted(update(32, changed));
    assert.equal(rebuilt.animations?.programs[0]?.curveTracks?.[0]?.keyframes[1]?.time, 1.25);
    console.log('App-hosted curves matched the native frame, exposed overshoot, rejected retired identities, and edited one CRLF source patch');
}

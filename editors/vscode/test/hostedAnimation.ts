import { strict as assert } from 'node:assert';
import type { HostedAnimationRequest } from '../src/hostedAnimationProtocol';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const keys = `<KeyFrame Cue="0%"><Setter Property="CornerRadius" Value="0"/></KeyFrame>
  <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="32"/></KeyFrame>`;

function source(radius: number, programs = ''): string {
    return `<Window xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Width="380" Height="220" Background="#EDF2FA">
    ${programs}
    <Border Name="ReviewCard" Width="180" Height="100" CornerRadius="${radius}" Background="#305DB6"/>
</Window>`;
}

export async function verifyHostedAnimation(preview: NativePreview, update: (revision: number, source: string) => HostedSource): Promise<void> {
    const programs = `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">${keys}</Animation>
    </Style.Animations></Style></Window.Styles>
    <Window.Resources><Animation x:Key="ReviewCorners" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Both">${keys}</Animation></Window.Resources>`;
    const authored = source(0, programs);
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
    await assert.rejects(sample(first, { program: style.id, elapsed: 0, restart: true }), /frame changed/i);
    const retired = frame(halfway.frame);
    const rewind = await sample(halfway.frame, { program: style.id, elapsed: .5, restart: true });
    assert.notEqual(rewind.frame.image.png, halfway.frame.image.png);
    await assert.rejects(sample(rewind.frame, { program: resource.id, elapsed: 1, restart: true }), /preview targets/i);
    const alternate = await sample(rewind.frame, { program: resource.id, targets: [card.id], elapsed: 1, restart: true });
    assert.equal(alternate.frame.image.png, halfway.frame.image.png);
    const stopped = await sample(alternate.frame, { elapsed: 0, restart: true });
    assert.equal(stopped.frame.image.png, first.image.png);
    const expected = await preview.renderHosted(update(12, source(8)));
    assert.equal(halfway.frame.image.png, expected.image.png, 'At one second the app-registered animator reaches Radius 8');
    const builtIn = await preview.renderHosted(update(13, source(16)));
    assert.notEqual(builtIn.image.png, halfway.frame.image.png, 'The built-in linear interpolation would reach Radius 16');
    await assert.rejects(preview.sampleHostedAnimation({ ...retired, program: style.id, elapsed: 1, restart: true }), /frame changed/i);
    console.log('App-hosted Timeline used the registered animator for a style and resource, matched the expected native frame and rejected retired samples');
}

import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { VisualStateGroup } from '../src/stateGroupProtocol';
import { applyStateDraft } from './hostedStateAuthoring';

const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

/** Exercise the loaded app's resource scope, name scope and fixture DataContext. */
export async function verifyHostedStateValues(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const artwork = (width: number, opacity: number) => `<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Width="420" Height="220" Background="#EFF4FB">
        <Window.Resources><x:Double x:Key="QueuedWidth">24</x:Double></Window.Resources>
        <Canvas Name="Review">
            <Canvas.Resources>
                <x:Double x:Key="QueuedWidth">120</x:Double>
                <x:Double x:Key="QueuedOpacity">0.25</x:Double>
            </Canvas.Resources>
            <!-- Native state endpoints 🧭 -->
            <Border Name="ReviewCard" Canvas.Left="30" Canvas.Top="40" Width="${width}" Height="100"
                Background="#305DB6" CornerRadius="12" Opacity="${opacity}" />
            <Border Name="Reference" IsVisible="False" Width="240" />
        </Canvas>
    </Window>`;
    const source = (revision: number, text: string): HostedSource => ({
        ...update(revision, text), projectPath: '/HostedStateValues.xaml', fixture: 'Review metrics',
    });
    const base = artwork(80, 1).replaceAll('\n', '\r\n');
    const initial = await preview.renderHosted(source(4100, base));
    assert.equal(initial.fixture, 'Review metrics');
    const owner = initial.animations?.stateAuthoring?.owners.find(owner => owner.label === 'Review');
    assert.ok(owner?.canCreate);
    const model: VisualStateGroup = {
        owner: owner.id, name: 'Delivery',
        states: [
            { name: 'Queued', setters: [
                { target: 'ReviewCard', property: 'Width', value: '{StaticResource QueuedWidth}' },
                { target: 'ReviewCard', property: 'Opacity', value: '{DynamicResource QueuedOpacity}' },
            ] },
            { name: 'Ready', setters: [
                { target: 'ReviewCard', property: 'Width', value: '{Binding Width, ElementName=Reference}' },
                { target: 'ReviewCard', property: 'Opacity', value: '{ReflectionBinding .}' },
            ] },
        ],
        transitions: [{ from: 'Queued', to: 'Ready', duration: 2, easing: { kind: 'Linear' } }],
    };
    const draft = await preview.planHostedStateGroup(identity(initial), { action: 'upsert', model });
    const authored = applyStateDraft(base, draft.patches);
    assert.match(authored, /Native state endpoints 🧭/u);
    assert.doesNotMatch(authored, /(?<!\r)\n/);
    const current = await preview.renderHosted(source(4101, authored));
    const programs = current.animations?.programs ?? [];
    const queued = programs.find(p => p.stateRoute?.endpoint && p.stateRoute.to === 'Queued');
    const ready = programs.find(p => p.stateRoute?.endpoint && p.stateRoute.to === 'Ready');
    const route = programs.find(p => p.stateRoute?.from === 'Queued' && p.stateRoute.to === 'Ready');
    assert.ok(queued && ready && route, 'Publish native state poses and routes with bound values');
    const sample = (frame: HostedFrame, program: number | undefined, elapsed: number) =>
        preview.sampleHostedAnimation({ ...identity(frame), program, elapsed, restart: true });
    const queuedPose = await sample(current, queued.id, 0);
    const readyPose = await sample(queuedPose.frame, ready.id, 0);
    const start = await sample(readyPose.frame, route.id, 0);
    const middle = await sample(start.frame, route.id, 1);
    const end = await sample(middle.frame, route.id, 2);
    const backwards = await sample(end.frame, route.id, 1);
    assert.equal(start.frame.image.png, queuedPose.frame.image.png);
    assert.equal(end.frame.image.png, readyPose.frame.image.png);
    assert.equal(backwards.frame.image.png, middle.frame.image.png);
    assert.notEqual(middle.frame.image.png, start.frame.image.png);
    assert.notEqual(middle.frame.image.png, end.frame.image.png);

    const revised = authored.replace('Key="QueuedWidth">120', 'Key="QueuedWidth">160');
    assert.notEqual(revised, authored);
    const restored = await preview.renderHosted({ ...source(4102, revised), restoreAnimation: {
        frame: identity(backwards.frame),
        presentation: { selection: route.id, overrides: {},
            position: { program: route.id, elapsed: 1, completed: false, playing: false } },
    } });
    assert.equal(restored.animationPresentation?.position?.elapsed, 1);
    assert.notEqual(restored.image.png, middle.frame.image.png);
    await assert.rejects(sample(backwards.frame, route.id, 1), /frame changed/i);
    const cleared = await sample(restored, undefined, 0);
    assert.equal(cleared.frame.image.png, initial.image.png, 'Remove the state layer and restore the app values');

    for (const [revision, frame, width, opacity] of [
        [4103, queuedPose.frame, 120, .25],
        [4104, middle.frame, 180, .525],
        [4105, readyPose.frame, 240, .8],
        [4106, restored, 200, .525],
    ] as const) {
        const literal = await preview.renderHosted(source(revision, artwork(width, opacity)));
        assert.equal(frame.image.png, literal.image.png, `Native state sample matches width ${width} and opacity ${opacity}`);
    }
    console.log('App Preview sampled scoped resources, name scope and fixture data through native state routes and restored the bound pose');
}

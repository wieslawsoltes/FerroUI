import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const property = 'RenderTransform';
const channel = 'transform_operation/2/rotate/angle';
const matrix = 'transform_matrix/m31';
const window = { start: 0, end: 2 };
const base = 'translate(8px, 10px) rotate(20deg) scale(1.05)';
const from = 'translate(32px, 18px) rotate(60deg) scale(1.4, 1.2)';
const bridge = 'translate(50px, 22px) skew(8deg, 4deg)';
const to = 'translate(68px, 28px) rotate(30deg) scale(1.2)';
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

function card(transform: string, animate: boolean) {
    return `<Window xmlns="https://github.com/ferroui" Width="420" Height="300" Background="#EDF2FA">
    ${animate ? `<Window.Styles><Style Selector="Border#Card"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Forward">
            <KeyFrame Cue="50%"><Setter Property="${property}" Value="${from}"/></KeyFrame>
            <KeyFrame Cue="75%"><Setter Property="${property}" Value="${bridge}"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="${property}" Value="${to}"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>` : ''}
    <Border Name="Card" Width="140" Height="72" Background="#3D5094" CornerRadius="10" RenderTransform="${transform}"/>
</Window>`;
}

export async function verifyHostedTransformList(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = card(base, true);
    const first = await preview.renderHosted(update(4140, source));
    const { program, track, query } = trackOf(first);
    assert.equal(track.canSample, true);
    const channels = track.curve_channels;
    assert.ok(channels?.includes(matrix) && channels.includes('transform_operation/2/skew/angle_x'));
    const angles = await preview.hostedCurve(identity(first), query);
    assert.ok(Math.abs(angles.find(point => point.time === .5)!.value - 40) < 1e-10);
    assert.ok(angles.every(point => point.time <= 1 || point.time === 2));
    assert.equal(angles.at(-1)?.break_before, true);
    const displacement = await preview.hostedCurve(identity(first), { ...query, channel: matrix });
    assert.ok(displacement.some(point => point.time === 1.25));
    const speed = await preview.hostedCurve(identity(first), { ...query, metric: 'velocity' });
    assert.ok(speed.some(point => Math.abs(point.time - .5) < .05 && Math.abs(point.value - 40) < 1e-8));

    const pose = await preview.sampleHostedAnimation({ ...identity(first), program: program.id, elapsed: .5, restart: true });
    assert.deepEqual(await preview.hostedCurve(identity(pose.frame), query), angles);
    const result = await preview.planHostedKeyframe(identity(pose.frame), { program: program.id,
        target: track.target, property, time: .5, previewTarget: query.target, sample: true });
    assert.ok(result.patch && result.sampledKey);
    assert.equal(result.sampledKey.value_kind, 'TransformList');
    const transformed = apply(source, result.patch);
    assert.ok(transformed.includes(`RenderTransform="${base}"`));
    assert.equal(transformed.match(/<KeyFrame /g)?.length, 4);
    const stopped = await preview.sampleHostedAnimation({ ...identity(pose.frame), elapsed: 0, restart: true });
    assert.equal(stopped.frame.image.png, first.image.png, 'Stopping restores the original rendered transform');
    const reference = await preview.renderHosted(update(4141, card(result.sampledKey.value_text, false)));
    assert.ok(pose.frame.image.png === reference.image.png, 'The recorded full transform list matches the static native frame');
    await assert.rejects(preview.hostedCurve(identity(first), query), /frame changed/i);
    await verifyMapped(preview, update);
    console.log('App Preview plotted a whole transform list and its composed matrix, matched native pixels, and preserved mapped Rust while editing a key');
}

async function verifyMapped(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = `fn untouched() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#Card").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.5, |key| key.set(ferroui_base::Visual::render_transform_property(), /* turn 🧭 */ r#"${from}"#))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::render_transform_property(), "${to}"))
        )],
        Border { name: "Card", width: 140.0, height: 72.0, background: "#3D5094", render_transform: "${base}" }
    }
} }
`.replaceAll('\n', '\r\n');
    const draft = update(4142, source);
    const frame = await preview.renderHosted({ ...draft, projectPath: '/TransformList.rs', document: { ...draft.document,
        uri: new URL('TransformList.rs', draft.document.uri).toString(), language: 'rust', caret: { line: 1, character: 16 } } });
    const { program, track, query } = trackOf(frame);
    const points = await preview.hostedCurve(identity(frame), query);
    assert.ok(Math.abs(points.find(point => point.time === .5)!.value - 40) < 1e-10);
    const value = from.replace('60deg', '72deg');
    const draftKey = { program: program.id, target: track.target, property, previous: 1, time: 1, value, easing: 'LinearEasing' } as const;
    await assert.rejects(preview.planHostedKeyframe(identity(frame), { ...draftKey, value: 'rotate(bad)' }));
    const { patch } = await preview.planHostedKeyframe(identity(frame), draftKey);
    assert.ok(patch);
    assert.equal(apply(source, patch), source.replace(from, value));
}

function trackOf(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    return { program, track, query: { program: program.id, target: program.defaultTargets[0], property, channel, window } } as const;
}

function apply(source: string, patch: { start: number; end: number; expected: string; replacement: string }) {
    assert.equal(source.slice(patch.start, patch.end), patch.expected);
    return source.slice(0, patch.start) + patch.replacement + source.slice(patch.end);
}

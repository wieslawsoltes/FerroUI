import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const property = 'BoxShadow';
const channel = 'box_shadow/2/spread';
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });
const base = '0 2 4 -2 #40263452, 4 8 12 2 #80325AC0';
const middle = '8 10 20 4 #80263452, inset 14 18 28 10 #C0325AC0';
const end = '2 4 8 0 #40263452';

function card(shadows: string, animate: boolean): string {
    return `<Window xmlns="https://github.com/ferroui" Width="420" Height="260" Background="#EDF2FA">
    ${animate ? `<Window.Styles><Style Selector="Border#Card"><Style.Animations>
        <Animation Duration="0:0:2">
            <KeyFrame Cue="50%"><Setter Property="BoxShadow" Value="${middle}"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="${end}"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>` : ''}
    <Border Name="Card" Width="180" Height="96" Background="White" CornerRadius="14" BoxShadow="${shadows}"/>
</Window>`;
}

export async function verifyHostedBoxShadows(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const xaml = card(base, true);
    const first = await preview.renderHosted(update(4130, xaml));
    const program = first.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    assert.equal(track.canSample, true);
    assert.equal(track.curve_channels?.length, 16);
    assert.ok(track.curve_channels.includes('box_shadow/1/alpha'));
    assert.ok(track.curve_channels.includes(channel));
    const query = { program: program.id, property, channel, target: program.defaultTargets[0], window: { start: 0, end: 2 } } as const;
    const points = await preview.hostedCurve(identity(first), query);
    assert.deepEqual(points.find(point => point.time === .5), { time: .5, value: 6 });
    assert.deepEqual(points.find(point => point.time === 1), { time: 1, value: 10, break_before: true });
    assert.ok(points.every(point => point.time < 2));
    const velocity = await preview.hostedCurve(identity(first), { ...query, metric: 'velocity' });
    assert.ok(velocity.some(point => Math.abs(point.time - .5) < .05 && Math.abs(point.value - 8) < 1e-8));

    const pose = await preview.sampleHostedAnimation({ ...identity(first), program: program.id, elapsed: .5, restart: true });
    assert.deepEqual(await preview.hostedCurve(identity(pose.frame), query), points);
    const plan = await preview.planHostedKeyframe(identity(pose.frame), { program: program.id, target: track.target,
        property, time: .5, previewTarget: query.target, sample: true });
    assert.ok(plan.patch && plan.sampledKey);
    const patch = plan.patch;
    assert.equal(xaml.slice(patch.start, patch.end), patch.expected);
    const edited = xaml.slice(0, patch.start) + patch.replacement + xaml.slice(patch.end);
    assert.ok(edited.includes(`BoxShadow="${base}"`));
    assert.equal(edited.match(/<KeyFrame /g)?.length, 3);
    const reference = await preview.renderHosted(update(4131, card(plan.sampledKey.value_text, false)));
    assert.ok(pose.frame.image.png === reference.image.png, 'The sampled shadow list matches the static native frame');
    await assert.rejects(preview.hostedCurve(identity(first), query), /frame changed/i);
    await verifyMappedShadows(preview, update);
    console.log('App Preview sampled layered box shadows, matched the native frame, and preserved Rust source while editing keys');
}

async function verifyMappedShadows(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = `fn untouched() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#Card").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.5, |key| key.set(ferroui_controls::Border::box_shadow_property(),
                    ferroui_base::media::BoxShadows::parse(/* lift 🧭 */ r#"${middle}"#).expect("valid shadows")))
                .key_frame(1.0, |key| key.set(ferroui_controls::Border::box_shadow_property(),
                    ferroui_base::media::BoxShadows::parse("${end}").unwrap()))
        )],
        Border { name: "Card", width: 180.0, height: 96.0, background: "#FFFFFF",
            box_shadow: ferroui_base::media::BoxShadows::parse("${base}").unwrap() }
    }
} }
`.replaceAll('\n', '\r\n');
    const draft = update(4132, source);
    const mapped = await preview.renderHosted({ ...draft, projectPath: '/CardShadows.rs', document: { ...draft.document,
        uri: new URL('CardShadows.rs', draft.document.uri).toString(), language: 'rust', caret: { line: 1, character: 16 } } });
    const program = mapped.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    const points = await preview.hostedCurve(identity(mapped), { program: program.id, target: program.defaultTargets[0],
        property, channel, window: { start: 0, end: 2 } });
    assert.deepEqual(points.find(point => point.time === .5), { time: .5, value: 6 });
    const value = middle.replace('28 10', '28 12');
    const { patch } = await preview.planHostedKeyframe(identity(mapped), { program: program.id, target: track.target,
        property, previous: 1, time: 1, value, easing: 'LinearEasing' });
    assert.ok(patch);
    assert.equal(source.slice(patch.start, patch.end), patch.expected);
    assert.equal(source.slice(0, patch.start) + patch.replacement + source.slice(patch.end), source.replace(middle, value));
}

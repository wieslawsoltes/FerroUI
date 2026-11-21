import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture } from './hostedAnimation';

const channels = ['red', 'green', 'blue', 'alpha'] as const;
const expected = [188, 92, 44, 192];
const window = { start: 0, end: 2 };
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

export async function verifyHostedColorCurves(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const style = (last: string) => `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
            <KeyFrame Cue="0%"><Setter Property="Background" Value="Black"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="Background" Value="${last}"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`;
    const started = await preview.renderHosted(update(210, animationFixture(0, style('#80FF8040'))));
    const xaml = await checkCurves(preview, started);
    const midpoint = await preview.sampleHostedAnimation({ revision: started.revision, sequence: started.sequence,
        program: xaml.program.id, elapsed: 1, restart: true });
    const reference = await preview.renderHosted(update(211, animationFixture(0).replace('#305DB6', '#C0BC5C2C')));
    assert.equal(midpoint.frame.image.png, reference.image.png, 'The four values reproduce the sampled native color');
    await assert.rejects(preview.hostedCurve(identity(started), { ...xaml.query, channel: 'red' }), /frame changed/i);

    const rust = `fn separate() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_controls::Border::background_property(), "#000000"))
                .key_frame(1.0, |key| key.set(ferroui_controls::Border::background_property(),
                    ferroui_base::media::Color::from_argb(128, /* Red 🧭 */ 255u8, 128, 64)))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "ReviewCard", width: 180.0, height: 100.0, background: "#305DB6" }
    }
}}
`.replaceAll('\n', '\r\n');
    const rsx = (revision: number, source: string): HostedSource => {
        const frame = update(revision, source);
        return { ...frame, projectPath: '/ColorReview.rs', document: { ...frame.document, language: 'rust',
            uri: new URL('ColorReview.rs', frame.document.uri).toString(), caret: { line: 2, character: 8 } } };
    };
    const mapped = await preview.renderHosted(rsx(212, rust));
    const { program, query } = await checkCurves(preview, mapped);
    const authored = program.authoring?.tracks.find(track => track.property === 'Background');
    assert.ok(authored);
    const draft = await preview.planHostedKeyframe(identity(mapped), { program: program.id, target: authored.target,
        property: 'Background', previous: 2, time: 2, value: '#60C08040', easing: 'LinearEasing' });
    assert.equal(draft.valueKind, 'Color');
    assert.ok(draft.patch);
    const { start, end, expected: original, replacement } = draft.patch;
    assert.equal(rust.slice(start, end), original);
    const rebuilt = rust.slice(0, start) + replacement + rust.slice(end);
    assert.equal(rebuilt, rust.replace('128, /* Red 🧭 */ 255u8', '96, /* Red 🧭 */ 192u8'));
    const edited = await preview.renderHosted(rsx(213, rebuilt));
    const next = edited.animations!.programs.find(item => item.curveTracks?.some(track => track.property === 'Background'))!;
    const red = await preview.hostedCurve(identity(edited), { ...query, program: next.id, target: next.defaultTargets[0], channel: 'red' });
    assert.equal(red.at(-1)?.value, 192 / 255);

    const gradient = `<Window.Resources><LinearGradientBrush x:Key="ReviewGradient" StartPoint="0%,0%" EndPoint="100%,0%">
        <GradientStop Color="Black" Offset="0"/><GradientStop Color="White" Offset="1"/>
    </LinearGradientBrush></Window.Resources>`;
    const mixed = await preview.renderHosted(update(214, animationFixture(0, gradient + style('{StaticResource ReviewGradient}'))));
    assert.ok(mixed.animations?.programs.length, 'A native mixed brush animation still loads');
    assert.ok(mixed.animations.programs.every(item => !item.curveTracks?.some(track => track.property === 'Background')));
    console.log('App Preview sampled native RGBA for XAML and mapped Rust, matched a native frame, and preserved Rust literals in a checked edit');
}

async function checkCurves(preview: NativePreview, frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === 'Background'));
    assert.ok(program, 'Publish curve tracks for the live solid brush');
    const track = program.curveTracks!.find(candidate => candidate.property === 'Background');
    assert.deepEqual(track?.curve_channels, channels);
    const query = { program: program.id, target: program.defaultTargets[0], property: 'Background', window };
    const current = identity(frame);
    for (const [index, channel] of channels.entries()) {
        const points = await preview.hostedCurve(current, { ...query, channel });
        assert.equal(points.find(point => point.time === 1)?.value, expected[index] / 255, channel);
    }
    const velocity = await preview.hostedCurve(current, { ...query, channel: 'alpha', metric: 'velocity' });
    assert.ok(velocity.some(point => point.value < 0));
    assert.ok(velocity.every(point => point.value <= 0));
    await assert.rejects(preview.hostedCurve(current, { ...query, channel: 'width' }), /curve channel/i);
    return { program, query };
}

import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const property = 'RenderTransformOrigin';
const window = { start: 0, end: 4 };
const originValues = ['0%,10%', '100%,20%', '24,40', '25%,70%', '75%,90%'];
export const relativeOriginChannels = ['percent_x', 'percent_y', 'absolute_x', 'absolute_y'];
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

export function relativeOriginMarkup(origin: string, clip: boolean) {
    const keys = originValues.map((value, index) =>
        `<KeyFrame Cue="${index * 25}%"><Setter Property="${property}" Value="${value}"/></KeyFrame>`).join('\n');
    return `<Window xmlns="https://github.com/ferroui" Width="380" Height="220" Background="#EDF2FA">
    ${clip ? `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:4" FillMode="Both">${keys}</Animation>
    </Style.Animations></Style></Window.Styles>` : ''}
    <Border Name="ReviewCard" Width="135" Height="68" Background="#305DB6" RenderTransformOrigin="${origin}">
        <Border.RenderTransform><RotateTransform Angle="38"/></Border.RenderTransform>
    </Border>
</Window>`;
}

export async function verifyHostedRelativeCurves(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const first = await preview.renderHosted(update(220, relativeOriginMarkup('0%,10%', true)));
    const program = first.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.curveTracks?.find(track => track.property === property);
    assert.ok(program && track);
    assert.deepEqual(track.curve_channels, relativeOriginChannels);
    const query = { program: program.id, target: program.defaultTargets[0], property, window };
    const percent = await preview.hostedCurve(identity(first), { ...query, channel: 'percent_x' });
    assert.deepEqual(percent.find(point => point.time === 0.5), { time: 0.5, value: 50 });
    assert.deepEqual(percent.find(point => point.time === 2.5), { time: 2.5, value: 25, break_before: true });
    assert.ok(percent.every(point => point.time < 1.5 || point.time >= 2.5));
    const pixels = await preview.hostedCurve(identity(first), { ...query, channel: 'absolute_y' });
    assert.ok(pixels.length);
    assert.ok(pixels.every(point => point.time >= 1.5 && point.time < 2.5 && point.value === 40));
    const velocity = await preview.hostedCurve(identity(first), { ...query, channel: 'percent_x', metric: 'velocity' });
    assert.ok(velocity.some(point => point.time > 2.5 && point.break_before));
    assert.ok(velocity.every(point => point.time < 1.5 || point.time > 2.5));

    const previewed = await preview.sampleHostedAnimation({ ...identity(first), program: program.id, elapsed: 1.75, restart: true });
    const staticPose = await preview.renderHosted(update(221, relativeOriginMarkup('24,40', false)));
    assert.ok(first.image.png !== staticPose.image.png, 'The two authored origin units move the rotated card');
    assert.ok(previewed.frame.image.png === staticPose.image.png, 'The sampled absolute origin matches the native rotation');
    await assert.rejects(preview.hostedCurve(identity(first), { ...query, channel: 'percent_x' }), /frame changed/i);

    const rust = `fn separate() { rsx! { TextBlock { text: "Other 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_base::Visual::render_transform_origin_property(),
                    ferroui_base::RelativePoint::new(0.0, 0.25, ferroui_base::RelativeUnit::Relative)))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::render_transform_origin_property(),
                    ferroui_base::RelativePoint::new(/* left 🧭 */ 24.0f64, 40.0, ferroui_base::RelativeUnit::Absolute)))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "ReviewCard", width: 135.0, height: 68.0, background: "#305DB6" }
    }
}}
`.replaceAll('\n', '\r\n');
    const rsx = (revision: number, source: string): HostedSource => {
        const frame = update(revision, source);
        return { ...frame, projectPath: '/OriginReview.rs', document: { ...frame.document, language: 'rust',
            uri: new URL('OriginReview.rs', frame.document.uri).toString(), caret: { line: 2, character: 8 } } };
    };
    const mapped = await preview.renderHosted(rsx(222, rust));
    const mappedProgram = mapped.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const authored = mappedProgram?.authoring?.tracks.find(track => track.property === property);
    assert.ok(mappedProgram && authored);
    const draft = await preview.planHostedKeyframe(identity(mapped), { program: mappedProgram.id, target: authored.target,
        property, previous: 2, time: 2, value: '30%,50%', easing: 'LinearEasing' });
    assert.equal(draft.valueKind, 'RelativePoint');
    assert.ok(draft.patch);
    const { start, end, expected, replacement } = draft.patch;
    assert.equal(rust.slice(start, end), expected);
    const rebuilt = rust.slice(0, start) + replacement + rust.slice(end);
    assert.equal(rebuilt, rust.replace('24.0f64, 40.0, ferroui_base::RelativeUnit::Absolute',
        '0.3f64, 0.5, ferroui_base::RelativeUnit::Relative'));
    const updated = await preview.renderHosted(rsx(223, rebuilt));
    const next = updated.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    assert.ok(next);
    assert.deepEqual(next.curveTracks?.find(track => track.property === property)?.curve_channels, ['percent_x', 'percent_y']);
    const samples = await preview.hostedCurve(identity(updated), { program: next.id, target: next.defaultTargets[0], property,
        window: { start: 0, end: 2 }, channel: 'percent_x' });
    assert.deepEqual(samples.at(-1), { time: 2, value: 30 });
    console.log('App Preview separated percentage and pixel curves, matched the rotated native frame, and edited mapped Rust units');
}

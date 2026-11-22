import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture } from './hostedAnimation';

const property = 'Effect';
const window = { start: 0, end: 2 };
const channels = ['blur_radius', 'offset_x', 'offset_y', 'red', 'green', 'blue', 'alpha'] as const;
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

export async function verifyHostedEffectCurves(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const authored = animationFixture(12, `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2">
            <KeyFrame Cue="25%"><Setter Property="Effect" Value="drop-shadow(0 2 4 Black)"/></KeyFrame>
            <KeyFrame Cue="75%"><Setter Property="Effect" Value="drop-shadow(12 -4 16 #80FF8040)"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`);
    const started = await preview.renderHosted(update(4000, authored));
    const { program, track, query } = effectTrack(started);
    assert.deepEqual(program.curveTracks?.[0].curve_channels, channels);
    const expected = [10, 6, -1, 188 / 255, 92 / 255, 44 / 255, 192 / 255];
    for (const [index, channel] of channels.entries()) {
        const points = await preview.hostedCurve(identity(started), { ...query, channel });
        assert.ok(points[0].time > 0 && points.at(-1)!.time < 2, 'Unset effect endpoints are gaps');
        assert.ok(Math.abs(points.find(point => point.time === 1)!.value - expected[index]) < 1e-8, channel);
    }
    const midpoint = await preview.sampleHostedAnimation({ ...identity(started), program: program.id, elapsed: 1, restart: true });
    const record = await preview.planHostedKeyframe(identity(midpoint.frame), { program: program.id, property, target: track.target,
        previewTarget: program.defaultTargets[0], time: 1, sample: true });
    assert.equal(record.valueKind, 'Effect');
    assert.equal(record.sampledKey?.value_text, 'drop-shadow(6 -1 10 #C0BC5C2C)');
    assert.ok(record.patch);
    assert.equal(authored.slice(record.patch.start, record.patch.end), record.patch.expected);
    assert.equal(record.patch.replacement.match(/<KeyFrame\b/g)?.length, 3, 'Preserve implicit endpoints');
    const reference = animationFixture(12).replace('Name="ReviewCard"', `Name="ReviewCard" Effect="${record.sampledKey.value_text}"`);
    const staticFrame = await preview.renderHosted(update(4001, reference));
    assert.notEqual(staticFrame.image.png, started.image.png);
    assert.equal(midpoint.frame.image.png, staticFrame.image.png, 'The graph and sampled key reproduce the rendered shadow');
    await assert.rejects(preview.hostedCurve(identity(midpoint.frame), { ...query, channel: 'offset_x' }), /frame changed/i);

    const rust = `fn separate() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    ferroui_base::media::Effect::parse("blur(2)").unwrap()))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    ferroui_base::media::Effect::parse(/* radius 🧭 */ r#"blur(14)"#).expect("valid blur")))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "ReviewCard", width: 180.0, height: 100.0, background: "#305DB6" }
    }
} }
`.replaceAll('\n', '\r\n');
    const projected = update(4002, rust);
    const request: HostedSource = { ...projected, projectPath: '/EffectReview.rs', document: { ...projected.document,
        uri: new URL('EffectReview.rs', projected.document.uri).toString(), language: 'rust', caret: { line: 2, character: 8 } } };
    const mapped = await preview.renderHosted(request);
    const effect = effectTrack(mapped);
    assert.deepEqual(effect.program.curveTracks?.[0].curve_channels, ['blur_radius']);
    const points = await preview.hostedCurve(identity(mapped), { ...effect.query, channel: 'blur_radius' });
    assert.equal(points.find(point => point.time === 1)?.value, 8);
    const edit = { program: effect.program.id, target: effect.track.target, property, previous: 2, time: 2, easing: 'LinearEasing' };
    await assert.rejects(preview.planHostedKeyframe(identity(mapped), { ...edit, value: 'blur(NaN)' }));
    const draft = await preview.planHostedKeyframe(identity(mapped), { ...edit, value: 'blur(18)' });
    assert.ok(draft.patch);
    const { start, end, expected: original, replacement } = draft.patch;
    assert.equal(rust.slice(start, end), original);
    const source = rust.slice(0, start) + replacement + rust.slice(end);
    assert.equal(source, rust.replace('blur(14)', 'blur(18)'));
    const refreshed = await preview.renderHosted({ ...request, revision: 4003, document: { ...request.document, source } });
    const updated = effectTrack(refreshed);
    const after = await preview.hostedCurve(identity(refreshed), { ...updated.query, channel: 'blur_radius' });
    assert.equal(after.find(point => point.time === 1)?.value, 10);
    console.log('App Preview verified native effect component curves, recorded an implicit-endpoint XAML track, and edited a mapped Rust literal');
}

function effectTrack(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track, 'The live Effect track supports sampled curves and authoring');
    const query = { program: program.id, target: program.defaultTargets[0], property, window };
    return { program, track, query };
}

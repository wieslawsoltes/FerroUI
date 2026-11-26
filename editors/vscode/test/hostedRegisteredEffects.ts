import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture } from './hostedAnimation';

const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });
const strength = 'effect_parameter/review-spotlight/strength/value';
const color = 'effect_parameter/review-spotlight/color/green';
const center = 'effect_parameter/review-spotlight/center-x/value';
const literal = (amount: number) => `review-spotlight(strength=${amount}, color=#FF82CCF0)`;

export async function verifyHostedRegisteredEffects(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = animationFixture(12, `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
            <KeyFrame Cue="0%"><Setter Property="Effect" Value="${literal(0)}" /></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="Effect" Value="${literal(1)}" /></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`);
    const frame = await preview.renderHosted(update(4020, source));
    const { program, track, query } = effectTrack(frame);
    assert.ok(track.curve_channels?.includes(strength));
    const values = await preview.hostedCurve(identity(frame), { ...query, channel: strength });
    assert.equal(values.find(point => point.time === 1)?.value, .25, 'Use the app shader’s quadratic interpolation');
    const green = await preview.hostedCurve(identity(frame), { ...query, channel: color });
    assert.equal(green.find(point => point.time === 1)?.value, .8);
    assert.equal(track.keyframes[1].curve_values?.find(value => value.channel === center)?.value, .55);
    const velocity = await preview.hostedCurve(identity(frame), { ...query, channel: strength, metric: 'velocity' });
    assert.ok(velocity.some(point => Math.abs(point.time - 1) < .02 && Math.abs(point.value - .5) < .02));
    await assert.rejects(preview.hostedCurve(identity(frame), {
        ...query, channel: 'effect_parameter/spotlight/strength/value',
    }), /curve channel/i);
    const key = { program: program.id, target: track.target, property: track.property, time: 1 };
    const componentDraft = await preview.planHostedKeyframe(identity(frame), {
        ...key, previous: 2, time: 2, component: { channel: center, value: .65 }, easing: 'LinearEasing',
    });
    assert.ok(componentDraft.patch);
    const xaml = componentDraft.patch;
    assert.equal(source.slice(xaml.start, xaml.end), xaml.expected);
    assert.equal(source.slice(0, xaml.start) + xaml.replacement + source.slice(xaml.end),
        source.replace(literal(1), literal(1).replace(')', ', center-x=0.65)')));
    await assert.rejects(preview.planHostedKeyframe(identity(frame), { ...key,
        value: 'review-spotlight(strength=NaN)', easing: 'LinearEasing' }), /finite/i);
    const pose = await preview.sampleHostedAnimation({ ...identity(frame), program: program.id, elapsed: 1, restart: true });
    const recorded = await preview.planHostedKeyframe(identity(pose.frame), {
        ...key, previewTarget: query.target, sample: true,
    });
    assert.match(recorded.sampledKey?.value_text ?? '', /^review-spotlight\(.*strength=0\.25[,)]/);
    assert.ok(recorded.patch?.replacement.includes('strength=0.25'));

    const staticSource = (amount: number) => animationFixture(12).replace('Background="#305DB6"/>',
        `Background="#305DB6" Effect="${literal(amount)}"/>`);
    const reference = await preview.renderHosted(update(4021, staticSource(.25)));
    assert.equal(pose.frame.image.png, reference.image.png, 'The resolved shader parameter reproduces the native frame');
    const base = await preview.renderHosted(update(4022, staticSource(0)));
    assert.notEqual(base.image.png, reference.image.png, 'The shader changes the rendered pixels');
    await assert.rejects(preview.hostedCurve(identity(pose.frame), { ...query, channel: strength }), /frame changed/i);

    const rustLiteral = ' Review_Spotlight ( Color = #FF82CCF0 ; Strength = 1 ) ';
    const rust = `fn other() { rsx! { TextBlock { text: "Keep 🧪" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    ferroui_base::media::Effect::parse("${literal(0)}").unwrap()))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    ferroui_base::media::Effect::parse(/* project shader 🧪 */ r#"${rustLiteral}"#).expect("registered shader")))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "ReviewCard", width: 180.0, height: 100.0, background: "#305DB6" }
    }
} }
`.replaceAll('\n', '\r\n');
    const input = update(4023, rust);
    const request: HostedSource = { ...input, projectPath: '/ShaderView.rs', document: {
        ...input.document, language: 'rust', uri: new URL('ShaderView.rs', input.document.uri).toString(),
        caret: { line: 2, character: 5 },
    } };
    const mapped = await preview.renderHosted(request);
    const mappedTrack = effectTrack(mapped);
    assert.ok(mappedTrack.track.curve_channels?.includes(strength));
    assert.equal(mappedTrack.track.keyframes[1].curve_values?.find(value => value.channel === strength)?.value, 1);
    const changed = await preview.planHostedKeyframe(identity(mapped), {
        program: mappedTrack.program.id, target: mappedTrack.track.target, property: 'Effect',
        previous: 2, time: 2, component: { channel: strength, value: .5 }, easing: 'LinearEasing',
    });
    assert.ok(changed.patch);
    const { start, end, expected, replacement } = changed.patch;
    assert.equal(rust.slice(start, end), expected);
    const edited = rust.slice(0, start) + replacement + rust.slice(end);
    assert.equal(edited, rust.replace(rustLiteral, rustLiteral.replace('Strength = 1', 'Strength = 0.5')),
        'Keep parameter aliases, whitespace, comments, raw-string quotes and CRLF');
    const next = await preview.renderHosted({ ...request, revision: 4024, document: { ...request.document, source: edited } });
    const nextTrack = effectTrack(next);
    const updated = await preview.hostedCurve(identity(next), { ...nextTrack.query, channel: strength });
    assert.equal(updated.find(point => point.time === 1)?.value, .125);
    await assert.rejects(preview.planHostedKeyframe(identity(mapped), {
        ...key, previous: 2, time: 2, component: { channel: strength, value: .7 }, easing: 'LinearEasing',
    }), /frame changed/i);
    console.log('App Preview sampled, recorded and edited a project-registered shader in XAML and mapped Rust');
}

function effectTrack(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === 'Effect'));
    const track = program?.authoring?.tracks.find(candidate => candidate.property === 'Effect');
    assert.ok(program && track);
    return { program, track, query: { program: program.id, target: program.defaultTargets[0], property: 'Effect',
        window: { start: 0, end: 2 } } };
}

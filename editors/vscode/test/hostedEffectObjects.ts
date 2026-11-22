import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture } from './hostedAnimation';

const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });
const property = 'Effect';
const window = { start: 0, end: 2 };
const directional = (direction: number, depth: number, radius: number, opacity: number) =>
    `<DropShadowDirectionEffect Direction="${direction}" ShadowDepth="${depth}" BlurRadius="${radius}" Color="#80234567" Opacity="${opacity}" />`;

export async function verifyHostedEffectObjects(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = animationFixture(12, `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
            <KeyFrame Cue="0%"><Setter Property="Effect"><Setter.Value>${directional(30, 4, 2, .25)}</Setter.Value></Setter></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="Effect">${directional(150, 20, 10, .75)}</Setter></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>`);
    const first = await preview.renderHosted(update(4010, source));
    const { program, query, track } = effectTrack(first);
    assert.deepEqual(program.curveTracks?.[0].curve_channels, ['blur_radius', 'direction', 'shadow_depth', 'shadow_opacity', 'red', 'green', 'blue', 'alpha']);
    for (const [channel, expected] of [['direction', 90], ['shadow_depth', 12], ['shadow_opacity', .5], ['alpha', 128 / 255]] as const) {
        const points = await preview.hostedCurve(identity(first), { ...query, channel });
        assert.equal(points.find(point => point.time === 1)?.value, expected, channel);
    }
    const velocity = await preview.hostedCurve(identity(first), { ...query, channel: 'direction', metric: 'velocity' });
    const aroundMiddle = velocity.filter(point => Math.abs(point.time - 1) < .05);
    assert.ok(aroundMiddle.length > 0 && aroundMiddle.every(point => Math.abs(point.value - 60) < 1e-8));
    const pose = await preview.sampleHostedAnimation({ ...identity(first), program: program.id, elapsed: 1, restart: true });
    const record = await preview.planHostedKeyframe(identity(pose.frame), { program: program.id, target: track.target, property,
        time: 1, previewTarget: program.defaultTargets[0], sample: true });
    assert.equal(record.sampledKey?.value_text, `<Setter.Value>${directional(90, 12, 6, .5)}</Setter.Value>`);
    assert.ok(record.patch);
    assert.equal(record.patch.replacement.match(/<Setter.Value>/g)?.length, 2);
    const reference = animationFixture(12).replace('Background="#305DB6"/>',
        `Background="#305DB6"><Border.Effect>${directional(90, 12, 6, .5)}</Border.Effect></Border>`);
    const staticFrame = await preview.renderHosted(update(4011, reference));
    assert.equal(pose.frame.image.png, staticFrame.image.png, 'The computed direction, depth and opacity reproduce the native pose');
    await assert.rejects(preview.hostedCurve(identity(first), { ...query, channel: 'direction' }), /frame changed/i);

    const rust = `fn separate() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    std::rc::Rc::new(ferroui_base::media::ImmutableDropShadowEffect::new(
                        2.0, 3.0, 4.0, ferroui_base::media::Color::from_argb(128, 35, 69, 103), 0.25
                    )) as std::rc::Rc<dyn ferroui_base::media::IEffect>))
                .key_frame(1.0, |key| key.set(ferroui_base::Visual::effect_property(),
                    std::rc::Rc::new(ferroui_base::media::ImmutableDropShadowEffect::new(
                        14.0, 7.0, 12.0, ferroui_base::media::Color::from_argb(128, 35, 69, 103), /* opacity 🧭 */ 0.75f64
                    )) as std::rc::Rc<dyn ferroui_base::media::IEffect>))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "ReviewCard", width: 180.0, height: 100.0, background: "#305DB6" }
    }
} }
`.replaceAll('\n', '\r\n');
    const buffer = update(4012, rust);
    const request = { ...buffer, projectPath: '/ShadowView.rs', document: { ...buffer.document, language: 'rust' as const,
        uri: new URL('ShadowView.rs', buffer.document.uri).toString(), caret: { line: 2, character: 8 } } };
    const mapped = await preview.renderHosted(request);
    const effect = effectTrack(mapped);
    assert.ok(effect.program.curveTracks?.[0].curve_channels?.includes('shadow_opacity'));
    const value = '<DropShadowEffect OffsetX="14" OffsetY="7" BlurRadius="12" Color="#80234567" Opacity="0.5" />';
    const draft = await preview.planHostedKeyframe(identity(mapped), { program: effect.program.id, target: effect.track.target,
        property, previous: 2, time: 2, value, easing: 'LinearEasing' });
    assert.ok(draft.patch);
    const { start, end, expected, replacement } = draft.patch;
    assert.equal(rust.slice(start, end), expected);
    const edited = rust.slice(0, start) + replacement + rust.slice(end);
    assert.equal(edited, rust.replace('0.75f64', '0.5f64'));
    const next = await preview.renderHosted({ ...request, revision: 4013, document: { ...request.document, source: edited } });
    const newEffect = effectTrack(next);
    const opacity = await preview.hostedCurve(identity(next), { ...newEffect.query, channel: 'shadow_opacity' });
    assert.equal(opacity.find(point => point.time === 1)?.value, .375);
    console.log('App Preview matched native directional-shadow pixels, retained object keys and independently edited a mapped Rust shadow opacity');
}

function effectTrack(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    return { program, track, query: { program: program.id, target: program.defaultTargets[0], property, window } };
}

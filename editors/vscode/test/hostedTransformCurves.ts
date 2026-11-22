import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const angle = 'RotateTransform.Angle';
const position = 'TranslateTransform.X';
const window = { start: 0, end: 2 };
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

export function transformCard(rotation: number, x: number, animate: boolean): string {
    return `<Window xmlns="https://github.com/ferroui" Width="380" Height="220" Background="#EDF2FA">
    ${animate ? `<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
        <Animation Duration="0:0:2">
            <KeyFrame Cue="50%"><Setter Property="${angle}" Value="80"/><Setter Property="${position}" Value="44"/></KeyFrame>
            <KeyFrame Cue="100%"><Setter Property="${angle}" Value="100"/></KeyFrame>
        </Animation>
    </Style.Animations></Style></Window.Styles>` : ''}
    <Border Name="ReviewCard" Width="128" Height="72" Background="#305DB6" RenderTransformOrigin="50%,50%">
        <Border.RenderTransform><TransformGroup>
            <RotateTransform Angle="${rotation}"/><TranslateTransform X="${x}"/>
        </TransformGroup></Border.RenderTransform>
    </Border>
</Window>`;
}

export async function verifyHostedTransformCurves(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const xaml = transformCard(20, 4, true);
    const first = await preview.renderHosted(update(4120, xaml));
    const { program, track, query } = transformTrack(first, angle);
    const angles = await preview.hostedCurve(identity(first), query);
    assert.deepEqual(angles.find(point => point.time === .5), { time: .5, value: 50 });
    const offsets = await preview.hostedCurve(identity(first), { ...query, property: position });
    assert.deepEqual(offsets.find(point => point.time === .5), { time: .5, value: 24 });
    const velocity = await preview.hostedCurve(identity(first), { ...query, metric: 'velocity' });
    assert.ok(velocity.some(point => Math.abs(point.time - .5) < .05 && Math.abs(point.value - 60) < 1e-8));

    const pose = await preview.sampleHostedAnimation({ ...identity(first), program: program.id, elapsed: .5, restart: true });
    const recorded = await preview.planHostedKeyframe(identity(pose.frame), { program: program.id, target: track.target, property: angle,
        time: .5, previewTarget: query.target, sample: true });
    assert.equal(recorded.sampledKey?.value_text, '50');
    assert.ok(recorded.patch);
    const edited = applyPatch(xaml, recorded.patch);
    assert.ok(edited.includes('<RotateTransform Angle="20"/><TranslateTransform X="4"/>'));
    assert.ok(!edited.includes('KeyTime="0:0:0"'), 'The underlying transform still supplies the implicit starting value');
    const reference = await preview.renderHosted(update(4121, transformCard(50, 24, false)));
    assert.ok(pose.frame.image.png === reference.image.png, 'The sampled rotation and translation match a static native frame');
    await assert.rejects(preview.hostedCurve(identity(first), query), /frame changed/i);

    const rust = `fn separate() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn preview() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#ReviewCard").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(0.5, |key| key.set(ferroui_base::media::RotateTransform::angle_property(), /* angle 🧭 */ 80.0f64))
                .key_frame(1.0, |key| key.set(ferroui_base::media::RotateTransform::angle_property(), 100.0))
        )],
        Border { name: "ReviewCard", width: 128.0, height: 72.0, background: "#305DB6" }
    }
} }
`.replaceAll('\n', '\r\n');
    const draft = update(4122, rust);
    const mapped = await preview.renderHosted({ ...draft, projectPath: '/TransformReview.rs', document: { ...draft.document,
        uri: new URL('TransformReview.rs', draft.document.uri).toString(), language: 'rust', caret: { line: 2, character: 6 } } });
    const rotation = transformTrack(mapped, angle);
    const mappedAngles = await preview.hostedCurve(identity(mapped), rotation.query);
    assert.deepEqual(mappedAngles.find(point => point.time === .5), { time: .5, value: 40 });
    const change = await preview.planHostedKeyframe(identity(mapped), { program: rotation.program.id, target: rotation.track.target,
        property: angle, previous: 1, time: 1, value: '72', easing: 'LinearEasing' });
    assert.ok(change.patch);
    assert.equal(applyPatch(rust, change.patch), rust.replace('80.0f64', '72.0f64'));
    console.log('App Preview sampled rotation and translation from native transforms, matched pixels, and preserved mapped Rust while editing a key');
}

function transformTrack(frame: HostedFrame, property: string) {
    const program = frame.animations?.programs.find(program => program.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    assert.equal(track.canSample, true);
    return { program, track, query: { program: program.id, target: program.defaultTargets[0], property, window } };
}

function applyPatch(source: string, patch: { start: number; end: number; expected: string; replacement: string }): string {
    assert.equal(source.slice(patch.start, patch.end), patch.expected);
    return source.slice(0, patch.start) + patch.replacement + source.slice(patch.end);
}

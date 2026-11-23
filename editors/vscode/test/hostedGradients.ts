import { strict as assert } from 'node:assert';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';

const property = 'Background';
const first = '<LinearGradientBrush StartPoint="0%,0%" EndPoint="100%,10%">'
    + '<GradientStop Offset="0.2" Color="#FF163148"/><GradientStop Offset="1" Color="#FF346C98"/></LinearGradientBrush>';
const second = '<LinearGradientBrush StartPoint="40%,0%" EndPoint="100%,10%" Opacity="0.8">'
    + '<GradientStop Offset="0.6" Color="#FF337BA4"/><GradientStop Offset="1" Color="#FF62BAAB"/></LinearGradientBrush>';
const radial = '<RadialGradientBrush Center="40%,60%" RadiusX="70%" RadiusY="30%">'
    + '<GradientStop Offset="0.4" Color="#FF51BDA3"/><GradientStop Offset="1" Color="#FF194852"/></RadialGradientBrush>';
const window = { start: 0, end: 2 };
const offset = 'gradient_stop/1/stop_offset';
const identity = ({ revision, sequence }: HostedFrame) => ({ revision, sequence });

function card(brush: string, animation = '') {
    return `<Window xmlns="https://github.com/ferroui" Width="420" Height="300" Background="#E9EEF7">
    ${animation}
    <Border Name="Card" Width="230" Height="126" CornerRadius="12">
        <Border.Background>${brush}</Border.Background>
    </Border>
</Window>`;
}

export async function verifyHostedGradients(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = card(first, `<Window.Styles><Style Selector="Border#Card"><Style.Animations>
    <Animation Duration="0:0:2" FillMode="Forward">
        <KeyFrame Cue="50%"><Setter Property="Background"><Setter.Value>${second}</Setter.Value></Setter></KeyFrame>
        <KeyFrame Cue="100%"><Setter Property="Background">${radial}</Setter></KeyFrame>
    </Animation>
</Style.Animations></Style></Window.Styles>`);
    const frame = await preview.renderHosted(update(4150, source));
    const { program, track, query } = trackOf(frame);
    assert.equal(track.canSample, true);
    assert.ok(track.curve_channels?.includes('gradient_stop/2/blue'));
    assert.ok(track.curve_channels?.includes('gradient/radius_y/percent'));
    const curve = await preview.hostedCurve(identity(frame), query);
    assert.ok(Math.abs(curve.find(point => point.time === .5)!.value - .4) < 1e-12);
    assert.ok(curve.some(point => point.time === 1.5 && point.break_before));
    const centers = await preview.hostedCurve(identity(frame), { ...query, channel: 'gradient/center/percent_x' });
    assert.ok(centers.every(point => point.time >= 1.5 && Math.abs(point.value - 40) < 1e-10));
    const velocity = await preview.hostedCurve(identity(frame), { ...query, metric: 'velocity' });
    assert.ok(velocity.some(point => point.time > .4 && point.time < .6 && Math.abs(point.value - .4) < 1e-9));
    const pose = await preview.sampleHostedAnimation({ ...identity(frame), program: program.id, elapsed: .5, restart: true });
    assert.deepEqual(await preview.hostedCurve(identity(pose.frame), query), curve);
    const key = track.keyframes[0];
    assert.equal(key.curve_values?.find(value => value.channel === offset)?.value, .6);
    const component = await preview.planHostedKeyframe(identity(pose.frame), {
        program: program.id, target: track.target, property, previous: key.time, time: key.time,
        easing: key.easing_text, component: { channel: offset, value: .44 },
    });
    assert.ok(component.patch && component.resolvedKey);
    assert.match(component.resolvedKey.value_text, /GradientStop Offset="0\.44"/);
    assert.match(apply(source, component.patch), /Color="#FF62BAAB"/);
    const result = await preview.planHostedKeyframe(identity(pose.frame), { program: program.id, target: track.target,
        property, time: .5, previewTarget: query.target, sample: true });
    assert.equal(result.sampledKey?.value_kind, 'Brush');
    assert.ok(result.patch && result.sampledKey);
    assert.equal(apply(source, result.patch).match(/<KeyFrame /g)?.length, 3);
    assert.match(result.sampledKey.value_text, /<Setter.Value/);
    const stopped = await preview.sampleHostedAnimation({ ...identity(pose.frame), elapsed: 0, restart: true });
    assert.equal(stopped.frame.image.png, frame.image.png, 'Stop restores the original gradient');
    const sample = result.sampledKey.value_text.replace(/^<Setter\.Value>|<\/Setter\.Value>$/g, '');
    const reference = await preview.renderHosted(update(4151, card(sample)));
    assert.equal(pose.frame.image.png, reference.image.png, 'The sampled gradient matches the native static brush');
    await assert.rejects(preview.hostedCurve(identity(frame), query), /frame changed/i);
    await verifyMappedGradient(preview, update);
    await verifyMappedCircularGradient(preview, update);
    await verifyMappedSolidBrush(preview, update);
    console.log('App Preview plotted native gradient geometry and stops, matched the recorded native frame, and preserved mapped Rust');
}

async function verifyMappedGradient(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const source = `use ferroui_view::{prelude::*, animation as clip, ms as duration};
use ferroui_base::media::{Color as Paint, IBrush as Brush,
    immutable::{ImmutableConicGradientBrush as Conic, ImmutableGradientStop as Stop}};
use ferroui_controls::Border as Card;
use std::rc::Rc as Shared;
fn title() { rsx! { TextBlock { text: "Keep 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [style("Border#Card").animation(
            clip(duration(2000))
                .key_frame(0.0, |key| key.set(Card::background_property(), "#1C3450"))
                .key_frame(1.0, |key| key.set(Card::background_property(),
                    Some(Shared::new(Conic::from_stops(&[
                        Stop::new(/* sweep 🧭 */ 0.28f64, Paint::from_uint32(0xFF315E88)),
                        Stop::new(0.9, Paint::from_uint32(0xFF58BAA7)),
                    ])) as Shared<dyn Brush>)))
        )],
        Border { name: "Card", width: 230.0, height: 126.0, background: "#1C3450" }
    }
} }
`.replaceAll('\n', '\r\n');
    const draft = update(4152, source);
    const frame = await preview.renderHosted({ ...draft, projectPath: '/GradientView.rs', document: { ...draft.document,
        uri: new URL('GradientView.rs', draft.document.uri).toString(), language: 'rust',
        caret: { line: source.split('\r\n').findIndex(line => line.startsWith('fn view()')), character: 16 } } });
    const { program, track, query } = trackOf(frame);
    assert.ok(track.curve_channels?.includes('gradient/angle/angle'));
    assert.ok((await preview.hostedCurve(identity(frame), query)).every(point => Math.abs(point.value - .28) < 1e-12));
    const value = track.keyframes.at(-1)!.value_text.replace('Offset="0.28"', 'Offset="0.34"');
    const edit = { program: program.id, target: track.target, property, previous: 2, time: 2, easing: 'LinearEasing' };
    await assert.rejects(preview.planHostedKeyframe(identity(frame), { ...edit, value: value.replace('0.34', '{Binding Position}') }));
    const { patch } = await preview.planHostedKeyframe(identity(frame), { ...edit, component: { channel: offset, value: .34 } });
    assert.ok(patch);
    assert.equal(apply(source, patch), source.replace('0.28f64', '0.34f64'));
}

async function verifyMappedCircularGradient(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const original = `fn headline() { rsx! { TextBlock { text: "Keep this heading 🧭" } } }
fn view() { rsx! {
    StackPanel {
        styles: [ferroui_view::style("Border#Card").animation(
            ferroui_view::animation(ferroui_view::ms(2000))
                .key_frame(1.0, |key| key.set(ferroui_controls::Border::background_property(),
                    Some(std::rc::Rc::new(ferroui_base::media::immutable::ImmutableRadialGradientBrush::with_radius(&[
                        ferroui_base::media::immutable::ImmutableGradientStop::new(0.0, ferroui_base::media::Color::from_uint32(0xFFEAF2FF)),
                        ferroui_base::media::immutable::ImmutableGradientStop::new(1.0, ferroui_base::media::Color::from_uint32(0xFF276280)),
                    ], 1.0, None, None, ferroui_base::media::GradientSpreadMethod::Pad, None, None,
                    /* native shared radius 🧭 */ 0.32f64)) as std::rc::Rc<dyn ferroui_base::media::IBrush>)))
                .fill_mode(ferroui_base::animation::FillMode::Both)
        )],
        Border { name: "Card", width: 230.0, height: 126.0, background: "#1C3450" }
    }
} }
`.replaceAll('\n', '\r\n');
    const source = (revision: number, text: string) => {
        const draft = update(revision, text);
        return { ...draft, projectPath: '/RadialGradientView.rs', document: { ...draft.document,
            uri: new URL('RadialGradientView.rs', draft.document.uri).toString(), language: 'rust' as const,
            caret: { line: 1, character: 16 },
        } };
    };
    const initial = await preview.renderHosted(source(4153, original));
    const { program, track } = trackOf(initial);
    const checkRadius = async (frame: HostedFrame, axis: 'x' | 'y', expected: number) => {
        const { query } = trackOf(frame);
        const curve = await preview.hostedCurve(identity(frame), { ...query, channel: `gradient/radius_${axis}/percent` });
        assert.ok(curve.some(point => point.time === 2 && Math.abs(point.value - expected) < 1e-10));
    };
    await checkRadius(initial, 'x', 32);
    await checkRadius(initial, 'y', 32);
    const before = await preview.sampleHostedAnimation({ ...identity(initial), program: program.id, elapsed: 2, restart: true });
    const key = track.keyframes.at(-1)!;
    const channel = 'gradient/radius_y/percent';
    assert.equal(key.curve_values?.find(value => value.channel === channel)?.value, 32);
    const draft = { program: program.id, target: track.target, property, previous: key.time, time: key.time,
        easing: key.easing_text, component: { channel, value: 47 } } as const;
    await assert.rejects(preview.planHostedKeyframe(identity(before.frame), {
        ...draft, component: { channel: 'gradient/radius_y/absolute', value: 47 },
    }), /no editable component/i);
    const { patch, resolvedKey } = await preview.planHostedKeyframe(identity(before.frame), draft);
    assert.match(resolvedKey?.value_text ?? '', /RadiusX="32%" RadiusY="47%"/);
    assert.ok(patch);
    const revised = apply(original, patch);
    assert.match(revised, /ImmutableRadialGradientBrush::new\(/);
    assert.match(revised, /native shared radius 🧭 \*\/ Some\(/u);
    assert.doesNotMatch(revised, /(?<!\r)\n/);
    assert.equal(revised.split('fn view()')[0], original.split('fn view()')[0]);
    const accepted = await preview.renderHosted(source(4154, revised));
    await assert.rejects(preview.planHostedKeyframe(identity(before.frame), draft), /frame changed/i);
    await checkRadius(accepted, 'x', 32);
    await checkRadius(accepted, 'y', 47);
    const editedProgram = trackOf(accepted).program;
    const pose = await preview.sampleHostedAnimation({ ...identity(accepted), program: editedProgram.id, elapsed: 2, restart: true });
    assert.notEqual(pose.frame.image.png, before.frame.image.png);
}

async function verifyMappedSolidBrush(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const packed = 'Solid::from_uint32(/* preserve 🧭 */ 0xFF33_6688_u32)';
    const original = `use ferroui_view::prelude::*;
use ferroui_controls::Border as Card;
use ferroui_base::{animation::FillMode, media::{IBrush, Color, immutable::ImmutableSolidColorBrush as Solid}};
use std::rc::Rc;
fn view() -> Element { rsx! {
    StackPanel {
        styles: [style("Border#Card").animation(animation(ms(2000))
            .key_frame(0.0, |key| key.set(Card::background_property(), Some(Rc::new(${packed}) as Rc<dyn IBrush>)))
            .key_frame(1.0, |key| key.set(Card::background_property(),
                Some(Rc::new(Solid::new(Color::from_rgb(51, 102, 136))) as Rc<dyn IBrush>)))
            .fill_mode(FillMode::Both))],
        Border { name: "Card", width: 230.0, height: 126.0, background: "#336688" }
    }
} }
`.replaceAll('\n', '\r\n');
    const source = (revision: number, text: string): HostedSource => {
        const draft = update(revision, text);
        return { ...draft, projectPath: '/SolidBrushMotion.rs', document: { ...draft.document,
            uri: new URL('SolidBrushMotion.rs', draft.document.uri).toString(), language: 'rust',
            caret: { line: 4, character: 25 },
        } };
    };
    const frame = await preview.renderHosted(source(4155, original));
    const { program, track } = trackOf(frame);
    const channel = 'gradient/opacity/value';
    assert.equal(track.keyframes[0].curve_values?.find(value => value.channel === channel)?.value, 1);
    const { patch } = await preview.planHostedKeyframe(identity(frame), { program: program.id,
        target: track.target, property, previous: 0, time: 0, easing: 'LinearEasing', component: { channel, value: .6 } });
    assert.ok(patch);
    const revised = apply(original, patch);
    assert.equal(revised, original.replace(packed,
        'Solid::with_opacity(/* preserve 🧭 */ ::ferroui_base::media::Color::from_uint32(0xFF33_6688_u32), 0.6)'));
    const accepted = await preview.renderHosted(source(4156, revised));
    const curve = await preview.hostedCurve(identity(accepted), { ...trackOf(accepted).query, channel });
    assert.equal(curve.find(point => point.time === 1)?.value, .8);
}

function trackOf(frame: HostedFrame) {
    const program = frame.animations?.programs.find(clip => clip.curveTracks?.some(track => track.property === property));
    const track = program?.authoring?.tracks.find(track => track.property === property);
    assert.ok(program && track);
    return { program, track, query: { program: program.id, target: program.defaultTargets[0], property, channel: offset, window } } as const;
}

function apply(source: string, patch: { start: number; end: number; expected: string; replacement: string }) {
    assert.equal(source.slice(patch.start, patch.end), patch.expected);
    return source.slice(0, patch.start) + patch.replacement + source.slice(patch.end);
}

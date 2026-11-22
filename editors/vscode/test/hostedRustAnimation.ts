import { strict as assert } from 'node:assert';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import type { BuiltProject } from '../src/builtProjects';
import type { HostedAnimationPosition } from '../src/hostedAnimationProtocol';
import type { HostedFrame } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { Source } from '../src/protocol';
import type { AnimationSourceIssue, TimelineProgram } from '../src/timelineProtocol';
import { applyClipDraft } from './hostedClips';
import { verifyHostedRustClipAuthoring } from './hostedRustClipAuthoring';

const getter = 'crate::app_preview::motion::review_corner_property';
const style = `styles: [ferroui_view::style("Border#ReviewCard")
    .animation(ferroui_view::animation(ferroui_view::ms(2000))
        .key_frame(0.0, |key| key.set(${getter}(), ferroui_base::CornerRadius::uniform(0.0)))
        .key_frame(1.0, |key| key.set(${getter}(),
            ferroui_base::CornerRadius::new(32.0, /* preserve 🧭 */ 32.0, 32.0, 32.0)))
        .fill_mode(ferroui_base::animation::FillMode::Both))
    .animation(motion_from_database(model))],`;

const source = `fn separate() { rsx! { TextBlock { text: "Other 🪴 approach" } } }
fn render() { rsx! {
    StackPanel { width: 380.0, height: 220.0,
        ${style}
        Border { name: "ReviewCard", width: 180.0, height: 100.0, corner_radius: 0.0, background: "#305DB6" }
    }
}}
`.replaceAll('\n', '\r\n');

export async function verifyHostedRustAnimation(preview: NativePreview): Promise<void> {
    const sandbox = await mkdtemp(join(tmpdir(), 'ferroui-hosted-rust-clip-'));
    try {
        const manifest = join(sandbox, 'Cargo.toml');
        await writeFile(manifest, '[package]\nname = "todo-workbench"\nversion = "0.0.0"\n');
        const project: BuiltProject = { workspaceRoot: sandbox, metadata: { manifest, directories: [sandbox], package: 'todo-workbench' } };
        await writeFile(join(sandbox, 'app.xamlmeta'), JSON.stringify({
            format: 2, name: 'todo-workbench', crate_name: 'todo_workbench',
            metadata: [[`FerroUI.Designer.RsxAnimation.v1:${getter}`, '::ferroui_controls::Border::corner_radius_property']],
        }));
        await writeFile(join(sandbox, 'controls.xamlmeta'), JSON.stringify({
            format: 2, name: 'FerroUI.Controls', crate_name: 'ferroui_controls',
            types: [{ name: 'Border', namespace: 'FerroUI.Controls', kind: 'class', rust_path: '::ferroui_controls::Border',
                object_model: true, registered: [{ name: 'CornerRadius', kind: 'styled', accessor: 'corner_radius_property',
                    registration: 'declared', value_type: '::ferroui_base::corner_radius::CornerRadius' }] }],
        }));
        const documentFor = (text: string) => preview.inProject({ uri: pathToFileURL(join(sandbox, 'motion.rs')).toString(),
            source: text, language: 'rust', caret: { line: 2, character: 8 } }, project);
        const render = async (revision: number, text: string, previous?: { frame: HostedFrame; position: HostedAnimationPosition }) => {
            const document = await documentFor(text);
            const restoreAnimation = previous && {
                frame: { revision: previous.frame.revision, sequence: previous.frame.sequence },
                presentation: { selection: previous.position.program, position: previous.position, overrides: {} },
            };
            return preview.renderHosted({ revision, document, assembly: 'todo-workbench', projectPath: '/motion.rs', restoreAnimation });
        };
        await verifyIsolatedRustAnimation(preview, await documentFor(source));
        const first = await render(40, source);
        const [clip] = first.animations?.programs ?? [];
        assert.ok(clip?.customAnimator && clip.authoring && clip.markerKey);
        assert.equal(first.animations?.programs.length, 1, 'An opaque neighbor does not suppress the mapped clip');
        assertSourceIssue(first.animations?.sourceIssues);
        assert.equal(clip.sourceOffset, source.indexOf('ferroui_view::animation('));
        const [track] = clip.authoring.tracks;
        assert.equal(track.property, 'CornerRadius');
        assert.equal(track.keyframes[1].value_kind, 'CornerRadius');
        const paused = await preview.sampleHostedAnimation({ revision: first.revision, sequence: first.sequence,
            program: clip.id, elapsed: 1, restart: true });
        const frame = { revision: paused.frame.revision, sequence: paused.frame.sequence };
        const keyframes = track.keyframes.map(key => ({ property: track.property, target: track.target, time: key.time }));
        const copied = await preview.planHostedKeyframeGroup(frame, {
            program: clip.id, command: { action: 'copy_keyframes', keyframes },
        });
        assert.match(copied.clipboard ?? '', /"kind":"CornerRadius"/);
        const scaled = await preview.planHostedKeyframeGroup(frame, {
            program: clip.id, command: { action: 'scale_keyframes', keyframes, pivot: 0, factor: .75 },
        });
        assert.deepEqual(scaled.keys?.map(key => key.time), [0, 1.5]);
        assert.equal(applyClipDraft(source, scaled.patches), source.replace('.key_frame(1.0,', '.key_frame(0.75,'));
        const radius = await preview.hostedCurve(frame, { program: clip.id, target: clip.defaultTargets[0],
            property: track.property, channel: 'bottom_right', window: { start: 0, end: 2 } });
        assert.equal(radius.find(point => point.time === 1)?.value, 8, 'Read the registered Rust corner channel without seeking');
        const draft = { program: clip.id, target: track.target, property: track.property, previous: 2, time: 1.6,
            value: '51.2,51.2,51.2,51.2', easing: track.keyframes[1].easing_text };
        await assert.rejects(preview.planHostedKeyframe(first, draft), /frame changed/i);
        await assert.rejects(preview.planHostedKeyframe(frame, { ...draft, value: 'wrong' }), /invalid numeric/i);
        const plan = await preview.planHostedKeyframe(frame, draft);
        assert.equal(plan.valueKind, 'CornerRadius');
        assert.ok(plan.patch);
        const { start, end, expected, replacement } = plan.patch;
        assert.equal(source.slice(start, end), expected);
        const edited = source.slice(0, start) + replacement + source.slice(end);
        assert.equal(edited, source.replace('.key_frame(1.0,', '.key_frame(0.8,').replaceAll('32.0', '51.2'));

        const retained = { program: clip.id, elapsed: 1, playing: false, completed: false };
        const next = await render(41, edited, { frame: paused.frame, position: retained });
        const rebuilt = next.animations?.programs[0];
        assert.ok(rebuilt);
        assert.equal(rebuilt.markerKey, clip.markerKey, 'Mapped Rust clips keep their marker identity through key edits');
        assert.ok(rebuilt.authoring?.tracks[0].keyframes.some(key => key.time === 1.6));
        assert.deepEqual(next.animationPresentation?.position, { ...retained, program: rebuilt.id });
        const resumed = await render(42, edited.replace('preserve 🧭', 'preserve the Rust comment 🧭'), {
            frame: next, position: { ...retained, program: rebuilt.id, playing: true },
        });
        assert.equal(resumed.animationPresentation?.position?.playing, true);
        assert.equal(resumed.image.png, next.image.png);
        const frozen = (radius: number) => source.replace(style.replaceAll('\n', '\r\n'), '')
            .replace('corner_radius: 0.0', `corner_radius: ${radius}.0`);
        const before = await render(43, frozen(8));
        const after = await render(44, frozen(20));
        assert.equal(paused.frame.image.png, before.image.png, 'The registered CornerRadius animator reaches 8 at one second');
        assert.equal(next.image.png, after.image.png, 'The first frame after a Rust key edit is already sampled at the retained time');
        const withDelay = source.replace('.key_frame(0.0,', '.delay(ferroui_view::ms(/* hold ⏱ */ 125u64))\r\n        .key_frame(0.0,');
        const delayed = await render(45, withDelay);
        const delayedClip = delayed.animations?.programs[0];
        assert.equal(delayedClip?.clip?.start, .125);
        assert.ok(delayedClip?.clip?.editable);
        const placement = await preview.planHostedClipPlacement({ revision: delayed.revision, sequence: delayed.sequence },
            { programs: [delayedClip.id], start: .375 });
        const placed = applyClipDraft(withDelay, placement.patches);
        assert.equal(placed, withDelay.replace('125u64', '375u64'));
        const placedFrame = await render(46, placed);
        const placedClip = placedFrame.animations?.programs[0];
        assert.equal(placedClip?.clip?.start, .375);
        assert.ok(placedClip);
        const shared = await preview.sampleHostedAnimation({ revision: placedFrame.revision, sequence: placedFrame.sequence,
            clips: [{ program: placedClip.id }], elapsed: 1.375, restart: true });
        assert.equal(shared.frame.image.png, before.image.png);
        const accepted = { revision: shared.frame.revision, sequence: shared.frame.sequence };
        const lane = placedClip.authoring?.tracks[0];
        assert.ok(lane?.canSample);
        const location = { program: placedClip.id, target: lane.target, property: lane.property };
        const sample = { ...location, time: 1, sample: true } as const;
        await assert.rejects(preview.planHostedKeyframe(placedFrame, sample), /frame changed/i);
        await assert.rejects(preview.planHostedKeyframe(accepted, { ...sample, time: 1.999999999 }), /occupies/i);
        await assert.rejects(preview.planHostedKeyframe(accepted, { ...sample, previewTarget: Number.MAX_SAFE_INTEGER }), /compatible live/i);
        const record = await preview.planHostedKeyframe(accepted, sample);
        assert.equal(record.sampledKey?.time, 1);
        assert.equal(record.sampledKey?.value_text, '8,8,8,8', 'Capture the registered quadratic animator at one authored second');
        assert.ok(record.patch);
        assert.equal(record.patch.expected, '');
        const recorded = applyClipDraft(placed, [record.patch]);
        assert.match(recorded, /\.key_frame\(0\.5, \|key\| key\.set\(.+CornerRadius::new\(8\.0, 8\.0, 8\.0, 8\.0\)\)\)/);
        assert.ok(recorded.includes('/* preserve 🧭 */'));
        assert.ok(recorded.includes('motion_from_database(model)'));
        assert.ok(!recorded.replaceAll('\r\n', '').includes('\n'));
        const explicit = await preview.planHostedKeyframe(accepted, { ...location, time: .75,
            value: '6,7,8,9', easing: 'LinearEasing' });
        assert.ok(explicit.patch);
        assert.match(applyClipDraft(placed, [explicit.patch]), /\.key_frame\(0\.375,.+CornerRadius::new\(6\.0, 7\.0, 8\.0, 9\.0\)/);
        const withKey = await render(47, recorded, { frame: shared.frame, position: {
            program: placedClip.id, clips: [placedClip.id], elapsed: 1.375, playing: false, completed: false,
        } });
        assert.equal(withKey.animations?.programs[0].authoring?.tracks[0].keyframes.find(key => key.time === 1)?.value_text, '8,8,8,8');
        assert.equal(withKey.animationPresentation?.position?.elapsed, 1.375);
        assert.equal(withKey.image.png, shared.frame.image.png, 'The first replacement frame holds the same sampled native pose');
        console.log('App Preview edited and inserted registered Rust keys with checked CRLF patches and retained native poses');
        await verifyHostedRustClipAuthoring(preview, source, source.replace(style.replaceAll('\n', '\r\n'), 'styles: styles_for(model),'),
            documentFor, withKey.revision);
    } finally { await rm(sandbox, { recursive: true, force: true }); }
}

async function verifyIsolatedRustAnimation(preview: NativePreview, document: Source): Promise<void> {
    const frame = 803;
    const previewed = await preview.render(document, { width: 380, height: 220, dark: false }, frame);
    assert.ok(previewed.image && previewed.mappedRustAnimations, previewed.renderError ?? 'Render the project-mapped RSX clip');
    const catalog = await preview.studioQuery(frame, 'designer_programs', { include_keyframe_text: true }) as {
        programs: TimelineProgram[]; sourceIssues?: AnimationSourceIssue[];
    };
    assertSourceIssue(catalog.sourceIssues);
    const [program] = catalog.programs;
    const track = program?.timeline?.tracks[0];
    assert.ok(program.sourceRange && track);
    const groupMove = await preview.studioEdit(document, frame, 'designer_program', {
        action: 'move_keyframes', program: program.id, offset: -0.4,
        keyframes: [{ target: track.target, property: 'CornerRadius', time: 2 }],
    }, []);
    assert.deepEqual(groupMove.edits.map(edit => edit.newText), ['0.8']);
    await preview.render(document, { width: 380, height: 220, dark: false }, frame + 1);
    const draft = { program: program.id, target: track.target, property: 'CornerRadius', previous: 2, time: 2,
        value: '40,42,44,46', easing: track.keyframes[1].easing_text };
    const check = await preview.studioQuery(frame + 1, 'designer_keyframe_draft', draft) as { valueKind: string };
    assert.equal(check.valueKind, 'CornerRadius');
    const plan = await preview.studioEdit(document, frame + 1, 'designer_program', { action: 'upsert_keyframe', ...draft }, []);
    assert.deepEqual(plan.edits.map(edit => edit.newText), ['40.0', '42.0', '44.0', '46.0']);
}

function assertSourceIssue(issues: readonly AnimationSourceIssue[] | undefined): void {
    assert.equal(issues?.length, 1);
    const [issue] = issues!;
    const { start, end } = issue.range;
    assert.equal(start.line, end.line);
    assert.equal(source.split('\r\n')[start.line].slice(start.character, end.character), 'motion_from_database(model)');
    assert.match(issue.reason, /computed expression/);
}

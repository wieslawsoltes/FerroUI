import { strict as assert } from 'node:assert';
import type { HostedClipEdit } from '../src/hostedAnimationProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { Source } from '../src/protocol';
import { applyClipDraft } from './hostedClips';

export async function verifyHostedRustClipAuthoring(preview: NativePreview, source: string, opaque: string,
    documentFor: (text: string) => Promise<Source>, lastRevision: number) {
    const document = await documentFor(source);
    let revision = lastRevision;
    const render = (text: string, selectClipKey?: string) => preview.renderHosted({
        revision: ++revision, document: { ...document, source: text }, assembly: 'todo-workbench', projectPath: '/motion.rs', selectClipKey,
    });
    const ambiguous = await render(opaque);
    assert.equal(ambiguous.animations?.clipAuthoring?.owners.some(owner => owner.typeName === 'StackPanel'), false);
    const frame = await render(source);
    const owner = frame.animations?.clipAuthoring?.owners.find(item => item.typeName === 'StackPanel');
    assert.ok(owner);
    assert.equal(owner.sourceLine, source.split('\r\n').findIndex(line => line.includes('StackPanel { width')) + 1);
    assert.equal(frame.animations?.clipAuthoring?.programs.length, 1, 'Only the mapped Rust clip can be edited');
    const create: HostedClipEdit = { action: 'create', owner: owner.id, name: 'Rust entrance', selector: 'Border#ReviewCard', duration: .85 };
    await assert.rejects(preview.planHostedClipEdit(frame, { ...create, duration: .8501 }), /whole milliseconds/i);
    const authored = await preview.planHostedClipEdit(frame, create);
    assert.ok(authored.selectedKey);
    const text = applyClipDraft(source, authored.patches);
    assert.match(text, /ferroui_view::ms\(850\)/);
    assert.match(text, /motion_from_database\(model\)/);
    assert.match(text, /\/\* preserve 🧭 \*\//u);
    assert.doesNotMatch(text, /(?<!\r)\n/);
    assert.equal(text.split('fn render()')[0], source.split('fn render()')[0]);
    const created = await render(text, authored.selectedKey);
    const inserted = created.animations?.programs.find(program => program.sourceKey === authored.selectedKey);
    assert.ok(inserted);
    assert.equal(inserted.name, 'Rust entrance');
    assert.equal(created.animationPresentation?.selection, inserted.id);
    assert.equal(inserted.duration, .85);

    const originalClip = created.animations!.programs.find(program => program.customAnimator)!;
    const renamed = await preview.planHostedClipEdit(created, { action: 'rename', program: originalClip.id, name: 'Review corners' });
    const namedText = applyClipDraft(text, renamed.patches);
    const named = await render(namedText, renamed.selectedKey!);
    const clip = named.animations!.programs.find(program => program.sourceKey === renamed.selectedKey)!;
    assert.equal(clip.name, 'Review corners');
    const duplicated = await preview.planHostedClipEdit(named, { action: 'duplicate', program: clip.id, name: 'Alternate corners' });
    const copiedText = applyClipDraft(namedText, duplicated.patches);
    const copied = await render(copiedText, duplicated.selectedKey!);
    const copy = copied.animations!.programs.find(program => program.sourceKey === duplicated.selectedKey)!;
    assert.notEqual(copy.sourceKey, clip.sourceKey);
    assert.deepEqual(copy.authoring?.tracks[0].keyframes, clip.authoring?.tracks[0].keyframes);
    const removal = await preview.planHostedClipEdit(copied, { action: 'remove', program: copy.id });
    const removed = applyClipDraft(copiedText, removal.patches);
    const result = await render(removed);
    assert.deepEqual(result.animations?.programs.map(program => program.name), ['Review corners', 'Rust entrance']);
    assert.match(removed, /motion_from_database\(model\)/);
    assert.match(removed, /\/\* preserve 🧭 \*\//u);
    assert.doesNotMatch(removed, /(?<!\r)\n/);
    await assert.rejects(preview.planHostedClipEdit(copied, { action: 'remove', program: copy.id }), /frame changed/i);
    console.log('App Preview authored whole mapped Rust clips and preserved projected neighbors, CRLF, and comments');
}

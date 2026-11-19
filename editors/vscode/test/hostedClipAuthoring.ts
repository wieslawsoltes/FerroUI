import { strict as assert } from 'node:assert';
import type { HostedClipEdit } from '../src/hostedAnimationProtocol';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import { animationFixture, cornerKeys } from './hostedAnimation';
import { applyClipDraft } from './hostedClips';

export async function verifyHostedClipAuthoring(preview: NativePreview, update: (revision: number, source: string) => HostedSource) {
    const plan = (frame: HostedFrame, command: HostedClipEdit) => preview.planHostedClipEdit(frame, command);
    const plain = animationFixture(0, '<!-- Preserve this view 🚀 -->').replaceAll('\n', '\r\n');
    const empty = await preview.renderHosted(update(180, plain));
    const owner = empty.animations?.clipAuthoring?.owners.find(item => item.label === 'ReviewCard');
    assert.ok(owner, 'Expose live owners even when there are no animations');
    assert.deepEqual(empty.animations?.programs, []);
    assert.equal(owner.selector, 'Border#ReviewCard');
    await assert.rejects(plan(empty, { action: 'create', owner: 9999, name: 'Retired', selector: owner.selector, duration: 1 }), /style owner/i);
    await assert.rejects(plan(empty, { action: 'create', owner: owner.id, name: 'Invalid', selector: 'Border[', duration: 1 }), /selector|closing|expected/i);
    const draft = await plan(empty, { action: 'create', owner: owner.id, name: 'Enter review', selector: owner.selector, duration: .85 });
    assert.ok(draft.selectedKey);
    const authored = applyClipDraft(plain, draft.patches);
    assert.match(authored, /<!-- Preserve this view 🚀 -->/u);
    assert.match(authored, /<Border\.Styles>/);
    assert.doesNotMatch(authored, /(?<!\r)\n/);
    const created = await preview.renderHosted({ ...update(181, authored), selectClipKey: draft.selectedKey });
    const [clip] = created.animations!.programs;
    assert.equal(clip.name, 'Enter review');
    assert.equal(clip.duration, .85);
    assert.equal(clip.sourceKey, draft.selectedKey);
    assert.equal(created.animationPresentation?.selection, clip.id);
    assert.deepEqual(created.animations?.clipAuthoring?.programs, [clip.id]);
    await assert.rejects(plan(empty, { action: 'remove', program: clip.id }), /frame changed/i);

    const typed = animationFixture(0, `<!-- Scene 🌱 -->
<Window.Styles><Style Selector="Border#ReviewCard"><Style.Animations>
<Animation Duration="0:0:2" FillMode="Both"><!-- Keep this key payload -->${cornerKeys}</Animation>
</Style.Animations></Style></Window.Styles>`).replaceAll('\n', '\r\n');
    const original = await preview.renderHosted(update(182, typed));
    const [legacy] = original.animations!.programs;
    assert.equal(legacy.sourceKey, undefined);
    const sample = await preview.sampleHostedAnimation({ revision: original.revision, sequence: original.sequence,
        program: legacy.id, elapsed: 1.125, restart: true });
    const restore = (frame: HostedFrame, program: number) => ({
        frame: { revision: frame.revision, sequence: frame.sequence },
        presentation: { selection: program, overrides: {}, position: { program, elapsed: 1.125, playing: false, completed: false } },
    });
    const named = await plan(sample.frame, { action: 'rename', program: legacy.id, name: 'Review corners' });
    assert.ok(named.selectedKey);
    const renamedText = applyClipDraft(typed, named.patches);
    const renamed = await preview.renderHosted({ ...update(183, renamedText), selectClipKey: named.selectedKey,
        restoreAnimation: restore(sample.frame, legacy.id) });
    const [namedClip] = renamed.animations!.programs;
    assert.equal(namedClip.name, 'Review corners');
    assert.equal(renamed.animationPresentation?.position?.elapsed, 1.125);
    assert.equal(renamed.image.png, sample.frame.image.png, 'Renaming a legacy clip retains its native pose');
    const undone = await preview.renderHosted({ ...update(184, typed), restoreAnimation: restore(renamed, namedClip.id) });
    assert.equal(undone.animationPresentation?.position?.elapsed, 1.125);
    assert.equal(undone.image.png, sample.frame.image.png, 'Undo across the new source identity retains the pose');

    const copyPlan = await plan(undone, { action: 'duplicate', program: undone.animations!.programs[0].id, name: 'Alternate corners' });
    assert.ok(copyPlan.selectedKey);
    const duplicatedText = applyClipDraft(typed, copyPlan.patches);
    assert.equal(duplicatedText.match(/Keep this key payload/g)?.length, 2);
    assert.doesNotMatch(duplicatedText, /(?<!\r)\n/);
    const duplicated = await preview.renderHosted({ ...update(185, duplicatedText), selectClipKey: copyPlan.selectedKey });
    const [base, copy] = duplicated.animations!.programs;
    assert.equal(copy.name, 'Alternate corners');
    assert.deepEqual(copy.authoring?.tracks[0].keyframes, base.authoring?.tracks[0].keyframes);
    assert.equal(duplicated.animationPresentation?.selection, copy.id);
    const removal = await plan(duplicated, { action: 'remove', program: copy.id });
    const removed = await preview.renderHosted(update(186, applyClipDraft(duplicatedText, removal.patches)));
    assert.equal(removed.animations?.programs.length, 1);
    await assert.rejects(plan(duplicated, { action: 'remove', program: copy.id }), /frame changed/i);
    console.log('App Preview created, renamed, copied and removed XAML clips, and kept the native pose across rename and Undo');
}

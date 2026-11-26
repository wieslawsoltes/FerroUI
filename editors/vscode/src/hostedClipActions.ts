import { newClipFields } from './animationPrompts';
import type { HostedAnimationProgram, HostedClipAuthoring, HostedClipEdit } from './hostedAnimationProtocol';
import { choose, editText } from './studioContext';

const title = 'FerroUI App Preview';

export async function chooseHostedClipEdit(
    authoring: HostedClipAuthoring, programs: readonly HostedAnimationProgram[], preferred?: number,
): Promise<HostedClipEdit | undefined> {
    const editable = programs.filter(program => authoring.programs.includes(program.id));
    const current = editable.find(program => program.id === preferred);
    const actions = [
        { action: 'create', label: '$(add) New Animation Clip…', enabled: authoring.owners.length > 0 },
        { action: 'rename', label: '$(edit) Rename Clip…', enabled: editable.length > 0 },
        { action: 'duplicate', label: '$(copy) Duplicate Clip…', enabled: editable.length > 0 },
        { action: 'remove', label: '$(trash) Remove Clip', enabled: editable.length > 0 },
    ] as const;
    const picked = await choose(current?.name ?? 'Animation Clips', actions.filter(action => action.enabled));
    if (!picked) return;
    if (picked.action === 'create') {
        const owner = await choose('Choose a live style owner', authoring.owners.map(owner => ({
            label: owner.label,
            description: owner.label === owner.selector ? `Line ${owner.sourceLine}` : `${owner.selector} · Line ${owner.sourceLine}`,
            owner,
        })));
        if (!owner) return;
        const fields = await newClipFields(owner.owner.selector, title);
        if (fields) return { action: 'create', owner: owner.owner.id, ...fields };
        return;
    }
    const clip = current ?? (await choose('Choose an authored clip', editable.map(clip => ({
        label: clip.name, description: clip.selector, clip,
    }))))?.clip;
    if (!clip) return;
    if (picked.action === 'remove') return { action: picked.action, program: clip.id };
    const copy = picked.action === 'duplicate';
    const name = (await editText(copy ? 'Name for the copied animation' : 'Animation name',
        copy ? `${clip.name} Copy` : clip.name, undefined, title))?.trim();
    if (name && (copy || name !== clip.name)) return { action: picked.action, program: clip.id, name };
}

import * as vscode from 'vscode';
import { choose, editText, editTime, type StudioContext } from './studioContext';
import { editAnimationTiming } from './studioAnimationTiming';
import type { TimelineProgram } from './timelineProtocol';

export async function createAnimationProgram(context: StudioContext, owner: number): Promise<void> {
    const name = (await editText('Animation name'))?.trim();
    if (!name) return;
    const node = context.panel.nodes.find(node => node.id === owner);
    const selector = (await editText('FerroUI selector', node?.name ? `${node.label}#${node.name}` : node?.label))?.trim();
    if (!selector) return;
    const duration = await editTime('Duration in seconds', 600, 1);
    if (duration !== undefined && duration > 0) await context.edit('designer_program', { action: 'create', owner, name, selector, duration });
}

export async function renameAnimationProgram(context: StudioContext, program: TimelineProgram): Promise<void> {
    const name = (await editText('Animation name', program.name))?.trim();
    if (name && name !== program.name) await context.edit('designer_program', { action: 'rename', program: program.id, name });
}

export async function chooseAnimationTargets(context: StudioContext, program: TimelineProgram): Promise<void> {
    const controls = context.panel.nodes.filter(node => node.bounds && node.selectable !== false);
    if (!controls.length) throw new Error('The current preview has no live source controls.');
    const items = controls.map(node => ({ label: node.name || node.label, description: node.name ? node.label : undefined,
        id: node.id, picked: program.targets.includes(node.id) }));
    const targets = await vscode.window.showQuickPick(items, { title: `Preview targets · ${program.name}`, canPickMany: true,
        matchOnDescription: true, placeHolder: 'Choose the controls that should play this animation' });
    if (targets) await context.present('designer_program', { action: 'preview_targets', program: program.id,
        targets: targets.length ? targets.map(target => target.id) : null });
}

export async function showAnimationActions(context: StudioContext, program: TimelineProgram | undefined): Promise<void> {
    const actions: { label: string; run: () => Promise<void> }[] = [];
    const nodes = context.panel.nodes;
    const targets = context.selection.filter(id => nodes.some(node => node.id === id && node.bounds));
    const owner = targets.at(-1) ?? nodes[0]?.id;
    if (context.writable && owner !== undefined) {
        actions.push({ label: '$(add) New animation program…', run: () => createAnimationProgram(context, owner) });
    }
    if (program) {
        const id = program.id;
        if (nodes.some(node => node.id === id)) {
            actions.push({ label: '$(go-to-file) Reveal animation in source', run: () => context.reveal(id) });
        }
        actions.push({ label: '$(target) Choose preview targets…', run: () => chooseAnimationTargets(context, program) });
        if (targets.length) {
            actions.push({ label: '$(preview) Preview on selected controls', run: () =>
                context.present('designer_program', { action: 'preview_targets', program: id, targets }) });
        }
        if (program.preview?.explicit_targets || !program.preview) {
            actions.push({ label: '$(discard) Clear preview target override', run: () =>
                context.present('designer_program', { action: 'preview_targets', program: id, targets: null }) });
        }
        if (context.writable) {
            const timeline = program.timeline;
            if (timeline) actions.push({ label: '$(watch) Timing and Playback…', run: () => editAnimationTiming(context, id, timeline) });
            actions.push(
                { label: '$(edit) Rename animation…', run: () => renameAnimationProgram(context, program) },
                { label: '$(copy) Duplicate animation…', run: async () => {
                    const name = (await editText('Name for the copied animation', `${program.name} Copy`))?.trim();
                    if (name) await context.edit('designer_program', { action: 'duplicate', program: id, name });
                } },
            );
            if (program.style != null) actions.push({ label: '$(target) Edit authored selector…', run: async () => {
                const selector = (await editText('FerroUI selector', program.selector))?.trim();
                if (selector && selector !== program.selector) {
                    await context.edit('designer_program', { action: 'selector', program: id, selector });
                }
            } });
            actions.push({ label: '$(trash) Remove animation', run: () =>
                context.edit('designer_program', { action: 'remove', program: id }) });
        }
    }
    const picked = await choose(program?.name ?? 'FerroUI Animation', actions);
    await picked?.run();
}

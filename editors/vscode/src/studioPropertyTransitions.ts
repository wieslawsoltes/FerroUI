import type { Range } from './protocol';
import type { TransitionCommand, TransitionEntry, TransitionList, TransitionTiming } from './propertyTransitionProtocol';
import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';
import { parseMappedEasing } from './stateAuthoringPrompts';

export const PROPERTY_TRANSITIONS = 'designer_property_transitions';
const EDIT_TRANSITION = 'designer_property_transition_edit';

export interface PropertyTransitionContext {
    read(targetType?: string): Promise<TransitionList>;
    edit(command: TransitionCommand, targetType?: string): Promise<void>;
    reveal(range: Range): Promise<void>;
}

function studioTransitions(context: StudioContext): PropertyTransitionContext {
    if ((!context.writable && !context.panel.supportsRustPropertyTransitions) || context.node === undefined) {
        throw new Error('Select an authored control in the current design.');
    }
    const owner = context.node;
    const target = (targetType?: string) => targetType === undefined ? { owner } : { owner, target_type: targetType };
    return {
        read: targetType => context.read<TransitionList>(PROPERTY_TRANSITIONS, target(targetType)),
        edit: (command, targetType) => context.edit(EDIT_TRANSITION, { ...target(targetType), ...command }),
        reveal: range => context.panel.language === 'rust' ? context.revealRustRange(range) : context.revealRange(range),
    };
}

async function chooseProperty(
    context: PropertyTransitionContext, list: TransitionList, title: string,
): Promise<{ property: string; type_name: string; target_type?: string } | undefined> {
    let targetType: string | undefined;
    if (list.requiresTarget) {
        targetType = (await editText('Registered XAML target type for this transition resource', 'Border',
            value => value.trim() ? undefined : 'Enter a registered target type.'))?.trim();
        if (targetType === undefined) return;
        list = await context.read(targetType);
    }
    const selected = await choose(title, (list.properties ?? []).map(property => ({
        label: property.label, description: property.typeName, property,
    })));
    if (selected) return {
        property: selected.property.property, type_name: selected.property.typeName,
        ...(targetType === undefined ? {} : { target_type: targetType }),
    };
}

function entryActions(context: PropertyTransitionContext, list: TransitionList, entry: TransitionEntry, index: number): StudioItem[] {
    const id = `transition:${entry.id}`;
    const actions = [action(`${id}:source`, 'Reveal transition in source', () => context.reveal(entry.range), 'go-to-file')];
    if (!list.canEdit) return actions;
    const update = (timing: TransitionTiming) => context.edit({ action: 'timing', transition: entry.id, timing });
    for (const [field, label, value] of [
        ['duration', 'Duration', entry.durationSeconds],
        ['delay', 'Start delay', entry.delaySeconds],
    ] as const) {
        if (value === null) continue;
        actions.push({
            id: `${id}:${field}`, label, description: `${value} s`, icon: 'watch',
            run: async () => {
                const input = await editText(`${label} in seconds`, String(value), value =>
                    value.trim() && Number.isFinite(Number(value)) && Number(value) >= 0
                        ? undefined : 'Enter a nonnegative number of seconds.');
                if (input !== undefined && Number(input) !== value) await update({ [field]: Number(input) });
            },
        });
    }
    if (entry.editableTiming.includes('Easing')) actions.push({
        id: `${id}:easing`, label: 'Easing', description: entry.easing ?? 'Linear', icon: 'graph-line',
        run: async () => {
            const input = await editText('Native easing name, four Bézier coordinates, or Spring settings',
                entry.easing ?? 'LinearEasing');
            if (input !== undefined && input.trim() !== (entry.easing ?? 'LinearEasing')) {
                await update({ easing: parseMappedEasing(input) });
            }
        },
    });
    if (!list.canEditCollection) return actions;
    if (entry.canRetarget && (list.requiresTarget || list.properties?.length)) {
        actions.push(action(`${id}:retarget`, 'Retarget transition', async () => {
            const property = await chooseProperty(context, list, 'Retarget transition');
            if (property) {
                const { target_type, ...choice } = property;
                await context.edit({ action: 'retarget', transition: entry.id, ...choice }, target_type);
            }
        }, 'target'));
    }
    if (index > 0) actions.push(action(`${id}:up`, 'Move transition earlier',
        () => context.edit({ action: 'move', transition: entry.id, index: index - 1 }), 'arrow-up'));
    if (index + 1 < list.entries.length) actions.push(action(`${id}:down`, 'Move transition later',
        () => context.edit({ action: 'move', transition: entry.id, index: index + 1 }), 'arrow-down'));
    actions.push(action(`${id}:remove`, 'Remove transition', () => context.edit({ action: 'remove', transition: entry.id }), 'trash'));
    return actions;
}

export function propertyTransitionItems(context: StudioContext): Promise<StudioItem[]> {
    return transitionItems(studioTransitions(context));
}

async function transitionItems(context: PropertyTransitionContext): Promise<StudioItem[]> {
    const list = await context.read();
    const items: StudioItem[] = list.entries.map((entry, index) => ({
        id: `transition:${entry.id}`, label: entry.property ?? entry.typeName,
        description: entry.durationSeconds === null ? entry.typeName : `${entry.durationSeconds} s`,
        icon: 'watch', children: async () => entryActions(context, list, entry, index),
    }));
    if (list.canAdd || (list.requiresTarget && list.canEditCollection)) {
        items.push(action('transitions:add', 'Add property transition', async () => {
            const property = await chooseProperty(context, list, 'Add property transition');
            if (property) {
                const { target_type, ...choice } = property;
                await context.edit({ action: 'add', ...choice }, target_type);
            }
        }, 'add'));
    }
    if (!items.length) {
        const range = list.range;
        return range ? [action('transitions:source', 'Reveal transition collection', () => context.reveal(range), 'go-to-file')]
            : [{ id: 'transitions:empty', label: 'No authored transitions on this control' }];
    }
    return items;
}

export function choosePropertyTransition(context: StudioContext): Promise<void> {
    return chooseTransitionAction(studioTransitions(context));
}

export async function chooseTransitionAction(context: PropertyTransitionContext): Promise<void> {
    const items = await transitionItems(context);
    const selected = await choose('Property transitions', items);
    if (!selected) return;
    if (selected.run) await selected.run();
    else if (selected.children) {
        const field = await choose(selected.label, await selected.children());
        await field?.run?.();
    }
}

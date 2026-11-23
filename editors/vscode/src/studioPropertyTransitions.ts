import type { Range } from './protocol';
import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';
import { parseMappedEasing } from './stateAuthoringPrompts';

export const PROPERTY_TRANSITIONS = 'designer_property_transitions';
const EDIT_TRANSITION = 'designer_property_transition_edit';

interface TransitionEntry {
    id: number;
    typeName: string;
    property: string | null;
    durationSeconds: number | null;
    delaySeconds: number | null;
    easing: string | null;
    editableTiming: string[];
    canRetarget?: boolean;
    range: Range;
}

interface TransitionProperty {
    property: string;
    label: string;
    typeName: string;
}

interface TransitionList {
    owner: number;
    canEdit: boolean;
    canEditCollection?: boolean;
    canAdd?: boolean;
    requiresTarget?: boolean;
    properties?: TransitionProperty[];
    range: Range | null;
    entries: TransitionEntry[];
}

function readTransitions(context: StudioContext, targetType?: string): Promise<TransitionList> {
    if (!context.writable || context.node === undefined) throw new Error('Select a control in the current XAML design.');
    return context.read<TransitionList>(PROPERTY_TRANSITIONS, {
        owner: context.node, ...(targetType === undefined ? {} : { target_type: targetType }),
    });
}

async function chooseProperty(
    context: StudioContext, list: TransitionList, title: string,
): Promise<{ property: string; type_name: string; target_type?: string } | undefined> {
    let targetType: string | undefined;
    if (list.requiresTarget) {
        targetType = (await editText('Registered XAML target type for this transition resource', 'Border',
            value => value.trim() ? undefined : 'Enter a registered target type.'))?.trim();
        if (targetType === undefined) return;
        list = await readTransitions(context, targetType);
    }
    const selected = await choose(title, (list.properties ?? []).map(property => ({
        label: property.label, description: property.typeName, property,
    })));
    if (selected) return {
        property: selected.property.property, type_name: selected.property.typeName,
        ...(targetType === undefined ? {} : { target_type: targetType }),
    };
}

function entryActions(context: StudioContext, list: TransitionList, entry: TransitionEntry, index: number): StudioItem[] {
    const id = `transition:${entry.id}`;
    const actions = [action(`${id}:source`, 'Reveal transition in source', () => context.revealRange(entry.range), 'go-to-file')];
    if (!list.canEdit) return actions;
    const update = (timing: object) => context.edit(EDIT_TRANSITION, {
        action: 'timing', owner: list.owner, transition: entry.id, timing,
    });
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
    const edit = (action: string, args: object = {}) => context.edit(EDIT_TRANSITION, {
        action, owner: list.owner, transition: entry.id, ...args,
    });
    if (entry.canRetarget && (list.requiresTarget || list.properties?.length)) {
        actions.push(action(`${id}:retarget`, 'Retarget transition', async () => {
            const property = await chooseProperty(context, list, 'Retarget transition');
            if (property) await edit('retarget', property);
        }, 'target'));
    }
    if (index > 0) actions.push(action(`${id}:up`, 'Move transition earlier',
        () => edit('move', { index: index - 1 }), 'arrow-up'));
    if (index + 1 < list.entries.length) actions.push(action(`${id}:down`, 'Move transition later',
        () => edit('move', { index: index + 1 }), 'arrow-down'));
    actions.push(action(`${id}:remove`, 'Remove transition', () => edit('remove'), 'trash'));
    return actions;
}

export async function propertyTransitionItems(context: StudioContext): Promise<StudioItem[]> {
    const list = await readTransitions(context);
    const items: StudioItem[] = list.entries.map((entry, index) => ({
        id: `transition:${entry.id}`, label: entry.property ?? entry.typeName,
        description: entry.durationSeconds === null ? entry.typeName : `${entry.durationSeconds} s`,
        icon: 'watch', children: async () => entryActions(context, list, entry, index),
    }));
    if (list.canAdd || (list.requiresTarget && list.canEditCollection)) {
        items.push(action('transitions:add', 'Add property transition', async () => {
            const property = await chooseProperty(context, list, 'Add property transition');
            if (property) await context.edit(EDIT_TRANSITION, { action: 'add', owner: list.owner, ...property });
        }, 'add'));
    }
    if (!items.length) {
        const range = list.range;
        return range ? [action('transitions:source', 'Reveal transition collection', () => context.revealRange(range), 'go-to-file')]
            : [{ id: 'transitions:empty', label: 'No authored transitions on this control' }];
    }
    return items;
}

export async function choosePropertyTransition(context: StudioContext): Promise<void> {
    const items = await propertyTransitionItems(context);
    const selected = await choose('Property transitions', items);
    if (!selected) return;
    if (selected.run) await selected.run();
    else if (selected.children) {
        const field = await choose(selected.label, await selected.children());
        await field?.run?.();
    }
}

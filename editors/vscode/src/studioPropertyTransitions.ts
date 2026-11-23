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
    range: Range;
}

interface TransitionList {
    owner: number;
    canEdit: boolean;
    range: Range | null;
    entries: TransitionEntry[];
}

async function readTransitions(context: StudioContext): Promise<TransitionList> {
    if (!context.writable || context.node === undefined) throw new Error('Select a control in the current XAML design.');
    return await context.read<TransitionList>(PROPERTY_TRANSITIONS, { owner: context.node });
}

function timingActions(context: StudioContext, list: TransitionList, entry: TransitionEntry): StudioItem[] {
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
    return actions;
}

export async function propertyTransitionItems(context: StudioContext): Promise<StudioItem[]> {
    const list = await readTransitions(context);
    if (!list.entries.length) {
        const range = list.range;
        return range ? [action('transitions:source', 'Reveal transition collection', () => context.revealRange(range), 'go-to-file')]
            : [{ id: 'transitions:empty', label: 'No authored transitions on this control' }];
    }
    return list.entries.map(entry => ({
        id: `transition:${entry.id}`, label: entry.property ?? entry.typeName,
        description: entry.durationSeconds === null ? entry.typeName : `${entry.durationSeconds} s`,
        icon: 'watch', children: async () => timingActions(context, list, entry),
    }));
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

import { action, choose, editText, editTime, type StudioContext, type StudioItem } from './studioContext';

interface Setter { property: string; value: string; target: string }
interface State { name: string; setters: Setter[] }
interface Transition { from: string | null; to: string; duration: number; easing: unknown }
interface StateGroup { owner: number; name: string; states: State[]; transitions: Transition[] }

export async function stateItems(context: StudioContext): Promise<StudioItem[]> {
    const { groups } = await context.read<{ groups: StateGroup[] }>('designer_states');
    const update = (model: StateGroup) => context.edit('designer_state', { action: 'upsert', model });
    const rows: StudioItem[] = groups.map(group => ({
        id: groupId(group), label: group.name, description: `${group.states.length} states`,
        children: async () => groupItems(context, group, update),
    }));
    rows.unshift(action('states:clear', 'Clear state preview', () => context.present('designer_state', { action: 'clear_preview' }), 'clear-all'));
    const owner = context.node;
    if (context.writable && owner !== undefined) rows.unshift(action('states:create', 'Add state group…', async () => {
        const name = await editText('State group name');
        if (!name) return;
        if (groups.some(group => group.owner === owner && group.name === name)) throw new Error('This control already has a group with that name.');
        const first = await editText('Initial state name', 'Normal');
        if (first) await update({ owner, name, states: [{ name: first, setters: [] }], transitions: [] });
    }, 'add'));
    return rows;
}

function groupId(group: StateGroup): string { return `states:${group.owner}:${group.name}`; }

function groupItems(context: StudioContext, group: StateGroup, update: (group: StateGroup) => Promise<void>): StudioItem[] {
    const id = groupId(group);
    const rows = group.states.map(state => action(`${id}:state:${state.name}`, state.name, async () => {
        const preview = () => context.present('designer_state', { action: 'preview', owner: group.owner, group: group.name, state: state.name });
        if (!context.writable) return preview();
        const selected = await choose(state.name, [
            { label: 'Preview state', run: preview },
            { label: 'Add or edit a state setter…', run: async () => {
                const setters = await authorSetter(state);
                if (setters) await update({ ...group, states: group.states.map(item => item === state ? { ...item, setters } : item) });
            } },
        ]);
        await selected?.run();
    }, 'symbol-enum-member'));
    if (!context.writable) return rows;
    rows.unshift(action(`${id}:add`, 'Add state…', async () => {
        const name = await editText('State name');
        if (!name) return;
        if (group.states.some(state => state.name === name)) throw new Error('This group already has a state with that name.');
        await update({ ...group, states: [...group.states, { name, setters: [] }] });
    }, 'add'));
    if (group.states.length) rows.push(action(`${id}:transition`, 'Add or edit transition…', async () => {
        const transitions = await authorTransition(group);
        if (transitions) await update({ ...group, transitions });
    }, 'arrow-swap'));
    rows.push(action(`${id}:remove`, 'Remove state group', () => context.edit('designer_state', {
        action: 'remove', owner: group.owner, group: group.name,
    }), 'trash'));
    return rows;
}

async function authorSetter(state: State): Promise<Setter[] | undefined> {
    const property = await editText('Property name');
    if (!property) return;
    const target = await editText('Named target. Leave empty for the owner.', '');
    if (target === undefined) return;
    const existing = state.setters.find(setter => setter.property === property && setter.target === target);
    const value = await editText('State value', existing?.value);
    if (value !== undefined) return [...state.setters.filter(setter => setter !== existing), { property, value, target }];
}

async function authorTransition(group: StateGroup): Promise<Transition[] | undefined> {
    const names = group.states.map(state => ({ label: state.name, name: state.name }));
    const from = await choose('Transition from', [{ label: 'Any state', name: null }, ...names]);
    if (!from) return;
    const to = await choose('Transition to', names.filter(state => state.name !== from.name));
    if (!to) return;
    const existing = group.transitions.find(transition => transition.from === from.name && transition.to === to.name);
    const duration = await editTime('Transition duration in seconds', 600, existing?.duration ?? .2);
    if (duration === undefined) return;
    const easing = existing?.easing ?? { kind: 'Linear' };
    return [...group.transitions.filter(transition => transition !== existing), { from: from.name, to: to.name, duration, easing }];
}

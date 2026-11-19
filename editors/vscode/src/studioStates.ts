import { action, choose, editText, editTime, type StudioContext, type StudioItem } from './studioContext';
import type { StateTransitionRoute } from './stateTransitionProtocol';
import type { Range } from './protocol';

interface Setter { property: string; value: string; target: string }
interface State { name: string; setters: Setter[] }
type Easing = { kind: 'Linear' } | { kind: 'Named'; value: string }
    | { kind: 'CubicBezier'; value: { x1: number; y1: number; x2: number; y2: number } }
    | { kind: 'Spring'; value: { mass: number; stiffness: number; damping: number; initial_velocity: number } };
interface Transition { from: string | null; to: string; duration: number; easing: Easing }
interface StateGroup { owner: number; name: string; states: State[]; transitions: Transition[] }
interface StateSource { owner: number; group: string; range: Range; transitions: Array<Pick<Transition, 'from' | 'to'> & { range: Range }> }

type OpenTransition = (context: StudioContext, route: StateTransitionRoute) => Promise<void>;

export async function stateItems(context: StudioContext, openTransition: OpenTransition): Promise<StudioItem[]> {
    const { groups, sources } = await context.read<{ groups: StateGroup[]; sources?: StateSource[] }>('designer_states');
    const update = (model: StateGroup) => context.edit('designer_state', { action: 'upsert', model });
    const rows: StudioItem[] = groups.map(group => ({
        id: groupId(group), label: group.name, description: `${group.states.length} states`,
        children: async () => groupItems(context, group, update, openTransition,
            sources?.find(source => source.owner === group.owner && source.group === group.name)),
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

function groupItems(context: StudioContext, group: StateGroup, update: (group: StateGroup) => Promise<void>,
    openTransition: OpenTransition, source?: StateSource): StudioItem[] {
    const id = groupId(group);
    const mapped = context.panel.supportsRustStates && source;
    const rows = group.states.map(state => action(`${id}:state:${state.name}`, state.name, async () => {
        const preview = () => context.present('designer_state', { action: 'preview', owner: group.owner, group: group.name, state: state.name });
        if (!context.writable && (!mapped || !state.setters.length)) return preview();
        const selected = await choose(state.name, [
            { label: 'Preview state', run: preview },
            { label: mapped ? 'Edit a state value…' : 'Add or edit a state setter…', run: async () => {
                const setters = await (mapped ? editMappedSetter(state) : authorSetter(state));
                if (setters) await update({ ...group, states: group.states.map(item => item === state ? { ...item, setters } : item) });
            } },
        ]);
        await selected?.run();
    }, 'symbol-enum-member'));
    if (group.transitions.length && context.panel.supportsStateTransitionPreview) {
        rows.unshift(action(`${id}:preview-transition`, 'Preview transition…', async () => {
            const route = await chooseTransition(group);
            if (route) await openTransition(context, route);
        }, 'play'));
    }
    if (mapped) {
        rows.push(action(`${id}:source`, 'Reveal Rust declaration', () => context.revealRange(mapped.range), 'code'));
        if (group.transitions.length) rows.push(action(`${id}:transition`, 'Edit transition…', async () => {
            const route = await pickTransition(group);
            if (!route) return;
            const duration = await editTime('Transition duration in seconds', 600, route.duration);
            if (duration === undefined) return;
            const value = await editText('Transition easing', easingText(route.easing));
            if (value !== undefined) await update({ ...group, transitions: group.transitions.map(item =>
                item === route ? { ...item, duration, easing: parseMappedEasing(value) } : item) });
        }, 'settings-gear'));
        if (mapped.transitions.length) rows.push(action(`${id}:route-source`, 'Reveal transition source…', async () => {
            const route = await pickTransition(group);
            const range = route && mapped.transitions.find(item => item.from === route.from && item.to === route.to)?.range;
            if (range) await context.revealRange(range);
        }, 'go-to-file'));
    }
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

async function chooseTransition(group: StateGroup): Promise<StateTransitionRoute | undefined> {
    const route = await pickTransition(group);
    if (!route) return;
    const { from, to } = route;
    if (from !== null) return { owner: group.owner, group: group.name, from, to };
    const directional = new Set(group.transitions.filter(transition => transition.to === to).map(transition => transition.from));
    const initial = await choose('Choose the starting state', [{ label: 'Base values', name: null },
        ...group.states.filter(state => state.name !== to && !directional.has(state.name)).map(state => ({ label: state.name, name: state.name }))]);
    if (initial) return { owner: group.owner, group: group.name, from: initial.name, to };
}

async function pickTransition(group: StateGroup): Promise<Transition | undefined> {
    const choice = await choose('State transitions', group.transitions.map(transition => ({
        label: `${transition.from ?? 'Any state'} → ${transition.to}`,
        description: `${transition.duration} s`,
        transition,
    })));
    return choice?.transition;
}

async function editMappedSetter(state: State): Promise<Setter[] | undefined> {
    const selection = await choose(state.name, state.setters.map(setter => ({
        label: `${setter.target || 'Owner'}.${setter.property}`, description: setter.value, setter,
    })));
    if (!selection) return;
    const value = await editText(selection.label, selection.setter.value);
    if (value !== undefined) return state.setters.map(setter => setter === selection.setter ? { ...setter, value } : setter);
}

function easingText(easing: Easing): string {
    if (easing.kind === 'Named') return easing.value;
    if (easing.kind === 'CubicBezier') {
        const { x1, y1, x2, y2 } = easing.value;
        return `${x1}, ${y1}, ${x2}, ${y2}`;
    }
    return 'LinearEasing';
}

function parseMappedEasing(text: string): Easing {
    const name = text.trim();
    if (!name || name === 'LinearEasing') return { kind: 'Linear' };
    if (!name.includes(',')) return { kind: 'Named', value: name };
    const fields = name.split(',').map(field => field.trim());
    if (fields.length !== 4 || fields.some(field => !field || !Number.isFinite(Number(field)))) {
        throw new Error('Enter a native easing name or four Bézier coordinates.');
    }
    const [x1, y1, x2, y2] = fields.map(Number);
    return { kind: 'CubicBezier', value: { x1, y1, x2, y2 } };
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

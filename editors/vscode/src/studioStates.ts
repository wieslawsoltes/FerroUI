import { action, choose, editText, editTime, type StudioContext, type StudioItem } from './studioContext';
import { BEHAVIOR_TOOL, pickBehaviorEvent } from './behaviorEditor';
import { addState, authorSetter, authorTransition, easingText, editMappedSetter, newStateGroup,
    parseMappedEasing, pickSetter, pickTransition, removeState } from './stateAuthoringPrompts';
import type { StateSetter as Setter, StateTransition as Transition, VisualStateGroup as StateGroup } from './stateGroupProtocol';
import type { StateTransitionRoute } from './stateTransitionProtocol';
import type { Range } from './protocol';
import type { BehaviorAction } from './studioBehaviors';

interface StateSource { owner: number; group: string; range: Range; transitions: Array<Pick<Transition, 'from' | 'to'> & { range: Range }> }

type OpenTransition = (context: StudioContext, route: StateTransitionRoute) => Promise<void>;

export async function stateItems(context: StudioContext, openTransition: OpenTransition): Promise<StudioItem[]> {
    const { groups, sources, creatableOwners } = await context.read<{
        groups: StateGroup[]; sources?: StateSource[]; creatableOwners?: number[];
    }>('designer_states');
    const update = (model: StateGroup) => context.edit('designer_state', { action: 'upsert', model });
    const rows: StudioItem[] = groups.map(group => ({
        id: groupId(group), label: group.name, description: `${group.states.length} states`,
        children: async () => groupItems(context, group, update, openTransition,
            sources?.find(source => source.owner === group.owner && source.group === group.name)),
    }));
    rows.unshift(action('states:clear', 'Clear state preview', () => context.present('designer_state', { action: 'clear_preview' }), 'clear-all'));
    const owner = context.node;
    const creatable = owner !== undefined && (context.writable
        || (context.panel.supportsRustStateStructure && creatableOwners?.includes(owner)));
    if (creatable) rows.unshift(action('states:create', 'Add state group…', async () => {
        const model = await newStateGroup(owner, groups);
        if (model) await update(model);
    }, 'add'));
    return rows;
}

function groupId(group: StateGroup): string { return `states:${group.owner}:${group.name}`; }

function groupItems(context: StudioContext, group: StateGroup, update: (group: StateGroup) => Promise<void>,
    openTransition: OpenTransition, source?: StateSource): StudioItem[] {
    const id = groupId(group);
    const mapped = context.panel.supportsRustStates && source;
    const writable = context.writable || (mapped && context.panel.supportsRustStateStructure);
    const rows = group.states.map(state => action(`${id}:state:${state.name}`, state.name, async () => {
        const preview = () => context.present('designer_state', { action: 'preview', owner: group.owner, group: group.name, state: state.name });
        if (!writable && (!mapped || !state.setters.length)) return preview();
        const updateSetters = (setters: Setter[]) => update({ ...group, states: group.states.map(item =>
            item === state ? { ...item, setters } : item) });
        const actions = [
            { label: 'Preview state', run: preview },
            { label: writable ? 'Add or edit a state setter…' : 'Edit a state value…', run: async () => {
                const setters = await (writable ? authorSetter(state, Boolean(mapped)) : editMappedSetter(state));
                if (setters) await updateSetters(setters);
            } },
        ];
        if (writable && state.setters.length) actions.push({ label: 'Remove setter…', run: async () => {
            const selected = await pickSetter(state);
            if (selected) await updateSetters(state.setters.filter(setter => setter !== selected.setter));
        } });
        const behaviorOwner = context.node;
        if (context.panel.supportsBehaviorAuthoring && behaviorOwner !== undefined) actions.push({
            label: 'Attach event trigger…',
            run: async () => {
                const event = await pickBehaviorEvent(context, behaviorOwner, 'Click');
                if (!event) return;
                const { actions } = await context.read<{ actions: BehaviorAction[] }>('designer_state_actions', {
                    owner: group.owner, group: group.name, from: null, to: state.name,
                });
                await context.edit(BEHAVIOR_TOOL, { action: 'upsert', model: {
                    owner: behaviorOwner, trigger: { Event: { event, source: null } }, actions,
                } });
            },
        });
        if (writable) actions.push({ label: 'Remove state', run: () => update(removeState(group, state)) });
        const selected = await choose(state.name, actions);
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
                item.from === route.from && item.to === route.to ? { ...item, duration, easing: parseMappedEasing(value) } : item) });
        }, 'settings-gear'));
        if (mapped.transitions.length) rows.push(action(`${id}:route-source`, 'Reveal transition source…', async () => {
            const route = await pickTransition(group);
            const range = route && mapped.transitions.find(item => item.from === route.from && item.to === route.to)?.range;
            if (range) await context.revealRange(range);
        }, 'go-to-file'));
    }
    if (!writable) return rows;
    rows.unshift(action(`${id}:add`, 'Add state…', async () => {
        const model = await addState(group);
        if (model) await update(model);
    }, 'add'));
    if (group.states.length) rows.push(action(`${id}:add-transition`, mapped ? 'Add transition…' : 'Add or edit transition…', async () => {
        const transitions = await authorTransition(group, Boolean(mapped));
        if (transitions) await update({ ...group, transitions });
    }, 'arrow-swap'));
    if (group.transitions.length) rows.push(action(`${id}:remove-transition`, 'Remove transition…', async () => {
        const route = await pickTransition(group);
        if (route) await update({ ...group, transitions: group.transitions.filter(item => item !== route) });
    }, 'trash'));
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

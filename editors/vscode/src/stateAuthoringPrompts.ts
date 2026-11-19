import { choose, editText, editTime } from './studioContext';
import type { StateEasing, StateSetter, StateTransition, VisualState, VisualStateGroup } from './stateGroupProtocol';

export async function newStateGroup(owner: number, groups: readonly VisualStateGroup[], title?: string): Promise<VisualStateGroup | undefined> {
    const name = (await editText('State group name', undefined, undefined, title))?.trim();
    if (!name) return;
    if (groups.some(group => group.owner === owner && group.name === name)) throw new Error('This control already has a group with that name.');
    const first = (await editText('Initial state name', 'Normal', undefined, title))?.trim();
    if (first) return { owner, name, states: [{ name: first, setters: [] }], transitions: [] };
}

export async function addState(group: VisualStateGroup, title?: string): Promise<VisualStateGroup | undefined> {
    const name = (await editText('State name', undefined, undefined, title))?.trim();
    if (!name) return;
    if (group.states.some(state => state.name === name)) throw new Error('This group already has a state with that name.');
    return { ...group, states: [...group.states, { name, setters: [] }] };
}

export function removeState(group: VisualStateGroup, state: VisualState): VisualStateGroup {
    return { ...group, states: group.states.filter(item => item !== state),
        transitions: group.transitions.filter(route => route.from !== state.name && route.to !== state.name) };
}

export async function pickTransition(group: VisualStateGroup): Promise<StateTransition | undefined> {
    const choice = await choose('State transitions', group.transitions.map(transition => ({
        label: `${transition.from ?? 'Any state'} → ${transition.to}`, description: `${transition.duration} s`, transition,
    })));
    return choice?.transition;
}

export function pickSetter(state: VisualState) {
    return choose(state.name, state.setters.map(setter => ({
        label: `${setter.target || 'Owner'}.${setter.property}`, description: setter.value, setter,
    })));
}

export async function editMappedSetter(state: VisualState, title?: string): Promise<StateSetter[] | undefined> {
    const selection = await pickSetter(state);
    if (!selection) return;
    const value = await editText(selection.label, selection.setter.value, undefined, title);
    if (value !== undefined) return state.setters.map(setter => setter === selection.setter ? { ...setter, value } : setter);
}

export function easingText(easing: StateEasing): string {
    if (easing.kind === 'Named') return easing.value;
    if (easing.kind === 'CubicBezier') {
        const { x1, y1, x2, y2 } = easing.value;
        return `${x1}, ${y1}, ${x2}, ${y2}`;
    }
    if (easing.kind === 'Spring') {
        const { mass, stiffness, damping, initial_velocity } = easing.value;
        return `Spring(${mass},${stiffness},${damping},${initial_velocity})`;
    }
    return 'LinearEasing';
}

export function parseMappedEasing(text: string): StateEasing {
    const name = text.trim();
    if (!name || name === 'LinearEasing') return { kind: 'Linear' };
    const spring = /^Spring\((.*)\)$/.exec(name);
    if (!spring && !/[\s,]/.test(name)) return { kind: 'Named', value: name };
    const fields = (spring ? spring[1] : name).split(/[\s,]+/).filter(Boolean).map(Number);
    if (fields.length !== 4 || fields.some(value => !Number.isFinite(value))) {
        throw new Error('Enter a native easing name, four Bézier coordinates, or Spring(mass,stiffness,damping,velocity).');
    }
    if (spring) {
        const [mass, stiffness, damping, initial_velocity] = fields;
        return { kind: 'Spring', value: { mass, stiffness, damping, initial_velocity } };
    }
    const [x1, y1, x2, y2] = fields;
    return { kind: 'CubicBezier', value: { x1, y1, x2, y2 } };
}

export async function authorSetter(state: VisualState, rust: boolean, title?: string): Promise<StateSetter[] | undefined> {
    if (state.setters.length) {
        const next = await choose('State setter', [
            { label: 'Add a property…', edit: false }, { label: 'Edit an existing value…', edit: true },
        ]);
        if (!next) return;
        if (next.edit) return editMappedSetter(state, title);
    }
    const property = (await editText('Property name', rust ? 'Opacity' : undefined, undefined, title))?.trim();
    if (!property) return;
    const response = await editText('Named target. Leave empty for the owner.', '', undefined, title);
    if (response === undefined) return;
    const target = response.trim();
    const existing = state.setters.find(setter => setter.property === property && setter.target === target);
    const value = await editText('State value', existing?.value, undefined, title);
    if (value !== undefined) return [...state.setters.filter(setter => setter !== existing), { property, value, target }];
}

export async function authorTransition(group: VisualStateGroup, withEasing: boolean, title?: string): Promise<StateTransition[] | undefined> {
    const names = group.states.map(state => ({ label: state.name, name: state.name }));
    const from = await choose('Transition from', [{ label: 'Any state', name: null }, ...names]);
    if (!from) return;
    const to = await choose('Transition to', names.filter(state => state.name !== from.name));
    if (!to) return;
    const existing = group.transitions.find(transition => transition.from === from.name && transition.to === to.name);
    const duration = await editTime('Transition duration in seconds', 600, existing?.duration ?? .2, title);
    if (duration === undefined) return;
    const initial = existing?.easing ?? { kind: 'Linear' } as const;
    const response = withEasing ? await editText('Transition easing', easingText(initial), undefined, title) : undefined;
    if (withEasing && response === undefined) return;
    const easing = response !== undefined ? parseMappedEasing(response) : initial;
    return [...group.transitions.filter(transition => transition !== existing), { from: from.name, to: to.name, duration, easing }];
}

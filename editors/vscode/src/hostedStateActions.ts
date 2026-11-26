import type { HostedAnimationProgram, HostedStateAuthoring } from './hostedAnimationProtocol';
import type { Range } from './protocol';
import { addState, authorSetter, authorTransition, newStateGroup, pickSetter, pickTransition, removeState } from './stateAuthoringPrompts';
import type { AuthoredStateGroup, StateGroupEdit, VisualState, VisualStateGroup } from './stateGroupProtocol';
import { choose } from './studioContext';

const title = 'FerroUI App Preview';
type Choice = { type: 'edit'; command: StateGroupEdit }
    | { type: 'preview'; program: HostedAnimationProgram }
    | { type: 'reveal'; range: Range };

function upsert(model: VisualStateGroup | undefined): Choice | undefined {
    return model && { type: 'edit', command: { action: 'upsert', model } };
}

export async function chooseHostedStateAction(
    authoring: HostedStateAuthoring, programs: readonly HostedAnimationProgram[], preferred?: number,
): Promise<Choice | undefined> {
    const active = programs.find(program => program.id === preferred)?.stateRoute;
    const groups = authoring.groups.map(group => ({
        label: `$(symbol-enum) ${group.model.name}`,
        description: authoring.owners.find(owner => owner.id === group.model.owner)?.label,
        detail: `${group.model.states.length} states · Line ${group.sourceRange.start.line + 1}`,
        group,
    }));
    const selected = groups.findIndex(({ group }) => active?.owner === group.model.owner && active.group === group.model.name);
    if (selected > 0) groups.unshift(...groups.splice(selected, 1));
    const owners = authoring.owners.filter(owner => owner.canCreate);
    const menu = [...owners.length ? [{ label: '$(add) New State Group…', group: undefined }] : [], ...groups];
    const choice = await choose('Visual States', menu);
    if (!choice) return;
    if (choice.group) {
        const { model } = choice.group;
        const matching = programs.filter(({ stateRoute }) => stateRoute?.owner === model.owner && stateRoute.group === model.name);
        return editGroup(choice.group, matching, authoring.language === 'rust');
    }
    const owner = await choose('Choose a live state owner', owners.map(owner => ({
        label: owner.label,
        description: `${owner.label === owner.typeName ? '' : `${owner.typeName} · `}Line ${owner.sourceLine}`,
        owner,
    })));
    if (owner) return upsert(await newStateGroup(owner.owner.id, authoring.groups.map(group => group.model), title));
}

async function editGroup(
    { model, sourceRange }: AuthoredStateGroup, programs: readonly HostedAnimationProgram[], rust: boolean,
): Promise<Choice | undefined> {
    const routes = programs.filter(program => !program.stateRoute?.endpoint);
    const actions = [
        { action: 'add', label: '$(add) Add State…', enabled: true },
        { action: 'state', label: '$(symbol-enum-member) State Actions…', enabled: model.states.length > 0 },
        { action: 'transition', label: '$(arrow-swap) Add or Edit Transition…', enabled: model.states.length > 0 },
        { action: 'preview', label: '$(play) Preview Transition…', enabled: routes.length > 0 },
        { action: 'removeRoute', label: '$(trash) Remove Transition…', enabled: model.transitions.length > 0 },
        { action: 'source', label: '$(go-to-file) Reveal State Group in Source', enabled: true },
        { action: 'remove', label: '$(trash) Remove State Group', enabled: true },
    ] as const;
    const picked = await choose(model.name, actions.filter(action => action.enabled));
    switch (picked?.action) {
        case 'add':
            return upsert(await addState(model, title));
        case 'state': {
            const state = await choose('Choose a state', model.states.map(state => ({
                label: state.name, description: `${state.setters.length} setters`, state,
            })));
            if (state) return editState(model, state.state,
                programs.find(({ stateRoute }) => stateRoute?.endpoint && stateRoute.to === state.state.name), rust);
            return;
        }
        case 'transition': {
            const transitions = await authorTransition(model, true, title);
            if (transitions) return upsert({ ...model, transitions });
            return;
        }
        case 'preview': {
            const route = await choose('Preview a transition', routes.map(program => ({
                label: program.name, description: `${program.duration} s`, program,
            })));
            if (route) return { type: 'preview', program: route.program };
            return;
        }
        case 'removeRoute': {
            const transition = await pickTransition(model);
            if (transition) return upsert({ ...model, transitions: model.transitions.filter(item => item !== transition) });
            return;
        }
        case 'source':
            return { type: 'reveal', range: sourceRange };
        case 'remove':
            return { type: 'edit', command: { action: 'remove', owner: model.owner, group: model.name } };
    }
}

async function editState(model: VisualStateGroup, state: VisualState, endpoint: HostedAnimationProgram | undefined, rust: boolean): Promise<Choice | undefined> {
    const actions = [
        { action: 'preview', label: '$(preview) Preview State', enabled: Boolean(endpoint) },
        { action: 'setter', label: '$(edit) Add or Edit Setter…', enabled: true },
        { action: 'removeSetter', label: '$(trash) Remove Setter…', enabled: state.setters.length > 0 },
        { action: 'remove', label: '$(trash) Remove State', enabled: true },
    ] as const;
    const picked = await choose(state.name, actions.filter(action => action.enabled));
    const update = (setters: VisualState['setters']) => upsert({
        ...model, states: model.states.map(item => item === state ? { ...state, setters } : item),
    });
    switch (picked?.action) {
        case 'preview':
            if (endpoint) return { type: 'preview', program: endpoint };
            return;
        case 'setter': {
            const setters = await authorSetter(state, rust, title);
            if (setters) return update(setters);
            return;
        }
        case 'removeSetter': {
            const setter = await pickSetter(state);
            if (setter) return update(state.setters.filter(item => item !== setter.setter));
            return;
        }
        case 'remove':
            return upsert(removeState(model, state));
    }
}

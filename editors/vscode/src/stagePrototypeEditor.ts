import type { PrototypeAction, PrototypeEdit, PrototypeGraph, PrototypeHotspot, PrototypeLinkTransition, StageInfo, StageModel, StageTarget } from './stagesProtocol';
import { editOverlayOptions } from './stagePrototypeOverlay';
import { editPrototypeTransition } from './stagePrototypeTransition';
import { choose, editText } from './studioContext';

type Hotspots = (stage: StageTarget) => Promise<PrototypeHotspot[]>;

export async function editPrototype(model: StageModel, hotspots: Hotspots, edit: PrototypeEdit):
    Promise<{ graph: PrototypeGraph; transitions?: PrototypeLinkTransition[] } | undefined> {
    const graph = structuredClone(model.manifest.prototype ?? { flows: [], links: [] });
    let transitions = model.prototypeTransitions ? structuredClone(model.manifest.prototype_transitions ?? []) : undefined;
    const index = edit.index;
    const flow = edit.kind === 'flow';
    const entries = flow ? graph.flows : graph.links;
    if (index !== undefined && !entries[index]) throw new Error('Choose an entry from the current prototype.');
    const apply = () => edit.kind === 'flow' ? editFlow(model, graph, index)
        : editLink(model, graph, hotspots, edit);
    if (index === undefined) return await apply() ? { graph, transitions } : undefined;
    const actions = [
        { label: '$(edit) Edit…', action: 'edit' },
        ...(!flow && transitions ? [{ label: '$(play) Transition…', action: 'transition' }] : []),
        { label: '$(trash) Remove', action: 'remove' },
    ];
    const selection = await choose(entries[index].name, actions);
    if (!selection) return;
    if (selection.action === 'edit') return await apply() ? { graph, transitions } : undefined;
    if (selection.action === 'transition' && transitions) {
        const previous = transitions.find(item => item.link === index);
        const transition = await editPrototypeTransition(previous?.transition);
        if (transition === undefined) return;
        transitions = transitions.filter(item => item.link !== index);
        if (transition) transitions.push({ link: index, transition });
    } else if (selection.action === 'remove') {
        entries.splice(index, 1);
        if (!flow) transitions = transitions?.filter(item => item.link !== index)
            .map(item => ({ ...item, link: item.link > index ? item.link - 1 : item.link }));
    }
    return { graph, transitions };
}

function screens(model: StageModel) {
    return model.stages.map((stage, index) => ({ label: stage.configuration.name, index, stage,
        description: `${stage.layout_size.width} × ${stage.layout_size.height}` }));
}

async function editFlow(model: StageModel, graph: PrototypeGraph, index = graph.flows.length): Promise<boolean> {
    const previous = graph.flows[index];
    const name = await editText('Flow name', previous?.name);
    if (!name) return false;
    const start = await choose('Starting screen', screens(model).map(item => ({ ...item, picked: item.index === previous?.start })));
    if (!start) return false;
    graph.flows[index] = { name, start: start.index };
    return true;
}

async function editLink(model: StageModel, graph: PrototypeGraph, hotspots: Hotspots,
    edit: Extract<PrototypeEdit, { kind: 'interaction' }>): Promise<boolean> {
    const index = edit.index ?? graph.links.length;
    const previous = graph.links[index];
    const destination = edit.destination === undefined ? undefined : model.stages[edit.destination];
    if (edit.destination !== undefined && !destination) throw new Error('Choose a current destination stage.');
    const name = await editText('Interaction name', previous?.name ?? (destination ? `Open ${destination.configuration.name}` : undefined));
    if (!name) return false;
    const available = screens(model).filter(({ stage }) => stage.status.state === 'ready');
    const source = edit.source !== undefined ? available.find(item => item.index === edit.source)
        : await choose('Source screen', available.map(item => ({ ...item, picked: item.index === previous?.source })));
    if (!source) return false;
    const targets = await hotspots(source.stage);
    if (!targets.length) throw new Error('The source screen has no selectable authored hotspot.');
    const target = await choose('Trigger control', targets.map(item => ({ ...item,
        picked: JSON.stringify(item.anchor) === JSON.stringify(previous?.trigger) })));
    if (!target) return false;
    const next = await editAction(model, source.stage, previous?.action, edit.destination);
    if (!next) return false;
    graph.links[index] = { name, source: source.index, trigger: target.anchor, action: next };
    return true;
}

async function editAction(model: StageModel, source: StageInfo, previous?: PrototypeAction, target?: number): Promise<PrototypeAction | undefined> {
    const choices = [
        { label: 'Navigate', value: 'navigate' }, { label: 'Go back', value: 'back' },
        { label: 'Open overlay', value: 'open_overlay' }, { label: 'Swap overlay', value: 'swap_overlay' },
        { label: 'Close overlay', value: 'close_overlay' },
    ] as const;
    const action = await choose('Interaction action', choices.filter(item => target === undefined || !['back', 'close_overlay'].includes(item.value))
        .map(item => ({ ...item, picked: item.value === previous?.kind })));
    if (!action) return;
    const kind = action.value;
    if (kind === 'back' || kind === 'close_overlay') return { kind };
    const options = screens(model).filter(({ stage }) => stage.id !== source.id);
    const destination = target !== undefined ? options.find(item => item.index === target)
        : await choose('Destination screen', options.map(item => ({ ...item, picked: previous !== undefined && 'stage' in previous && item.index === previous.stage })));
    if (!destination) return;
    const stage = destination.index;
    if (kind === 'navigate') return { kind, stage };
    const overlay = await editOverlayOptions(previous && 'options' in previous ? previous.options : undefined);
    if (overlay) return { kind, stage, options: overlay };
}

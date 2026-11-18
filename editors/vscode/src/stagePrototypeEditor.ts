import type { PrototypeAction, PrototypeGraph, PrototypeHotspot, StageInfo, StageModel, StageTarget } from './stagesProtocol';
import { choose, editText } from './studioContext';

type Hotspots = (stage: StageTarget) => Promise<PrototypeHotspot[]>;

export async function editPrototype(model: StageModel, hotspots: Hotspots): Promise<PrototypeGraph | undefined> {
    const graph = structuredClone(model.manifest.prototype ?? { flows: [], links: [] });
    const button = (label: string, run: () => Promise<boolean>, description?: string) => ({ label, description, run });
    const selection = await choose('FerroUI Prototype', [
        button('Add flow…', () => editFlow(model, graph)),
        button('Add interaction…', () => editLink(model, graph, hotspots)),
        ...graph.flows.map((flow, index) => button(flow.name, () => chooseEdit('Flow',
            () => editFlow(model, graph, index), () => graph.flows.splice(index, 1)), 'Flow')),
        ...graph.links.map((link, index) => button(link.name, () => chooseEdit('Interaction',
            () => editLink(model, graph, hotspots, index), () => graph.links.splice(index, 1)), 'Interaction')),
    ]);
    return await selection?.run() ? graph : undefined;
}

async function chooseEdit(label: string, edit: () => Promise<boolean>, remove: () => void): Promise<boolean> {
    const selection = await choose(label, [{ label: 'Edit…', edit: true }, { label: 'Remove', edit: false }]);
    if (!selection) return false;
    if (selection.edit) return edit();
    remove();
    return true;
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

async function editLink(model: StageModel, graph: PrototypeGraph, hotspots: Hotspots, index = graph.links.length): Promise<boolean> {
    const previous = graph.links[index];
    const name = await editText('Interaction name', previous?.name);
    if (!name) return false;
    const source = await choose('Source screen', screens(model).filter(({ stage }) => stage.status.state === 'ready')
        .map(item => ({ ...item, picked: item.index === previous?.source })));
    if (!source) return false;
    const targets = await hotspots(source.stage);
    if (!targets.length) throw new Error('The source screen has no selectable authored hotspot.');
    const target = await choose('Trigger control', targets.map(item => ({ ...item,
        picked: JSON.stringify(item.anchor) === JSON.stringify(previous?.trigger) })));
    if (!target) return false;
    const next = await editAction(model, source.stage, previous?.action);
    if (!next) return false;
    graph.links[index] = { name, source: source.index, trigger: target.anchor, action: next };
    return true;
}

async function editAction(model: StageModel, source: StageInfo, previous?: PrototypeAction): Promise<PrototypeAction | undefined> {
    const choices = [
        { label: 'Navigate', value: 'navigate' }, { label: 'Go back', value: 'back' },
        { label: 'Open overlay', value: 'open_overlay' }, { label: 'Swap overlay', value: 'swap_overlay' },
        { label: 'Close overlay', value: 'close_overlay' },
    ] as const;
    const action = await choose('Interaction action', choices.map(item => ({ ...item, picked: item.value === previous?.kind })));
    if (!action) return;
    const kind = action.value;
    if (kind === 'back' || kind === 'close_overlay') return { kind };
    const destination = await choose('Destination screen', screens(model).filter(({ stage }) => stage.id !== source.id)
        .map(item => ({ ...item, picked: previous !== undefined && 'stage' in previous && item.index === previous.stage })));
    if (!destination) return;
    const stage = destination.index;
    if (kind === 'navigate') return { kind, stage };
    const options = previous && 'options' in previous ? previous.options
        : { placement: { kind: 'center' }, dim_background: true, dismiss_on_outside: true };
    return { kind, stage, options };
}

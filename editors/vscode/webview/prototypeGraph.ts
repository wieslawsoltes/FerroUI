import type { PrototypeAction, PrototypeEdit, SourceAnchor, StageModel, StagesRequest } from '../src/stagesProtocol';
import { create, element, icon, iconButton } from './dom';
import { connectStageDrag } from './prototypeStageDrag';

export class PrototypeGraphView {
    private readonly panel = element('prototype-graph');
    private readonly flows = element('prototype-flows');
    private readonly stages = element('prototype-stages');
    private readonly addFlow = element<HTMLButtonElement>('prototype-add-flow');
    private readonly addLink = element<HTMLButtonElement>('prototype-add-link');
    private model?: StageModel;
    private drawn?: StageModel;
    private readonly thumbnails = new Map<number, HTMLImageElement>();

    constructor(private readonly send: (message: StagesRequest) => void, private readonly openStage: (id: number) => void,
        private readonly imageFor: (id: number) => string | undefined) {
        this.addFlow.onclick = () => this.edit({ kind: 'flow' });
        this.addLink.onclick = () => this.edit({ kind: 'interaction' });
    }

    show(visible: boolean): void {
        this.panel.hidden = !visible;
        if (visible) this.draw();
    }

    update(model: StageModel): void {
        this.model = model;
        if (!this.panel.hidden) this.draw();
    }

    updatePreview(id: number, src: string): void {
        const thumbnail = this.thumbnails.get(id);
        if (!thumbnail) return;
        thumbnail.src = src;
        thumbnail.hidden = false;
    }

    private edit(edit: PrototypeEdit): void {
        if (this.model) this.send({ type: 'editPrototype', epoch: this.model.epoch, edit });
    }

    private draw(): void {
        const model = this.model;
        if (!model || model === this.drawn) return;
        this.drawn = model;
        this.thumbnails.clear();
        const active = document.activeElement instanceof HTMLElement ? document.activeElement.dataset.graphFocus : undefined;
        const graph = model.manifest.prototype;
        this.addFlow.disabled = !model.stages.length;
        this.addLink.disabled = !model.stages.some(stage => stage.status.state === 'ready');
        this.flows.replaceChildren(...(graph?.flows ?? []).map((flow, index) => {
            const row = create('li');
            const play = tool('play', `Play ${flow.name}`, `flow-play-${index}`, () => {
                this.send({ type: 'prototype', command: { action: 'start', flow: flow.name } });
            });
            const description = create('span', `Starts at ${this.stageName(flow.start)}`);
            description.className = 'prototype-flow-start';
            row.append(play, create('strong', flow.name), description,
                tool('edit', `Edit ${flow.name}`, `flow-edit-${index}`, () => this.edit({ kind: 'flow', index })));
            return row;
        }));
        if (!graph?.flows.length) {
            const empty = create('li', 'Add a flow to choose a starting stage.');
            empty.className = 'prototype-flow-empty';
            this.flows.append(empty);
        }
        this.stages.replaceChildren(...model.stages.map((stage, source) => this.stageCard(model, source, stage.id)));
        if (active) this.panel.querySelector<HTMLElement>(`[data-graph-focus="${CSS.escape(active)}"]`)?.focus({ preventScroll: true });
    }

    private stageCard(model: StageModel, source: number, id: number): HTMLElement {
        const stage = model.stages[source];
        const card = create('article');
        card.className = 'prototype-node';
        card.dataset.prototypeStage = String(source);
        const header = create('header');
        const name = stage.configuration.name;
        const title = create('h3');
        const stageTitle = button('', `Show ${name} stage`, `stage-${id}`, () => this.openStage(id));
        const stageName = create('span', name);
        stageName.className = 'prototype-stage-name';
        stageTitle.append(icon('preview'), stageName);
        title.append(stageTitle);
        const add = tool('link', `Add interaction from ${name}`, `stage-add-${id}`,
            () => this.edit({ kind: 'interaction', source }));
        add.disabled = stage.status.state !== 'ready';
        header.append(title, add);
        if (!add.disabled) {
            stageTitle.title = `Show ${name} stage. Drag to connect to another stage.`;
            stageTitle.classList.add('prototype-drag-title');
            connectStageDrag(stageTitle, source, () => this.model === model,
                destination => this.edit({ kind: 'interaction', source, destination }));
        }
        const thumbnail = create('img');
        thumbnail.className = 'prototype-thumbnail';
        thumbnail.alt = `${name} native preview`;
        thumbnail.draggable = false;
        thumbnail.hidden = true;
        this.thumbnails.set(id, thumbnail);
        const src = this.imageFor(id);
        if (src) this.updatePreview(id, src);
        const board = create('div');
        board.className = 'prototype-thumbnail-board preview-surface';
        board.append(thumbnail);
        const routes = create('ul');
        routes.className = 'prototype-routes';
        model.manifest.prototype?.links.forEach((link, index) => {
            if (link.source !== source) return;
            const row = create('li');
            const trigger = (link.trigger as SourceAnchor | undefined)?.path?.at(-1);
            const target = this.actionText(link.action);
            const summary = create('span');
            summary.className = 'prototype-route-summary';
            summary.append(create('span', trigger?.name ?? trigger?.type_name ?? 'Trigger'), icon('arrow-right'), create('span', target));
            const edit = button(link.name, `Edit ${link.name}`, `interaction-${index}`, () => this.edit({ kind: 'interaction', index }));
            edit.className = 'prototype-route-title';
            edit.prepend(icon('link'));
            edit.append(summary);
            const reveal = tool('go-to-file', `Reveal ${link.name} trigger in source`, `trigger-${index}`,
                () => this.send({ type: 'revealTrigger', epoch: model.epoch, index }));
            reveal.disabled = add.disabled;
            row.append(edit, reveal);
            routes.append(row);
        });
        card.append(header, board, routes);
        return card;
    }

    private stageName(index: number): string {
        return this.model?.stages[index]?.configuration.name ?? 'Unavailable stage';
    }

    private actionText(action: PrototypeAction): string {
        switch (action.kind) {
            case 'navigate': return this.stageName(action.stage);
            case 'open_overlay': return `Open ${this.stageName(action.stage)} overlay`;
            case 'swap_overlay': return `Swap to ${this.stageName(action.stage)} overlay`;
            case 'close_overlay': return 'Close overlay';
            case 'back': return 'Go back';
        }
    }

}

function button(label: string, accessibleName: string, focus: string, action: () => void): HTMLButtonElement {
    const control = create('button', label);
    control.setAttribute('aria-label', accessibleName);
    control.title = accessibleName;
    control.dataset.graphFocus = focus;
    control.onclick = action;
    return control;
}

function tool(glyph: string, label: string, focus: string, action: () => void): HTMLButtonElement {
    const button = iconButton(glyph, label, action);
    button.dataset.graphFocus = focus;
    return button;
}

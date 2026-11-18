import type { PrototypeAction, PrototypeEdit, SourceAnchor, StageModel, StagesRequest } from '../src/stagesProtocol';
import { create, element } from './dom';

const stageMime = 'application/vnd.ferroui.prototype-stage';

export class PrototypeGraphView {
    private readonly panel = element('prototype-graph');
    private readonly flows = element('prototype-flows');
    private readonly stages = element('prototype-stages');
    private readonly addFlow = element<HTMLButtonElement>('prototype-add-flow');
    private readonly addLink = element<HTMLButtonElement>('prototype-add-link');
    private model?: StageModel;
    private drawn?: StageModel;
    private drag?: { source: number; epoch: number };

    constructor(private readonly send: (message: StagesRequest) => void, private readonly openStage: (id: number) => void) {
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

    private edit(edit: PrototypeEdit): void {
        if (this.model) this.send({ type: 'editPrototype', epoch: this.model.epoch, edit });
    }

    private draw(): void {
        const model = this.model;
        if (!model || model === this.drawn) return;
        this.drawn = model;
        this.drag = undefined;
        const active = document.activeElement instanceof HTMLElement ? document.activeElement.dataset.graphFocus : undefined;
        const graph = model.manifest.prototype;
        this.addFlow.disabled = !model.stages.length;
        this.addLink.disabled = !model.stages.some(stage => stage.status.state === 'ready');
        this.flows.replaceChildren(...(graph?.flows ?? []).map((flow, index) => {
            const row = create('li');
            const play = button(flow.name, `Play ${flow.name}`, `flow-play-${index}`, () => {
                this.send({ type: 'prototype', command: { action: 'start', flow: flow.name } });
            });
            row.append(play, create('span', `Starts at ${this.stageName(flow.start)}`),
                button('Edit', `Edit ${flow.name}`, `flow-edit-${index}`, () => this.edit({ kind: 'flow', index })));
            return row;
        }));
        this.stages.replaceChildren(...model.stages.map((stage, source) => this.stageCard(model, source, stage.id)));
        if (active) this.panel.querySelector<HTMLElement>(`[data-graph-focus="${CSS.escape(active)}"]`)?.focus({ preventScroll: true });
    }

    private stageCard(model: StageModel, source: number, id: number): HTMLElement {
        const stage = model.stages[source];
        const card = create('article');
        card.className = 'prototype-node';
        const header = create('header');
        const name = stage.configuration.name;
        const title = create('h3');
        title.append(button(name, `Show ${name} stage`, `stage-${id}`, () => this.openStage(id)));
        const add = button('Add interaction', `Add interaction from ${name}`, `stage-add-${id}`,
            () => this.edit({ kind: 'interaction', source }));
        add.disabled = stage.status.state !== 'ready';
        header.append(title, add);
        if (!add.disabled) this.dragFrom(title, source, model.epoch);
        this.dropOn(card, source, model.epoch);
        const routes = create('ul');
        routes.className = 'prototype-routes';
        model.manifest.prototype?.links.forEach((link, index) => {
            if (link.source !== source) return;
            const row = create('li');
            const trigger = (link.trigger as SourceAnchor | undefined)?.path?.at(-1);
            const summary = create('span', `${trigger?.name ?? trigger?.type_name ?? 'Trigger'} → ${this.actionText(link.action)}`);
            const edit = button(link.name, `Edit ${link.name}`, `interaction-${index}`, () => this.edit({ kind: 'interaction', index }));
            const reveal = button('Reveal trigger', `Reveal ${link.name} trigger in source`, `trigger-${index}`,
                () => this.send({ type: 'revealTrigger', epoch: model.epoch, index }));
            reveal.disabled = add.disabled;
            row.append(edit, summary, reveal);
            routes.append(row);
        });
        card.append(header, routes);
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

    private dragFrom(control: HTMLElement, source: number, epoch: number): void {
        control.draggable = true;
        control.addEventListener('dragstart', event => {
            if (!event.dataTransfer) return;
            this.drag = { source, epoch };
            event.dataTransfer.setData(stageMime, String(source));
            event.dataTransfer.effectAllowed = 'link';
        });
        control.addEventListener('dragend', () => {
            this.drag = undefined;
            for (const card of this.stages.querySelectorAll('[data-drop]')) card.removeAttribute('data-drop');
        });
    }

    private dropOn(card: HTMLElement, destination: number, epoch: number): void {
        const valid = (event: DragEvent) => Boolean(this.drag && this.drag.source !== destination && this.drag.epoch === epoch
            && this.model?.epoch === epoch && event.dataTransfer?.types.includes(stageMime));
        card.addEventListener('dragover', event => {
            if (!valid(event) || !event.dataTransfer) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = 'link';
            card.dataset.drop = 'true';
        });
        card.addEventListener('dragleave', event => {
            if (!(event.relatedTarget instanceof Node) || !card.contains(event.relatedTarget)) delete card.dataset.drop;
        });
        card.addEventListener('drop', event => {
            delete card.dataset.drop;
            if (!valid(event) || !this.drag) return;
            event.preventDefault();
            const source = this.drag.source;
            this.drag = undefined;
            this.edit({ kind: 'interaction', source, destination });
        });
    }
}

function button(label: string, accessibleName: string, focus: string, action: () => void): HTMLButtonElement {
    const control = create('button', label);
    control.setAttribute('aria-label', accessibleName);
    control.dataset.graphFocus = focus;
    control.onclick = action;
    return control;
}

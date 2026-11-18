import type { FromStages, PrototypeCommand, PrototypeScene, StageInfo, StageModel, StagesRequest, ToStages } from '../src/stagesProtocol';
import { create, element } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: StagesRequest): void; setState(state: { uri: string }): void };
const vscode = acquireVsCodeApi();
const status = element('stages-status');
const root = element('stage-cards');
const controls = element('prototype-actions');
type StageField = Extract<FromStages, { type: 'edit' }>['field'];
interface StageCard {
    stage: StageInfo;
    card: HTMLElement;
    title: HTMLElement;
    info: HTMLElement;
    image: HTMLImageElement;
    displayedEpoch?: number;
    designData: HTMLButtonElement;
    mode: HTMLButtonElement;
    pause: HTMLButtonElement;
    checks: HTMLButtonElement;
}
const visible = new Map<number, StageCard>();
let model: StageModel | undefined;
let scene: PrototypeScene | null = null;

window.addEventListener('message', async ({ data }: MessageEvent<ToStages>) => {
    if (data.type === 'status') { status.textContent = data.message; return; }
    if (data.type === 'prototype') { showPrototype(data.scene); return; }
    if (data.type === 'model') {
        model = data.model;
        status.textContent = '';
        vscode.setState({ uri: data.uri });
        const keep = new Set(data.model.stages.map(stage => stage.id));
        for (const [id, previous] of visible) {
            if (!keep.has(id)) { previous.card.remove(); visible.delete(id); }
        }
        for (const stage of data.model.stages) update(stage);
        showPrototype(model.prototype);
        if (!keep.size) status.textContent = 'Add a stage to compare this design.';
    } else {
        const current = visible.get(data.id);
        if (current?.stage.epoch !== data.epoch) return;
        const image = new Image();
        image.src = `data:image/png;base64,${data.image.png}`;
        try { await image.decode(); } catch { return; }
        if (visible.get(data.id)?.stage.epoch === data.epoch) {
            current.image.src = image.src;
            current.image.hidden = false;
            current.displayedEpoch = data.epoch;
        }
    }
});

function update(stage: StageInfo): void {
    let current = visible.get(stage.id);
    if (!current) {
        current = createStageCard(stage);
        visible.set(stage.id, current);
        root.append(current.card);
    }
    current.stage = stage;
    current.title.textContent = stage.configuration.name;
    current.image.alt = `${stage.configuration.name} preview`;
    const description = stage.status.message ?? `${Math.round(stage.layout_size.width)} × ${Math.round(stage.layout_size.height)} · ${stage.status.state.replaceAll('_', ' ')}`;
    const states = stage.configuration.states.map(({ group, state }) => `${group} · ${state}`);
    const detail = [description, ...states].join(' | ');
    if (current.info.textContent !== detail) current.info.textContent = detail;
    current.designData.textContent = stage.configuration.design_time_data ? 'Design data on' : 'Design data off';
    current.designData.setAttribute('aria-pressed', String(stage.configuration.design_time_data));
    current.mode.textContent = stage.configuration.mode === 'select' ? 'Select' : 'Interact';
    current.pause.textContent = stage.configuration.auto_update ? 'Pause' : 'Resume';
    current.checks.disabled = stage.status.state !== 'ready';
}

function createStageCard(stage: StageInfo): StageCard {
    const { id } = stage;
    const card = create('article');
    card.className = 'stage-card';
    const title = create('h2');
    const row = create('div');
    row.className = 'stage-actions';
    const info = create('p');
    info.setAttribute('role', 'status');
    const control = (label: string, field: StageField) => {
        const button = create('button', label);
        button.addEventListener('click', () => vscode.postMessage({ type: 'edit', id, epoch: current.stage.epoch, field }));
        row.append(button);
        return button;
    };
    control('Environment', 'environment');
    control('Sizing', 'layout');
    const designData = control('', 'designData');
    designData.title = 'Design-time overrides and sample data';
    control('States…', 'state');
    const mode = control('', 'mode');
    const pause = control('', 'pause');
    control('Refresh', 'refresh');
    const checks = create('button', 'Checks');
    checks.addEventListener('click', () => vscode.postMessage({ type: 'checks', id, epoch: current.stage.epoch }));
    row.append(checks);
    control('Rename', 'name');
    control('Save PNG', 'export');
    control('Remove', 'remove');
    const image = create('img');
    image.draggable = false;
    image.hidden = true;
    const current: StageCard = { stage, card, title, info, image, designData, mode, pause, checks };
    image.onclick = event => {
        const { stage } = current;
        if (stage.status.state !== 'ready' || current.displayedEpoch !== stage.epoch) return;
        if (scene && scene.current !== id) {
            if (scene.overlays.at(-1)?.options.dismiss_on_outside) sendPrototype({ action: 'closeOverlay' });
            return;
        }
        const bounds = image.getBoundingClientRect();
        vscode.postMessage({ type: 'pointer', id, epoch: stage.epoch,
            x: (event.clientX - bounds.left) / bounds.width, y: (event.clientY - bounds.top) / bounds.height });
    };
    card.append(title, row, info, image);
    return current;
}

function sendPrototype(command: PrototypeCommand): void { vscode.postMessage({ type: 'prototype', command }); }

function showPrototype(next: PrototypeScene | null): void {
    scene = next;
    controls.replaceChildren();
    controls.hidden = !scene;
    root.classList.toggle('prototype-view', Boolean(scene));
    const base = scene && visible.get(scene.screen)?.stage.layout_size;
    for (const [id, { card, stage }] of visible) {
        const layer = scene?.overlays.find(overlay => overlay.stage === id);
        card.hidden = !!scene && id !== scene.screen && !layer;
        card.classList.toggle('prototype-overlay', Boolean(layer));
        card.classList.toggle('dim-background', Boolean(layer?.options.dim_background));
        card.style.removeProperty('left');
        card.style.removeProperty('top');
        card.style.removeProperty('width');
        card.style.removeProperty('z-index');
        if (layer && base) {
            const size = stage.layout_size;
            const { kind, x, y } = layer.options.placement;
            const place = (size: number, parent: number, offset: number) => 100 * Math.max(0, Math.min(offset, parent - size)) / parent;
            const left = x ?? (kind.endsWith('left') ? 0 : (base.width - size.width) * (kind.endsWith('right') ? 1 : .5));
            const top = y ?? (kind.startsWith('top') ? 0 : (base.height - size.height) * (kind.startsWith('bottom') ? 1 : .5));
            card.style.left = `${place(size.width, base.width, left)}%`;
            card.style.top = `${place(size.height, base.height, top)}%`;
            card.style.width = `${Math.min(100, 100 * size.width / base.width)}%`;
            card.style.zIndex = String(1 + scene!.overlays.indexOf(layer));
        }
    }
    if (!scene) return;
    const button = (label: string, command: PrototypeCommand, disabled = false) => {
        const control = create('button', label);
        control.disabled = disabled;
        control.onclick = () => sendPrototype(command);
        controls.append(control);
    };
    controls.append(create('strong', scene.flow));
    button('Back', { action: 'back' }, !scene.can_go_back);
    button('Restart', { action: 'restart' });
    button('Exit Prototype', { action: 'stop' });
    model?.manifest.prototype?.links.forEach((link, index) => {
        if (model?.stages[link.source]?.id === scene?.current) button(link.name, { action: 'activate', link: index });
    });
}

element('stages-add').onclick = () => vscode.postMessage({ type: 'add' });
element('stages-save').onclick = () => vscode.postMessage({ type: 'saveManifest' });
element('stages-load').onclick = () => vscode.postMessage({ type: 'loadManifest' });
element('stages-edit-prototype').onclick = () => vscode.postMessage({ type: 'editPrototype' });
element('stages-prototype').onclick = () => vscode.postMessage({ type: 'prototype' });
vscode.postMessage({ type: 'ready' });

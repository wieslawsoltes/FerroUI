import type { PrototypeScene, StageModel, StagesRequest, ToStages } from '../src/stagesProtocol';
import { element } from './dom';
import { PrototypeGraphView } from './prototypeGraph';
import { PrototypePlayer } from './prototypePlayer';
import { StageCard } from './stageCard';
import { StageMenu } from './stageMenu';

declare function acquireVsCodeApi(): { postMessage(message: StagesRequest): void; setState(state: { uri: string }): void };
const vscode = acquireVsCodeApi();
const send = (message: StagesRequest) => vscode.postMessage(message);
const status = element('stages-status');
const root = element('stage-cards');
const tabs = element('stages-tabs');
const commands = element('stages-commands');
const boardTab = element<HTMLButtonElement>('stages-show');
const graphTab = element<HTMLButtonElement>('stages-edit-prototype');
const play = element<HTMLButtonElement>('stages-prototype');
const save = element<HTMLButtonElement>('stages-save');
const cards = new Map<number, StageCard>();
const menu = new StageMenu();
const player = new PrototypePlayer(root, send);
let model: StageModel | undefined;
let scene: PrototypeScene | null = null;
let graphOpen = false;
let inputCurrent = false;
const graph = new PrototypeGraphView(send, id => {
    switchSurface(false);
    const stage = cards.get(id);
    stage?.element.scrollIntoView({ block: 'nearest' });
    stage?.heading.focus({ preventScroll: true });
}, id => cards.get(id)?.imageSource);

window.addEventListener('message', async ({ data }: MessageEvent<ToStages>) => {
    if (data.type === 'status') { report(data.message); return; }
    if (data.type === 'prototype') { showScene(data.scene); report(data.scene?.diagnostics?.join('\n') ?? ''); return; }
    if (data.type === 'prototypeImage') { await player.artboard.present(data.frame, data.image); return; }
    if (data.type === 'prototypeFocus') { player.artboard.focused(data.frame, data.focus); return; }
    if (data.type === 'inputUnavailable') {
        inputCurrent = false;
        updateInput();
        return;
    }
    if (data.type === 'keyboardFocus') {
        cards.get(data.id)?.keyboardFocus(data.epoch, data.focus);
        return;
    }
    if (data.type === 'model') {
        model = data.model;
        inputCurrent = true;
        vscode.setState({ uri: data.uri });
        const keep = new Set(model.stages.map(stage => stage.id));
        for (const [id, previous] of cards) {
            if (!keep.has(id)) { previous.cancelContacts(); previous.element.remove(); cards.delete(id); menu.close(); }
        }
        for (const stage of model.stages) {
            const previous = cards.get(stage.id);
            if (previous) previous.update(stage);
            else {
                const card = new StageCard(stage, menu, send, () => scene);
                cards.set(stage.id, card);
                root.append(card.element);
            }
        }
        graph.update(model);
        showScene(model.prototype);
        report(scene?.diagnostics?.join('\n') || (keep.size ? '' : 'Add a stage to compare the current design or another workspace view.'));
        save.disabled = !keep.size;
        play.disabled = !model.manifest.prototype?.flows.length;
        return;
    }
    const src = await cards.get(data.id)?.present(data.epoch, data.image);
    if (src) graph.updatePreview(data.id, src);
});

window.addEventListener('blur', () => {
    for (const card of cards.values()) card.cancelContacts();
    player.artboard.cancelContacts();
});

function showScene(next: PrototypeScene | null): void {
    if (scene !== next) menu.close();
    scene = next;
    player.update(scene, model, cards);
    showSurface();
}

function updateInput(): void {
    for (const card of cards.values()) card.setInput(model?.keyboardInput === true, model?.pointerInput === true, inputCurrent && !graphOpen);
    player.artboard.setCurrent(inputCurrent);
}

function report(message: string): void {
    status.textContent = message;
    status.hidden = !message;
}

function switchSurface(open: boolean): void {
    menu.close();
    graphOpen = open;
    showSurface();
}

function showSurface(): void {
    const showGraph = graphOpen && !scene;
    root.hidden = showGraph || scene?.frame !== undefined;
    graph.show(showGraph);
    tabs.hidden = commands.hidden = Boolean(scene);
    boardTab.setAttribute('aria-pressed', String(!showGraph));
    graphTab.setAttribute('aria-pressed', String(showGraph));
    updateInput();
}

element('stages-add').onclick = () => send({ type: 'add' });
save.onclick = () => send({ type: 'saveManifest' });
element('stages-load').onclick = () => send({ type: 'loadManifest' });
boardTab.onclick = () => switchSurface(false);
graphTab.onclick = () => switchSurface(true);
play.onclick = () => send({ type: 'prototype' });
let reducedMotion: boolean | undefined;
function syncMotionPreference(): void {
    const reduced = document.body.classList.contains('vscode-reduce-motion');
    if (reduced === reducedMotion) return;
    reducedMotion = reduced;
    send({ type: 'prototypeMotionPreference', reduced });
}
new MutationObserver(syncMotionPreference).observe(document.body, { attributes: true, attributeFilter: ['class'] });
syncMotionPreference();
send({ type: 'ready' });

import type { CanvasSnapshot, DesignerViewState, FromDesigner, RuntimeRow, ToDesigner } from '../src/protocol';
import { Artboard } from './artboard';
import { CanvasDrag } from './canvasDrag';
import { CanvasInput } from './canvasInput';
import { CanvasViewport } from './canvasViewport';
import { Counterpart } from './counterpart';
import { anchorPopover, element } from './dom';
import { ViewportMenu } from './viewportMenu';

declare function acquireVsCodeApi(): { postMessage(message: FromDesigner): void; setState(state: DesignerViewState): void };
const vscode = acquireVsCodeApi();
const status = element('status');
const blocks = element<HTMLSelectElement>('blocks');
const previewTab = element('preview-tab');
const sourceTab = element('source-tab');
const zoomMenu = element('zoom-menu');
const zoomOut = element<HTMLButtonElement>('zoom-out');
const zoomIn = element<HTMLButtonElement>('zoom-in');
const zoomButton = element<HTMLButtonElement>('zoom-button');
const zoomValue = element('zoom-value');
const fitSelection = element<HTMLButtonElement>('fit-selection');
const selectionLabel = element('selection-label');
const actionsMenu = element('actions-menu');
const selectedControl = element('selected-control');
let snapshot: CanvasSnapshot | undefined;
let selected: number | undefined;
let runtime: RuntimeRow | undefined;
let token = 0;
let current = false;
let applying = false;

const viewport = new CanvasViewport((fit, scale, hasImage) => {
    const percent = `${Math.round(scale * 100)}%`;
    zoomValue.textContent = fit ? 'Fit' : percent;
    zoomButton.title = fit ? `Fit to view · ${percent}` : `Canvas zoom · ${percent}`;
    zoomButton.disabled = !hasImage;
    zoomIn.disabled = !hasImage || scale >= 8;
    zoomOut.disabled = !hasImage || scale <= 0.125;
});
const input = new CanvasInput(viewport, reveal);
const artboard = new Artboard(viewport, () => !input.panning, select);
const drag = new CanvasDrag(artboard, () => current && !applying && !input.panning, vscode.postMessage);
const size = new ViewportMenu(viewport => vscode.postMessage({ type: 'viewport', viewport }));
const counterpart = new Counterpart(apply);

anchorPopover(zoomButton, zoomMenu);
zoomOut.addEventListener('click', () => viewport.step(-1));
zoomIn.addEventListener('click', () => viewport.step(1));
for (const preset of zoomMenu.querySelectorAll<HTMLButtonElement>('[data-scale]')) {
    preset.addEventListener('click', () => { viewport.zoom(Number(preset.dataset.scale)); zoomMenu.hidePopover(); });
}
element('fit').addEventListener('click', () => { viewport.fit(); zoomMenu.hidePopover(); });
element('actual').addEventListener('click', () => { viewport.zoom(1); zoomMenu.hidePopover(); });
fitSelection.addEventListener('click', () => {
    const bounds = runtime?.bounds ?? selectedNode()?.bounds;
    if (bounds) viewport.fitSelection(bounds);
    zoomMenu.hidePopover();
});
selectionLabel.addEventListener('click', reveal);
anchorPopover(element('actions-button'), actionsMenu);
const actions = [
    ['toolbox', 'toolbox-button', { type: 'dock', view: 'toolbox' }],
    ['outline', 'outline-button', { type: 'dock', view: 'outline' }],
    ['properties', 'properties-button', { type: 'dock', view: 'properties' }],
    ['refresh', 'refresh', { type: 'refresh' }],
] satisfies [string, string, FromDesigner][];
for (const [action, id, message] of actions) {
    element(id).addEventListener('click', () => vscode.postMessage(message));
    element(`menu-${action}`).addEventListener('click', () => {
        actionsMenu.hidePopover();
        vscode.postMessage(message);
    });
}
for (const view of ['events', 'resources', 'assets'] as const) {
    element(`menu-${view}`).addEventListener('click', () => {
        actionsMenu.hidePopover();
        vscode.postMessage({ type: 'dock', view });
    });
}
blocks.addEventListener('change', () => vscode.postMessage({ type: 'block', token, index: blocks.selectedIndex }));
previewTab.addEventListener('click', () => showDesign(true));
sourceTab.addEventListener('click', () => showDesign(false));
window.addEventListener('message', event => receive(event.data as ToDesigner));
vscode.postMessage({ type: 'ready' });

function showDesign(show: boolean): void {
    element('preview').hidden = !show;
    element('canvas-tools').hidden = !show;
    element('counterpart').hidden = show;
    previewTab.setAttribute('aria-pressed', String(show));
    sourceTab.setAttribute('aria-pressed', String(!show));
}

function reveal(): void {
    if (!runtime && selected !== undefined) select(selected, true);
}

function select(id: number, openSource: boolean): void {
    if (!current || applying) return;
    selected = id;
    runtime = undefined;
    updateSelection();
    vscode.postMessage({ type: 'select', token, id, openSource });
}

function apply(source: string): void {
    if (!current || applying) return;
    applying = true;
    updateSelection();
    vscode.postMessage({ type: 'apply', token, source });
}

function selectedNode() { return snapshot?.nodes.find(node => node.id === selected); }

function updateSelection(): void {
    const node = selectedNode();
    const enabled = current && !applying;
    if (runtime) artboard.inspect(runtime.bounds);
    else artboard.select(node);
    selectionLabel.hidden = !enabled || !node?.bounds || !snapshot?.image || Boolean(runtime);
    if (node) selectedControl.textContent = node.name ? `${node.label} · ${node.name}` : node.label;
    const bounds = runtime?.bounds ?? node?.bounds;
    fitSelection.disabled = !enabled || !snapshot?.image || !bounds?.width || !bounds.height;
    counterpart.setEnabled(enabled);
    blocks.disabled = !enabled;
}

function showStatus(kind: 'busy' | 'error' | 'notice' | 'ready', text = ''): void {
    status.dataset.state = kind;
    status.textContent = text;
    status.hidden = !text;
    if (!artboard.hasFrame) element('empty-state').textContent = kind === 'error' ? 'Preview unavailable' : 'Rendering preview…';
}

function receive(message: ToDesigner): void {
    switch (message.type) {
        case 'viewState': vscode.setState(message.state); return;
        case 'drag': drag.setGesture(message.gesture); return;
        case 'placement': drag.show(message); return;
        case 'pending':
            current = false;
            drag.setFrame(undefined);
            artboard.fresh(false);
            showStatus('busy', artboard.hasFrame ? 'Updating preview…' : '');
            break;
        case 'error':
            current = false;
            drag.setFrame(undefined);
            artboard.fresh(false);
            showStatus('error', `${message.message}${artboard.hasFrame ? ' Showing the previous frame.' : ''}`);
            break;
        case 'snapshot': {
            snapshot = message.snapshot;
            token = message.frame.token;
            selected = message.selected;
            runtime = message.runtime;
            current = true;
            artboard.show(snapshot, message.viewport);
            drag.setFrame(snapshot.image ? message.frame : undefined);
            size.show(message.viewport);
            counterpart.show(snapshot.counterpart, message.sourceKey);
            blocks.replaceChildren(...snapshot.blocks.map((span, index) => new Option(`RSX · line ${span.start.line + 1}`, String(index))));
            blocks.hidden = snapshot.blocks.length <= 1;
            const active = snapshot.activeBlock?.start;
            blocks.selectedIndex = snapshot.blocks.findIndex(({ start }) => start.line === active?.line && start.character === active?.character);
            if (snapshot.renderError) {
                showStatus('error', `${snapshot.renderError}${artboard.hasFrame ? ' Showing the previous frame.' : ''}`);
            } else {
                showStatus(snapshot.notices.length ? 'notice' : 'ready', snapshot.notices.join(' · '));
            }
            break;
        }
        case 'selection':
            if (message.token === token) {
                selected = message.selected;
                runtime = message.runtime;
            }
            break;
        case 'edit':
            if (message.error) showStatus('error', message.error);
            else if (applying) counterpart.accepted();
            applying = false;
            break;
    }
    updateSelection();
}

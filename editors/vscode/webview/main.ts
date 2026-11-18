import type { CanvasAction, CanvasSnapshot, DesignerViewState, FromDesigner, RuntimeRow, ToDesigner } from '../src/protocol';
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
const preview = element('preview');
const blocks = element<HTMLSelectElement>('blocks');
const previewTab = element('preview-tab');
const sourceTab = element('source-tab');
const zoomMenu = element('zoom-menu');
const zoomOut = element<HTMLButtonElement>('zoom-out');
const zoomIn = element<HTMLButtonElement>('zoom-in');
const zoomButton = element<HTMLButtonElement>('zoom-button');
const zoomValue = element('zoom-value');
const fitSelection = element<HTMLButtonElement>('fit-selection');
const selectionLabel = element<HTMLButtonElement>('selection-label');
const actionsMenu = element('actions-menu');
const selectedControl = element('selected-control');
type SnapshotMessage = Extract<ToDesigner, { type: 'snapshot' }>;
let staged: SnapshotMessage | undefined;
const canvasActions: Extract<ToDesigner, { type: 'canvasAction' }>[] = [];
let snapshot: CanvasSnapshot | undefined;
let selected: number | undefined;
let selection: number[] = [];
let runtime: RuntimeRow | undefined;
let token = 0;
let current = false;
let applying = false;
let tab: 'preview' | 'counterpart' = 'preview';

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
    tab = show ? 'preview' : 'counterpart';
    preview.hidden = !show;
    element('canvas-tools').hidden = !show;
    element('counterpart').hidden = show;
    previewTab.setAttribute('aria-pressed', String(show));
    sourceTab.setAttribute('aria-pressed', String(!show));
}

function controlCanvas(request: number, action: CanvasAction): void {
    const { zoom, scale, pan_x, pan_y, pan_tool } = action;
    const geometry = zoom || scale !== undefined || pan_x !== undefined || pan_y !== undefined;
    if (geometry && !artboard.isCurrent) {
        vscode.postMessage({ type: 'canvasResult', request, error: 'Render an artboard before changing its zoom or pan.' });
        return;
    }
    const bounds = runtime?.bounds ?? selectedNode()?.bounds;
    if (zoom === 'fit_selection' && (!bounds?.width || !bounds.height)) {
        vscode.postMessage({ type: 'canvasResult', request, error: 'Select a control with visible bounds first.' });
        return;
    }
    if (geometry) {
        showDesign(true);
        if (zoom === 'fit') viewport.fit();
        else if (zoom === 'fit_selection') viewport.fitSelection(bounds!);
        else if (zoom === 'actual') viewport.zoom(1);
        else if (scale !== undefined) viewport.zoom(scale);
        if (pan_x !== undefined || pan_y !== undefined) viewport.panBy(pan_x ?? 0, pan_y ?? 0);
    }
    if (action.tab !== undefined) showDesign(action.tab === 'preview');
    if (pan_tool !== undefined) input.setTool(pan_tool);
    vscode.postMessage({ type: 'canvasResult', request, state: { tab, ...viewport.state, panTool: input.panTool } });
}

function reveal(): void {
    if (artboard.isCurrent && !runtime && selected !== undefined) select(selected, true);
}

function select(id: number, openSource: boolean, extend = false): void {
    if (!current || applying) return;
    vscode.postMessage({ type: 'select', token, id, openSource, extend });
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
    if (artboard.isCurrent) {
        if (runtime) artboard.inspect(runtime.bounds);
        else artboard.select(node, snapshot?.nodes.filter(item => item.id !== selected && selection.includes(item.id)));
        selectionLabel.hidden = !node?.bounds || Boolean(runtime);
        if (node) selectedControl.textContent = selection.length > 1 ? `${selection.length} controls selected`
            : node.name ? `${node.label} · ${node.name}` : node.label;
    }
    selectionLabel.disabled = !enabled || !artboard.isCurrent;
    const bounds = runtime?.bounds ?? node?.bounds;
    fitSelection.disabled = !enabled || !artboard.isCurrent || !bounds?.width || !bounds.height;
    counterpart.setEnabled(enabled);
    blocks.disabled = !enabled;
}

function showStatus(kind: 'busy' | 'error' | 'notice' | 'ready', text = ''): void {
    status.dataset.state = kind;
    if (status.textContent !== text) status.textContent = text;
    status.hidden = !text;
    if (!artboard.hasFrame) element('empty-state').textContent = kind === 'error' ? 'Preview unavailable' : 'Rendering preview…';
}

function pauseCanvas(): void {
    current = false;
    artboard.pause();
    drag.setFrame(undefined);
    updateSelection();
}

function showPreviewError(message: string): void {
    showStatus('error', `${message}${artboard.hasFrame ? ' Showing the previous frame.' : ''}`);
}

function flushCanvasActions(): void {
    for (const { request, action } of canvasActions.splice(0)) controlCanvas(request, action);
}

function stageSnapshot(message: SnapshotMessage): void {
    staged = message;
    pauseCanvas();
    preview.setAttribute('aria-busy', 'true');
    const image = message.snapshot.image;
    if (image) {
        void artboard.decode(image).then(frame => presentSnapshot(message, frame), () => presentSnapshot(message));
    } else {
        presentSnapshot(message);
    }
}

function presentSnapshot(message: SnapshotMessage, frame?: HTMLImageElement): void {
    if (staged !== message) return;
    staged = undefined;
    snapshot = message.snapshot;
    token = message.frame.token;
    selected = message.selected;
    selection = message.selection ?? [];
    runtime = message.runtime;
    current = true;
    if (frame) artboard.present(snapshot, message.viewport, frame);
    drag.setFrame(frame ? message.frame : undefined);
    size.show(message.viewport);
    counterpart.show(snapshot.counterpart, message.sourceKey);
    blocks.replaceChildren(...snapshot.blocks.map((span, index) => new Option(`RSX · line ${span.start.line + 1}`, String(index))));
    blocks.hidden = snapshot.blocks.length <= 1;
    const active = snapshot.activeBlock?.start;
    blocks.selectedIndex = snapshot.blocks.findIndex(({ start }) => start.line === active?.line && start.character === active?.character);
    const error = snapshot.renderError ?? (snapshot.image && !frame ? 'The preview image could not be decoded.' : undefined);
    if (error) showPreviewError(error);
    else showStatus(snapshot.notices.length ? 'notice' : 'ready', snapshot.notices.join(' · '));
    preview.setAttribute('aria-busy', 'false');
    updateSelection();
    flushCanvasActions();
}

function receive(message: ToDesigner): void {
    switch (message.type) {
        case 'viewState': vscode.setState(message.state); return;
        case 'canvasAction':
            if (staged) canvasActions.push(message);
            else controlCanvas(message.request, message.action);
            return;
        case 'drag': drag.setGesture(message.gesture); return;
        case 'placement': drag.show(message); return;
        case 'pending':
        case 'error':
            staged = undefined;
            pauseCanvas();
            preview.setAttribute('aria-busy', String(message.type === 'pending'));
            if (message.type === 'error') showPreviewError(message.message);
            else if (!artboard.hasFrame) showStatus('busy');
            flushCanvasActions();
            return;
        case 'snapshot': stageSnapshot(message); return;
        case 'selection':
            if (staged && message.token === staged.frame.token) {
                staged.selected = message.selected;
                staged.selection = message.selection;
                staged.runtime = message.runtime;
            } else if (current && message.token === token) {
                selected = message.selected;
                selection = message.selection ?? [];
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

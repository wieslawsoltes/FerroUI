import type { FromDesigner, Operation, Snapshot, ToDesigner } from '../src/protocol';
import { Artboard } from './artboard';
import { Counterpart } from './counterpart';
import { element } from './dom';
import { Inspector } from './inspector';
import { Outline } from './outline';

declare function acquireVsCodeApi(): { postMessage(message: FromDesigner): void };
const vscode = acquireVsCodeApi();

const status = element('status');
const blocks = element<HTMLSelectElement>('blocks');
const theme = element<HTMLSelectElement>('theme');
const width = element<HTMLInputElement>('width');
const height = element<HTMLInputElement>('height');
const previewTab = element('preview-tab');
const sourceTab = element('source-tab');
let snapshot: Snapshot | undefined;
let selected: number | undefined;
let token = 0;
let current = false;
let operation: Operation | undefined;
let renderedDark = false;
const outline = new Outline(select);
const artboard = new Artboard(select);
const inspector = new Inspector(edit);
const counterpart = new Counterpart(source => edit({ type: 'applyCounterpart', source }));

element('refresh').addEventListener('click', () => vscode.postMessage({ type: 'refresh' }));
blocks.addEventListener('change', () => vscode.postMessage({ type: 'block', token, index: blocks.selectedIndex }));
element('viewport').addEventListener('submit', event => { event.preventDefault(); updateViewport(); });
theme.addEventListener('change', updateViewport);
previewTab.addEventListener('click', () => showDesign(true));
sourceTab.addEventListener('click', () => showDesign(false));
window.addEventListener('message', event => receive(event.data as ToDesigner));
vscode.postMessage({ type: 'ready' });

function updateViewport(): void {
    if (width.validity.valid && height.validity.valid) {
        renderedDark = theme.value === 'dark';
        vscode.postMessage({ type: 'viewport', viewport: { width: width.valueAsNumber, height: height.valueAsNumber, dark: renderedDark } });
    }
}

function showDesign(show: boolean): void {
    element('preview').hidden = !show;
    element('counterpart').hidden = show;
    previewTab.setAttribute('aria-pressed', String(show));
    sourceTab.setAttribute('aria-pressed', String(!show));
}

function select(id: number): void {
    if (!current) return;
    selected = id;
    updateSelection();
    vscode.postMessage({ type: 'select', token, id });
}

function edit(next: Operation): void {
    if (!current || operation) return;
    operation = next;
    updateSelection();
    vscode.postMessage({ type: 'edit', token, operation: next });
}

function updateSelection(): void {
    const node = snapshot?.nodes.find(node => node.id === selected);
    const enabled = current && !operation;
    outline.select(selected, enabled);
    artboard.select(node);
    inspector.show(node, enabled);
    counterpart.setEnabled(enabled);
    blocks.disabled = !enabled;
}

function showStatus(text: string, state: 'ready' | 'busy' | 'error'): void {
    status.textContent = text;
    status.dataset.state = state;
}

function receive(message: ToDesigner): void {
    switch (message.type) {
        case 'pending':
            current = false;
            artboard.fresh(false);
            showStatus('Rendering the current buffer…', 'busy');
            break;
        case 'error':
            current = false;
            artboard.fresh(false);
            showStatus(`${message.message}${artboard.hasFrame ? ' Showing the previous frame.' : ''}`, 'error');
            break;
        case 'snapshot': {
            snapshot = message.snapshot;
            token = message.token;
            selected = message.selected;
            current = true;
            artboard.show(snapshot, renderedDark);
            outline.show(snapshot.nodes);
            counterpart.show(snapshot.counterpart, message.sourceKey);
            blocks.replaceChildren(...snapshot.blocks.map((span, index) => new Option(`RSX at line ${span.start.line + 1}`, String(index))));
            blocks.hidden = snapshot.blocks.length <= 1;
            blocks.selectedIndex = snapshot.blocks.findIndex(span => span.start.line === snapshot!.activeBlock?.start.line
                && span.start.character === snapshot!.activeBlock?.start.character);
            if (snapshot.renderError) {
                showStatus(`${snapshot.renderError}${artboard.hasFrame ? ' Showing the previous frame.' : ''}`, 'error');
            } else {
                showStatus(snapshot.notices.join(' · ') || 'Preview up to date', 'ready');
            }
            break;
        }
        case 'selection':
            if (message.token === token) selected = message.selected;
            break;
        case 'edit':
            if (message.error) showStatus(message.error, 'error');
            else if (operation?.type === 'applyCounterpart') counterpart.accepted();
            operation = undefined;
            break;
    }
    updateSelection();
}

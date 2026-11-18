import type { FromProperties, NodeOperation, NodeTarget, PropertiesState, PropertyAuthoring, Range, ToProperties } from '../src/protocol';
import { element } from './dom';
import { Inspector } from './inspector';

declare function acquireVsCodeApi(): { postMessage(message: FromProperties): void };
const vscode = acquireVsCodeApi();
const message = element('view-message');
const recording = element('property-recording');
const recordingTime = element('property-recording-time');
const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
const inspector = new Inspector(edit, () => vscode.postMessage({ type: 'toolbox' }), reveal, author);
let state: PropertiesState = {};
let error: string | undefined;
let request = 0;
let pending: { request: number; target: NodeTarget } | undefined;

window.addEventListener('message', event => receive(event.data as ToProperties));
vscode.postMessage({ type: 'ready' });

function submit(message: Omit<Extract<FromProperties, { type: 'edit' }>, 'request' | 'target'>
    | Omit<Extract<FromProperties, { type: 'author' }>, 'request' | 'target'>): void {
    const target = state.target;
    if (!target || pending) return;
    pending = { request: ++request, target };
    error = undefined;
    vscode.postMessage({ ...message, request, target });
    show();
}

function edit(operation: NodeOperation): void {
    if (operation.node === state.target?.node) submit({ type: 'edit', operation });
}

function author(property: string, action: PropertyAuthoring): void {
    submit({ type: 'author', action, property });
}

function reveal(range: Range): void {
    if (state.frame) vscode.postMessage({ type: 'reveal', frame: state.frame, range });
}

function receive(next: ToProperties): void {
    if (next.type === 'state') {
        if (state.session !== next.state.session || state.node?.id !== next.state.node?.id) error = undefined;
        if (pending && (pending.target.session !== next.state.session || pending.target.node !== next.state.node?.id)) pending = undefined;
        state = next.state;
    } else if (next.request === pending?.request) {
        pending = undefined;
        error = next.error;
    }
    show();
}

function show(): void {
    const status = error ?? (pending ? 'Applying edit…' : state.message);
    const text = status ?? (state.node || state.runtime ? '' : 'Select a control on the canvas or in the outline.');
    message.textContent = text;
    message.hidden = !text;
    message.classList.toggle('error', Boolean(error));
    recording.hidden = state.recordingTime === undefined || !state.node;
    if (!recording.hidden) {
        const label = `Recording at ${seconds.format(state.recordingTime!)} s`;
        if (recordingTime.textContent !== label) recordingTime.textContent = label;
    }
    inspector.show(state, Boolean(state.target && state.fields && !pending));
}

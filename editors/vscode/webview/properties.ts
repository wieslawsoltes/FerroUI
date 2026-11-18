import type { FromProperties, NodeOperation, NodeTarget, PropertiesState, ToProperties } from '../src/protocol';
import { element } from './dom';
import { Inspector } from './inspector';

declare function acquireVsCodeApi(): { postMessage(message: FromProperties): void };
const vscode = acquireVsCodeApi();
const message = element('view-message');
const inspector = new Inspector(edit, () => vscode.postMessage({ type: 'toolbox' }));
let state: PropertiesState = {};
let error: string | undefined;
let request = 0;
let pending: { request: number; target: NodeTarget } | undefined;

window.addEventListener('message', event => receive(event.data as ToProperties));
vscode.postMessage({ type: 'ready' });

function edit(operation: NodeOperation): void {
    const target = state.target;
    if (!target || operation.node !== target.node || pending) return;
    pending = { request: ++request, target };
    error = undefined;
    vscode.postMessage({ type: 'edit', request, target, operation });
    show();
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
    const text = status ?? (state.node ? '' : 'Select a control on the canvas or in the outline.');
    message.textContent = text;
    message.hidden = !text;
    message.classList.toggle('error', Boolean(error));
    inspector.show(state.session, state.node, Boolean(state.target && !pending));
}

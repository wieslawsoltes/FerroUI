import type { DetailsState, DetailsView, EventField, FromDetails, NodeOperation, Range, ResourceRow, ToDetails } from '../src/protocol';
import { create, element } from './dom';
import { actionButton } from './propertyEditor';

declare function acquireVsCodeApi(): { postMessage(message: FromDetails): void };
const vscode = acquireVsCodeApi();
const view = document.querySelector<HTMLElement>('main[data-view]')!.dataset.view as DetailsView;
const filter = element<HTMLInputElement>('details-search');
const scope = element<HTMLInputElement>('details-filtered');
const list = element('details-list');
const status = element('details-message');
const title = element('details-title');
const commandBinding = document.querySelector<HTMLButtonElement>('#edit-command-binding');
let state: DetailsState = {};
let contentKey = '';
let request = 0;
let pending: number | undefined;
let pendingMessage: string | undefined;
let error: string | undefined;

filter.addEventListener('input', applyFilter);
scope.addEventListener('change', applyFilter);
commandBinding?.addEventListener('click', () => {
    if (!state.target || pending !== undefined) return;
    pending = ++request;
    error = undefined;
    vscode.postMessage({ type: 'commandBinding', request: pending, target: state.target });
    applyFilter();
});
window.addEventListener('message', ({ data }: MessageEvent<ToDetails>) => {
    if (data.type === 'state') {
        if (data.state.frame?.session !== state.frame?.session || data.state.target?.node !== state.target?.node) error = undefined;
        state = data.state;
        render();
    } else if (data.request === pending) {
        pending = undefined;
        pendingMessage = undefined;
        error = data.error;
        applyFilter();
    }
});
vscode.postMessage({ type: 'ready' });

function render(): void {
    title.textContent = state.label ?? '';
    title.hidden = !state.label;
    const fields = view === 'events' ? state.events : state.resources;
    const key = JSON.stringify([state.frame?.session, state.target?.node, state.codeBehind, state.reviewResources, fields]);
    if (fields && key !== contentKey) {
        contentKey = key;
        list.replaceChildren(...(view === 'events'
            ? sortEvents(state.events ?? []).map(eventRow)
            : [...(state.resources ?? [])].sort((a, b) => a.name.localeCompare(b.name)).map(resourceRow)));
    } else if (!fields && !state.target) {
        contentKey = '';
        list.replaceChildren();
    }
    applyFilter();
}

function applyFilter(): void {
    const items = view === 'events' ? state.events : state.resources;
    const query = filter.value.trim().toLowerCase();
    let count = 0;
    for (const row of list.querySelectorAll<HTMLElement>('[data-search]')) {
        row.hidden = !row.dataset.search!.includes(query) || (scope.checked && row.dataset.matches !== 'true');
        if (!row.hidden) count++;
    }
    list.toggleAttribute('inert', !state.frame || !items || pending !== undefined);
    if (commandBinding) {
        commandBinding.hidden = !state.commandBinding;
        commandBinding.disabled = pending !== undefined || !state.target;
    }
    const message = error ?? pendingMessage ?? state.message
        ?? (count ? '' : items?.length ? 'No matches.' : view === 'events'
            ? state.target ? 'This control has no catalogued events.' : 'Select an authored control to browse its events.'
            : state.frame ? 'This document has no authored resource definitions.' : 'Open a XAML designer to browse resources.');
    status.textContent = message;
    status.hidden = !message;
    status.classList.toggle('error', Boolean(error));
}

function eventRow(event: EventField): HTMLElement {
    const row = create('form');
    row.className = 'detail-row';
    row.dataset.search = `${event.name} ${event.handler ?? ''} ${event.argumentType}`.toLowerCase();
    row.dataset.matches = String(event.handler !== null);
    const name = create('strong', event.name);
    const type = create('code', event.argumentType);
    type.title = event.argumentType;
    const heading = create('div');
    heading.className = 'detail-heading';
    heading.append(name, type);
    if (event.range) {
        const range = event.range;
        heading.append(actionButton('go-to-file', `Reveal ${event.name} handler in source`, () => {
            if (state.codeBehind && event.editable && event.handler) handler('reveal', event.name, range);
            else reveal(range);
        }));
    }
    const field = state.language === 'rust' ? create('textarea') : create('input');
    field.setAttribute('aria-label', `${event.name} handler`);
    field.value = event.handler ?? '';
    field.placeholder = state.language === 'rust' ? 'Handler expression' : 'Handler name';
    field.spellcheck = false;
    field.readOnly = !event.editable;
    if (field instanceof HTMLTextAreaElement) {
        field.rows = event.handler?.includes('\n') ? 3 : 1;
        field.addEventListener('keydown', key => {
            if (key.key === 'Enter' && (key.metaKey || key.ctrlKey)) { key.preventDefault(); row.requestSubmit(); }
        });
    }
    const controls = create('div');
    controls.className = 'detail-controls';
    controls.append(field);
    if (event.editable) {
        const submit = actionButton('check', `Assign ${event.name} handler`, () => {});
        submit.type = 'submit';
        controls.append(submit);
        if (state.codeBehind) controls.append(actionButton('new-file', `Create or open Rust handler for ${event.name}`, () => handler('create', event.name)));
        if (event.handler !== null) controls.append(actionButton('close', `Clear ${event.name} handler`, () => editEvent(event.name, null)));
        row.addEventListener('submit', submitEvent => {
            submitEvent.preventDefault();
            const value = field.value.trim();
            if (value && value !== event.handler) editEvent(event.name, value);
        });
    }
    row.append(heading, controls);
    return row;
}

function resourceRow(resource: ResourceRow): HTMLElement {
    const row = create('section');
    row.className = 'detail-row';
    row.dataset.search = `${resource.name} ${resource.typeName} ${resource.scope}`.toLowerCase();
    row.dataset.matches = String(resource.visible);
    const name = create('strong', resource.name);
    const heading = create('div');
    heading.className = 'detail-heading';
    heading.append(name);
    if (state.reviewResources) {
        if (resource.key) heading.append(actionButton('rename', `Rename ${resource.name} and references`, () => review('renameResource', resource)));
        heading.append(actionButton('file-code', `Extract the dictionary containing ${resource.name}`, () => review('extractDictionary', resource)));
    }
    heading.append(actionButton('go-to-file', `Reveal ${resource.name} declaration`, () => reveal(resource.range)));
    const meta = create('p', [resource.typeName, resource.scope,
        resource.usages ? `${resource.usages} ${resource.usages === 1 ? 'use' : 'uses'}` : ''].filter(Boolean).join(' · '));
    meta.className = 'detail-meta';
    row.append(heading, meta);
    if (resource.properties.length) row.append(resourceForm(resource));
    return row;
}

function resourceForm(resource: ResourceRow): HTMLElement {
    const form = create('form');
    form.className = 'detail-controls apply-resource';
    const property = create('select');
    property.setAttribute('aria-label', `Property for ${resource.name}`);
    property.append(...resource.properties.map(name => new Option(name)));
    const mode = create('select');
    mode.setAttribute('aria-label', `Reference type for ${resource.name}`);
    mode.append(new Option('Static', 'static'), new Option('Dynamic', 'dynamic'));
    const apply = create('button', 'Apply');
    apply.type = 'submit';
    apply.className = 'secondary';
    form.addEventListener('submit', event => {
        event.preventDefault();
        const target = state.target;
        if (target) edit({ type: 'applyResource', frame: target.token, node: target.node,
            resource: resource.id, property: property.value, dynamic: mode.value === 'dynamic' });
    });
    form.append(property, mode, apply);
    return form;
}

function sortEvents(events: EventField[]): EventField[] {
    const isCommon = (event: EventField) => /^(Click|TextChanged|SelectionChanged|IsCheckedChanged|ValueChanged|Tapped)$/.test(event.name);
    return [...events].sort((a, b) => Number(b.handler !== null) - Number(a.handler !== null)
        || Number(isCommon(b)) - Number(isCommon(a)) || a.name.localeCompare(b.name));
}

function editEvent(name: string, value: string | null): void {
    const target = state.target;
    if (target) edit({ type: 'setEvent', node: target.node, name, value });
}

function edit(operation: NodeOperation): void {
    if (!state.target || pending !== undefined) return;
    pending = ++request;
    pendingMessage = 'Applying edit…';
    error = undefined;
    vscode.postMessage({ type: 'edit', request, target: state.target, operation });
    applyFilter();
}

function handler(action: 'create' | 'reveal', event: string, range?: Range): void {
    if (!state.target || pending !== undefined) return;
    pending = ++request;
    error = undefined;
    vscode.postMessage({ type: 'handler', request: pending, target: state.target, action, event, range });
    applyFilter();
}

function reveal(range: Range): void {
    if (state.frame) vscode.postMessage({ type: 'reveal', frame: state.frame, range });
}

function review(type: 'extractDictionary' | 'renameResource', resource: ResourceRow): void {
    if (!state.frame || pending !== undefined) return;
    pending = ++request;
    error = undefined;
    const message = { request: pending, frame: state.frame };
    vscode.postMessage(type === 'renameResource' ? { ...message, type, range: resource.range }
        : { ...message, type, resource: resource.id });
    applyFilter();
}

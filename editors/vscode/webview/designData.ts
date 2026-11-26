import type { DesignDataState, FromDesignData, ToDesignData } from '../src/designDataProtocol';
import { element, labelButton } from './dom';

declare function acquireVsCodeApi(): { postMessage(message: FromDesignData): void };
const vscode = acquireVsCodeApi();
const form = element<HTMLElement>('data-editor');
const preview = element<HTMLSelectElement>('data-preview');
const source = element<HTMLTextAreaElement>('data-source');
const apply = element<HTMLButtonElement>('data-apply');
const discard = element<HTMLButtonElement>('data-discard');
const examples = element<HTMLButtonElement>('data-examples');
const status = element('data-status');
const error = element('data-error');

interface Draft { version: number; text: string; applied?: boolean }
const drafts = new Map<number, Draft>();
let model: DesignDataState = {};
let pending = false;
let choices: Extract<FromDesignData, { type: 'preview' }>[] = [];

preview.addEventListener('change', () => {
    const choice = choices[Number(preview.value)];
    if (choice && !pending) send(choice);
});
source.addEventListener('input', () => updateDraft(source.value));
examples.addEventListener('click', () => {
    updateDraft(JSON.stringify({ default: 'Content', fixtures: [
        { name: 'Loading', data: { Status: 'Loading…', Busy: true, Items: [] } },
        { name: 'Empty', data: { Status: 'No items', Busy: false, Items: [] } },
        { name: 'Content', data: { Status: 'Ready', Busy: false, Items: [{ Name: 'Example', Amount: 19.5 }] } },
        { name: 'Error', data: { Status: 'Could not load items', Busy: false, Items: [] } },
    ] }, null, 2));
    source.focus();
});
discard.addEventListener('click', () => {
    if (model.session !== undefined) drafts.delete(model.session);
    report('');
    display();
});
apply.addEventListener('click', () => {
    const draft = currentDraft();
    if (model.frame && draft && draft.version === model.version && !pending) {
        send({ type: 'apply', frame: model.frame, version: draft.version, source: draft.text });
    }
});
window.addEventListener('message', event => {
    const message = event.data as ToDesignData | { type: 'state'; state: DesignDataState };
    if (message.type === 'state') {
        const state = message.state;
        const changed = model.session !== state.session;
        const accepted = state.catalog && (state.frame?.token !== model.frame?.token || changed);
        if (changed || accepted) {
            pending = false;
            if (changed) report('');
        }
        model = { ...state, catalog: state.catalog ?? (changed ? undefined : model.catalog) };
    } else if (message.type === 'applied') {
        const draft = drafts.get(message.session);
        if (draft?.version === message.version) {
            if (message.resultVersion === message.version) drafts.delete(message.session);
            else draft.applied = true;
        }
        if (model.session === message.session) pending = false;
    } else {
        pending = false;
        report(message.message);
    }
    display();
});
vscode.postMessage({ type: 'ready' });

function currentDraft(): Draft | undefined {
    return model.session === undefined ? undefined : drafts.get(model.session);
}

function updateDraft(text: string): void {
    if (model.session === undefined || model.version === undefined) return;
    drafts.set(model.session, { version: currentDraft()?.version ?? model.version, text });
    report('');
    display();
}

function display(): void {
    const catalog = model.catalog;
    form.hidden = !catalog;
    status.textContent = model.message ?? '';
    status.hidden = !model.message;
    if (!catalog) return;
    let draft = currentDraft();
    if (draft?.applied && draft.version !== model.version && model.frame && model.session !== undefined) {
        drafts.delete(model.session);
        draft = undefined;
    }
    const text = draft?.text ?? (catalog.manifest ? JSON.stringify(catalog.manifest, null, 2) : '');
    if (source.value !== text) source.value = text;
    const stale = Boolean(draft && !draft.applied && draft.version !== model.version);
    if (stale) {
        status.textContent = 'The source changed while you were editing. Copy any draft text to keep, then reload from source.';
        status.hidden = false;
    }
    source.readOnly = pending;
    examples.hidden = Boolean(catalog.manifest);
    examples.disabled = pending || !model.frame || stale;
    discard.disabled = !draft || pending;
    labelButton(discard, stale ? 'Reload from source' : 'Discard draft', stale ? 'refresh' : 'discard');
    apply.disabled = !draft || !!draft.applied || stale || pending || !model.frame;
    showChoices();
}

function showChoices(): void {
    const catalog = model.catalog!;
    const values = [
        { label: 'Application values', choice: 'application' as const },
        { label: catalog.manifest ? `View default · ${catalog.manifest.default}` : 'Design values', choice: 'default' as const },
        ...(catalog.manifest?.fixtures ?? []).map(({ name }) => ({ label: name, choice: 'named' as const, name })),
    ];
    const selected = values.findIndex(value => 'name' in value && value.name === catalog.selected);
    const labels = values.map(value => value.label);
    if (JSON.stringify([...preview.options].map(option => option.text)) !== JSON.stringify(labels)) {
        preview.replaceChildren(...labels.map((label, index) => new Option(label, String(index))));
    }
    preview.value = String(catalog.enabled ? (selected === -1 ? 1 : selected) : 0);
    const frame = model.frame;
    choices = frame ? values.map(({ label: _label, ...value }) => ({ type: 'preview', frame, ...value })) : [];
    preview.disabled = pending || !frame;
}

function send(message: FromDesignData): void {
    pending = true;
    report('');
    display();
    vscode.postMessage(message);
}

function report(message: string): void {
    error.textContent = message;
    error.hidden = !message;
}

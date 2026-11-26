import { continueAppMessage } from '../src/debuggerPause';
import { runtimeFilterLength, type FromRuntime, type RuntimeFrame, type RuntimeHierarchy, type RuntimeInspection,
    type RuntimeProperty, type RuntimeViewChange, type ToRuntime } from '../src/runtimeProtocol';
import type { AppRuntime } from '../src/runtimeTargets';
import { openRuntimeTree, runtimeNodeLabel } from '../src/runtimeTree';
import { create, element, iconButton } from './dom';
import { PreviewBitmap } from './previewBitmap';
import { RuntimeKeyboard } from './runtimeKeyboard';
import { RuntimeTree } from './runtimeTree';

declare function acquireVsCodeApi(): { postMessage(message: FromRuntime): void };
const vscode = acquireVsCodeApi();
const target = element<HTMLSelectElement>('runtime-target');
const windowChoice = element<HTMLSelectElement>('runtime-window');
const hierarchy = element<HTMLSelectElement>('runtime-hierarchy');
const detail = element<HTMLSelectElement>('runtime-detail');
const refresh = element<HTMLButtonElement>('runtime-refresh');
const status = element('runtime-status');
const filter = element<HTMLInputElement>('runtime-filter');
const propertyFilter = element<HTMLInputElement>('runtime-property-filter');
const inspector = element('runtime-details');
const image = element<HTMLCanvasElement>('runtime-image');
const presenter = new PreviewBitmap(image);
const surface = image.parentElement!;
const problem = element('runtime-problem');
const main = document.querySelector<HTMLElement>('.runtime-main')!;
const form = element<HTMLFormElement>('runtime-input');
const input = element<HTMLInputElement>('runtime-input-value');
const selectMode = element('runtime-inspect');
const interactMode = element('runtime-interact');
const reveal = element('runtime-reveal');
const templates = element('runtime-templates');
let frame: RuntimeFrame | undefined;
let pending = 0;
let decoding: number | undefined;
type Inspection = Extract<ToRuntime, { type: 'inspection' }>;
let pendingInspection: Inspection | undefined;
let inspectionRevision = 0;
let selected: number | undefined;
let interaction = false;
let currentTarget: AppRuntime | undefined;
let currentGeneration: number | undefined;
let currentWindow: number | undefined;
let debuggerPaused = false;
let disclosure = openRuntimeTree;

const send = (message: FromRuntime) => {
    if (debuggerPaused && 'token' in message) showProblem(continueAppMessage);
    else vscode.postMessage(message);
};
const keyboard = new RuntimeKeyboard(surface, interactMode, send, showProblem);
element('runtime-keyboard').append(keyboard.button);
const changeView = (change: RuntimeViewChange) => {
    if (currentGeneration !== undefined) send({ type: 'presentation', generation: currentGeneration, change });
};
const inspect = (node: number, view = detail.value as RuntimeInspection) => {
    if (frame) changeView({ action: 'select', frame: frame.token, node, view });
};
const tree = new RuntimeTree(element('runtime-tree'), element('runtime-tree-status'), filter, inspect, (node, expanded) => {
    if (frame) changeView({ action: 'disclosure', frame: frame.token, node, expanded });
});
filter.maxLength = propertyFilter.maxLength = runtimeFilterLength;

function setMode(interact: boolean) {
    interaction = interact;
    surface.dataset.mode = interact ? 'interact' : 'inspect';
    selectMode.setAttribute('aria-pressed', String(!interact));
    interactMode.setAttribute('aria-pressed', String(interact));
    form.hidden = !interact || selected === undefined;
    updateKeyboard();
}

function updateKeyboard(): void {
    const scope = frame?.keyboardScope;
    keyboard.follow(scope, frame?.root.name || 'the running app', scope !== undefined && interaction && !debuggerPaused);
}

function showProblem(message: string): void {
    problem.textContent = message;
    problem.hidden = false;
}

function showProperties(data: RuntimeProperty[], canApply: boolean, canTrace: boolean) {
    const rows = data.map(property => {
        const row = create('form');
        row.dataset.property = property.name;
        const name = create('label', property.name);
        name.append(create('small', ` · ${property.priority}${property.binding ? ' · binding' : ''}`));
        const value = create('input');
        value.value = typeof property.value === 'string' ? property.value : JSON.stringify(property.value) ?? '';
        value.setAttribute('aria-label', property.name);
        const editable = !property.read_only && !property.binding && (property.value === null || typeof property.value !== 'object');
        value.readOnly = !editable;
        const apply = iconButton('check', `Set ${property.name} live`);
        apply.type = 'submit';
        apply.disabled = !editable;
        const actions = create('span');
        actions.className = 'runtime-property-actions';
        actions.append(apply);
        row.append(name, value, actions);
        if (canApply && editable) {
            const source = iconButton('edit', `Apply ${property.name} to Source`, () => {
                if (frame && selected !== undefined) send({ type: 'assignSource', token: frame.token, node: selected, property: property.name, value: value.value });
            });
            actions.append(source);
        }
        if (canTrace) {
            actions.append(iconButton('layers', `Trace ${property.name} value`, () => {
                if (frame && selected !== undefined) send({ type: 'trace', token: frame.token, node: selected, property: property.name });
            }));
        }
        row.addEventListener('submit', event => {
            event.preventDefault();
            if (editable && frame && selected !== undefined) send({ type: 'assign', token: frame.token,
                node: selected, property: property.name, value: value.value });
        });
        return row;
    });
    inspector.replaceChildren(...rows);
    filterProperties();
}

function filterProperties(): void {
    const search = propertyFilter.value.trim().toLocaleLowerCase();
    for (const row of inspector.querySelectorAll<HTMLElement>('[data-property]')) {
        row.hidden = !(row.dataset.property ?? '').toLocaleLowerCase().includes(search);
    }
}

function updateSelection(): void {
    setMode(interaction);
    tree.select(selected);
}

function clearSelection(): void {
    selected = undefined;
    pendingInspection = undefined;
    inspectionRevision = 0;
    inspector.replaceChildren();
    reveal.hidden = templates.hidden = propertyFilter.hidden = true;
    updateSelection();
}

function invalidateFrame(): void {
    pending++;
    decoding = undefined;
    frame = undefined;
    updateKeyboard();
    main.setAttribute('aria-busy', 'true');
    detail.disabled = true;
    tree.update(undefined, disclosure);
    clearSelection();
    problem.hidden = true;
}

function showInspection(message: Inspection): void {
    if (inspectionRevision === message.revision) return;
    inspectionRevision = message.revision;
    const { node, view, data, canReveal, canApply, canTrace, canInspectTemplates } = message.inspection;
    selected = node;
    detail.value = view;
    reveal.hidden = !canReveal;
    templates.hidden = !canInspectTemplates;
    propertyFilter.hidden = view !== 'properties';
    updateSelection();
    if (view === 'properties') showProperties(data as RuntimeProperty[], canApply, canTrace);
    else inspector.replaceChildren(create('pre', JSON.stringify(data, null, 2)));
}

window.addEventListener('message', async (event: MessageEvent<ToRuntime>) => {
    const message = event.data;
    if (message.type === 'state') {
        const options = message.targets.map(item => new Option(item.label, item.runtime));
        if (target.options.length !== options.length || options.some((option, i) => option.value !== target.options[i]?.value || option.label !== target.options[i]?.label)) {
            target.replaceChildren(...options);
        }
        target.disabled = options.length < 2;
        target.value = message.target ?? '';
        const windows = message.windows.map(node => new Option(runtimeNodeLabel(node), String(node.node_id)));
        if (windows.length !== windowChoice.options.length || windows.some((option, i) => option.value !== windowChoice.options[i]?.value
            || option.label !== windowChoice.options[i]?.label)) {
            windowChoice.replaceChildren(...windows);
        }
        windowChoice.value = String(message.root ?? '');
        if (debuggerPaused && !message.debuggerPaused && problem.textContent === continueAppMessage) problem.hidden = true;
        debuggerPaused = message.debuggerPaused;
        surface.toggleAttribute('data-paused', debuggerPaused);
        windowChoice.disabled = debuggerPaused || windows.length < 2;
        refresh.disabled = hierarchy.disabled = debuggerPaused;
        detail.disabled = debuggerPaused || !frame;
        if (status.textContent !== message.status) status.textContent = message.status;
        if (currentTarget !== message.target || currentWindow !== message.root || currentGeneration !== message.generation) {
            if (currentTarget !== message.target) main.hidden = true;
            currentTarget = message.target;
            currentGeneration = message.generation;
            currentWindow = message.root;
            invalidateFrame();
        }
        updateKeyboard();
    } else if (message.type === 'presentation') {
        const { hierarchy: treeKind, inspection, mode, treeFilter, propertyFilter: propertyQuery, disclosure: branches } = message.presentation;
        if (treeKind !== undefined) hierarchy.value = treeKind;
        if (inspection !== undefined) detail.value = inspection;
        if (mode !== undefined) setMode(mode === 'interact');
        const filterChanged = treeFilter !== undefined && filter.value !== treeFilter;
        if (filterChanged) filter.value = treeFilter;
        if (branches) disclosure = branches;
        if (filterChanged || branches) tree.update(frame?.tree, disclosure);
        if (propertyQuery !== undefined && propertyFilter.value !== propertyQuery) {
            propertyFilter.value = propertyQuery;
            filterProperties();
        }
    } else if (message.type === 'invalidateFrame') {
        invalidateFrame();
    } else if (message.type === 'clearSelection') {
        clearSelection();
    } else if (message.type === 'releaseKeyboard') {
        keyboard.leave();
    } else if (message.type === 'frame') {
        if (frame?.token === message.frame.token || decoding === message.frame.token) return;
        const revision = ++pending;
        decoding = message.frame.token;
        let bitmap: ImageBitmap | null;
        try { bitmap = await presenter.decode(message.frame.png); }
        catch {
            if (pending === revision) decoding = undefined;
            return;
        }
        if (pending !== revision) {
            bitmap?.close();
            return;
        }
        const changed = JSON.stringify(frame?.tree) !== JSON.stringify(message.frame.tree);
        presenter.present(message.frame.png, bitmap);
        frame = message.frame;
        updateKeyboard();
        decoding = undefined;
        main.hidden = false;
        main.setAttribute('aria-busy', 'false');
        detail.disabled = debuggerPaused;
        problem.hidden = true;
        if (changed) tree.update(frame.tree, disclosure);
        if (pendingInspection?.token === frame.token) showInspection(pendingInspection);
        pendingInspection = undefined;
    } else if (message.type === 'inspection') {
        if (frame?.token === message.token) showInspection(message);
        else if (decoding === message.token) pendingInspection = message;
    } else if (message.type === 'keyboardResult') keyboard.receive(message);
    else if (message.type === 'problem') showProblem(message.message);
});

element('runtime-start').onclick = () => send({ type: 'start' });
element('runtime-attach').onclick = () => send({ type: 'attach' });
refresh.onclick = () => changeView({ action: 'refresh' });
reveal.onclick = () => { if (frame && selected !== undefined) send({ type: 'reveal', token: frame.token, node: selected }); };
templates.onclick = () => { if (frame && selected !== undefined) send({ type: 'templates', token: frame.token, node: selected }); };
target.onchange = () => send({ type: 'target', target: target.value as AppRuntime });
windowChoice.onchange = () => changeView({ action: 'window', window: Number(windowChoice.value) });
hierarchy.onchange = () => changeView({ action: 'hierarchy', hierarchy: hierarchy.value as RuntimeHierarchy });
detail.onchange = () => { if (frame) inspect(selected ?? frame.root.node_id); };
filter.oninput = () => {
    tree.update(frame?.tree, disclosure);
    changeView({ action: 'filter', field: 'treeFilter', query: filter.value });
};
filter.onkeydown = event => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.isComposing) return;
    if (event.key === 'ArrowDown' || event.key === 'Enter') {
        tree.focus();
    } else if (event.key === 'Escape' && filter.value) {
        filter.value = '';
        filter.dispatchEvent(new Event('input'));
    } else return;
    event.preventDefault();
    event.stopPropagation();
};
propertyFilter.oninput = () => {
    filterProperties();
    changeView({ action: 'filter', field: 'propertyFilter', query: propertyFilter.value });
};
selectMode.onclick = () => changeView({ action: 'mode', mode: 'inspect' });
interactMode.onclick = () => changeView({ action: 'mode', mode: 'interact' });
image.onpointerdown = event => {
    if (event.button === 0 && keyboard.capturing) event.preventDefault();
};
image.onclick = event => {
    if (!frame) return;
    const bounds = image.getBoundingClientRect();
    send({ type: 'pointer', token: frame.token, x: (event.clientX - bounds.left) / bounds.width,
        y: (event.clientY - bounds.top) / bounds.height, interact: interaction, overlap: event.altKey });
    if (interaction) keyboard.refreshFocus();
};
image.addEventListener('wheel', event => {
    if (!interaction || !frame || debuggerPaused) return;
    event.preventDefault();
    send({ type: 'scroll', token: frame.token, node: selected ?? frame.root.node_id, x: -event.deltaX / 100, y: -event.deltaY / 100 });
}, { passive: false });
form.onsubmit = event => {
    event.preventDefault();
    if (debuggerPaused) { showProblem(continueAppMessage); return; }
    if (!frame || selected === undefined || !input.value) return;
    const target = { token: frame.token, node: selected };
    send(element<HTMLSelectElement>('runtime-input-kind').value === 'key'
        ? { type: 'key', ...target, key: input.value } : { type: 'text', ...target, text: input.value });
    input.value = '';
};
send({ type: 'ready' });

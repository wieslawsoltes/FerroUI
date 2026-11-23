import { runtimeFilterLength, type FromRuntime, type RuntimeFrame, type RuntimeHierarchy, type RuntimeInspection,
    type RuntimeNode, type RuntimeProperty, type RuntimeViewChange, type ToRuntime } from '../src/runtimeProtocol';
import type { AppRuntime } from '../src/runtimeTargets';
import { create, element, iconButton } from './dom';
import { PreviewBitmap } from './previewBitmap';

declare function acquireVsCodeApi(): { postMessage(message: FromRuntime): void };
const vscode = acquireVsCodeApi();
const target = element<HTMLSelectElement>('runtime-target');
const windowChoice = element<HTMLSelectElement>('runtime-window');
const hierarchy = element<HTMLSelectElement>('runtime-hierarchy');
const detail = element<HTMLSelectElement>('runtime-detail');
const tree = element('runtime-tree');
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

const send = (message: FromRuntime) => vscode.postMessage(message);
const changeView = (change: RuntimeViewChange) => {
    if (currentGeneration !== undefined) send({ type: 'presentation', generation: currentGeneration, change });
};
const inspect = (node: number, view = detail.value as RuntimeInspection) => {
    if (frame) changeView({ action: 'select', frame: frame.token, node, view });
};
filter.maxLength = propertyFilter.maxLength = runtimeFilterLength;

function label(node: RuntimeNode) {
    const name = node.name || node.text;
    return name ? `${node.type} · ${name}` : node.type || `Node ${node.node_id}`;
}

function setMode(interact: boolean) {
    interaction = interact;
    surface.dataset.mode = interact ? 'interact' : 'inspect';
    selectMode.setAttribute('aria-pressed', String(!interact));
    interactMode.setAttribute('aria-pressed', String(interact));
    form.hidden = !interact || selected === undefined;
}

function renderTree() {
    const search = filter.value.toLocaleLowerCase().trim();
    const items: HTMLLIElement[] = [];
    const add = (node: RuntimeNode, level: number) => {
        if (items.length >= 1200) return;
        const description = label(node);
        if (!search || description.toLocaleLowerCase().includes(search)) {
            const row = create('li');
            const button = create('button', `${'\u00a0\u00a0'.repeat(level)}${description}`);
            button.dataset.node = String(node.node_id);
            button.setAttribute('aria-pressed', String(node.node_id === selected));
            button.addEventListener('click', () => inspect(node.node_id));
            row.append(button);
            items.push(row);
        }
        (node.children ?? []).forEach(child => add(child, level + 1));
    };
    frame?.tree.forEach(node => add(node, 0));
    const focused = (document.activeElement as HTMLElement | null)?.dataset.node;
    tree.replaceChildren(...items);
    if (focused && /^\d+$/.test(focused)) tree.querySelector<HTMLElement>(`[data-node="${focused}"]`)?.focus();
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
    for (const button of tree.querySelectorAll('button')) {
        button.setAttribute('aria-pressed', String(button.dataset.node === String(selected)));
    }
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
    main.setAttribute('aria-busy', 'true');
    detail.disabled = true;
    tree.replaceChildren();
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
        const windows = message.windows.map(node => new Option(label(node), String(node.node_id)));
        if (windows.length !== windowChoice.options.length || windows.some((option, i) => option.value !== windowChoice.options[i]?.value
            || option.label !== windowChoice.options[i]?.label)) {
            windowChoice.replaceChildren(...windows);
        }
        windowChoice.value = String(message.root ?? '');
        windowChoice.disabled = windows.length < 2;
        element('runtime-status').textContent = message.status;
        if (currentTarget !== message.target || currentWindow !== message.root || currentGeneration !== message.generation) {
            if (currentTarget !== message.target) main.hidden = true;
            currentTarget = message.target;
            currentGeneration = message.generation;
            currentWindow = message.root;
            invalidateFrame();
        }
    } else if (message.type === 'presentation') {
        const { hierarchy: treeKind, inspection, mode, treeFilter, propertyFilter: propertyQuery } = message.presentation;
        if (treeKind !== undefined) hierarchy.value = treeKind;
        if (inspection !== undefined) detail.value = inspection;
        if (mode !== undefined) setMode(mode === 'interact');
        if (treeFilter !== undefined && filter.value !== treeFilter) {
            filter.value = treeFilter;
            renderTree();
        }
        if (propertyQuery !== undefined && propertyFilter.value !== propertyQuery) {
            propertyFilter.value = propertyQuery;
            filterProperties();
        }
    } else if (message.type === 'invalidateFrame') {
        invalidateFrame();
    } else if (message.type === 'clearSelection') {
        clearSelection();
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
        decoding = undefined;
        main.hidden = false;
        main.setAttribute('aria-busy', 'false');
        detail.disabled = false;
        problem.hidden = true;
        if (changed) renderTree();
        if (pendingInspection?.token === frame.token) showInspection(pendingInspection);
        pendingInspection = undefined;
    } else if (message.type === 'inspection') {
        if (frame?.token === message.token) showInspection(message);
        else if (decoding === message.token) pendingInspection = message;
    } else if (message.type === 'problem') {
        problem.textContent = message.message;
        problem.hidden = false;
    }
});

element('runtime-start').onclick = () => send({ type: 'start' });
element('runtime-attach').onclick = () => send({ type: 'attach' });
element('runtime-refresh').onclick = () => changeView({ action: 'refresh' });
reveal.onclick = () => { if (frame && selected !== undefined) send({ type: 'reveal', token: frame.token, node: selected }); };
templates.onclick = () => { if (frame && selected !== undefined) send({ type: 'templates', token: frame.token, node: selected }); };
target.onchange = () => send({ type: 'target', target: target.value as AppRuntime });
windowChoice.onchange = () => changeView({ action: 'window', window: Number(windowChoice.value) });
hierarchy.onchange = () => changeView({ action: 'hierarchy', hierarchy: hierarchy.value as RuntimeHierarchy });
detail.onchange = () => { if (frame) inspect(selected ?? frame.root.node_id); };
filter.oninput = () => {
    renderTree();
    changeView({ action: 'filter', field: 'treeFilter', query: filter.value });
};
propertyFilter.oninput = () => {
    filterProperties();
    changeView({ action: 'filter', field: 'propertyFilter', query: propertyFilter.value });
};
selectMode.onclick = () => changeView({ action: 'mode', mode: 'inspect' });
interactMode.onclick = () => changeView({ action: 'mode', mode: 'interact' });
image.onclick = event => {
    if (!frame) return;
    const bounds = image.getBoundingClientRect();
    send({ type: 'pointer', token: frame.token, x: (event.clientX - bounds.left) / bounds.width,
        y: (event.clientY - bounds.top) / bounds.height, interact: interaction, overlap: event.altKey });
};
image.addEventListener('wheel', event => {
    if (!interaction || !frame) return;
    event.preventDefault();
    send({ type: 'scroll', token: frame.token, node: selected ?? frame.root.node_id, x: -event.deltaX / 100, y: -event.deltaY / 100 });
}, { passive: false });
form.onsubmit = event => {
    event.preventDefault();
    if (!frame || selected === undefined || !input.value) return;
    const target = { token: frame.token, node: selected };
    send(element<HTMLSelectElement>('runtime-input-kind').value === 'key'
        ? { type: 'key', ...target, key: input.value } : { type: 'text', ...target, text: input.value });
    input.value = '';
};
send({ type: 'ready' });

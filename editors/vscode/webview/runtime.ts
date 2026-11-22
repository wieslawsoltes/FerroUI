import type { FromRuntime, RuntimeFrame, RuntimeHierarchy, RuntimeInspection, RuntimeNode, RuntimeProperty, ToRuntime } from '../src/runtimeProtocol';
import type { RuntimeTarget } from '../src/runtimeConnections';
import { create, element, iconButton } from './dom';

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
const image = element<HTMLImageElement>('runtime-image');
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
let selected: number | undefined;
let interaction = false;
let currentTarget: RuntimeTarget | undefined;
let currentWindow: number | undefined;

const send = (message: FromRuntime) => vscode.postMessage(message);
const inspect = (node: number, view = detail.value as RuntimeInspection) => {
    if (frame) send({ type: 'select', token: frame.token, node, view });
};

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

window.addEventListener('message', async (event: MessageEvent<ToRuntime>) => {
    const message = event.data;
    if (message.type === 'state') {
        const options = message.targets.map(item => new Option(item === 'managed' ? 'Managed application' : 'Attached application', item));
        if (target.options.length !== options.length || options.some((option, i) => option.value !== target.options[i]?.value)) {
            target.replaceChildren(...options);
        }
        target.disabled = options.length < 2;
        target.value = message.target ?? '';
        const windows = message.windows.map(node => new Option(label(node), String(node.node_id)));
        if (windows.length !== windowChoice.options.length || windows.some((option, i) => option.value !== windowChoice.options[i]?.value)) {
            windowChoice.replaceChildren(...windows);
        }
        windowChoice.value = String(message.root ?? '');
        windowChoice.disabled = windows.length < 2;
        element('runtime-status').textContent = message.status;
        if (currentTarget !== message.target || currentWindow !== message.root) {
            currentTarget = message.target;
            currentWindow = message.root;
            pending++;
            frame = undefined;
            selected = undefined;
            main.setAttribute('aria-busy', 'true');
            inspector.replaceChildren();
            reveal.hidden = true;
            templates.hidden = true;
            propertyFilter.hidden = true;
            problem.hidden = true;
            setMode(false);
        }
    } else if (message.type === 'frame') {
        const revision = ++pending;
        const decoded = new Image();
        decoded.src = `data:image/png;base64,${message.frame.png}`;
        try { await decoded.decode(); } catch { return; }
        if (pending !== revision) return;
        const changed = JSON.stringify(frame?.tree) !== JSON.stringify(message.frame.tree);
        frame = message.frame;
        image.src = decoded.src;
        main.hidden = false;
        main.setAttribute('aria-busy', 'false');
        problem.hidden = true;
        if (changed) renderTree();
    } else if (message.type === 'inspection') {
        if (frame?.token !== message.token) return;
        selected = message.node;
        detail.value = message.view;
        reveal.hidden = !message.canReveal;
        templates.hidden = !message.canInspectTemplates;
        propertyFilter.hidden = message.view !== 'properties';
        setMode(interaction);
        for (const button of tree.querySelectorAll('button')) button.setAttribute('aria-pressed', String(button.dataset.node === String(selected)));
        if (message.view === 'properties') showProperties(message.data as RuntimeProperty[], message.canApply, message.canTrace);
        else inspector.replaceChildren(create('pre', JSON.stringify(message.data, null, 2)));
    } else if (message.type === 'problem') {
        problem.textContent = message.message;
        problem.hidden = false;
    }
});

element('runtime-start').onclick = () => send({ type: 'start' });
element('runtime-attach').onclick = () => send({ type: 'attach' });
element('runtime-refresh').onclick = () => send({ type: 'refresh' });
reveal.onclick = () => { if (frame && selected !== undefined) send({ type: 'reveal', token: frame.token, node: selected }); };
templates.onclick = () => { if (frame && selected !== undefined) send({ type: 'templates', token: frame.token, node: selected }); };
target.onchange = () => send({ type: 'target', target: target.value as RuntimeTarget });
windowChoice.onchange = () => send({ type: 'window', id: Number(windowChoice.value) });
hierarchy.onchange = () => send({ type: 'hierarchy', tree: hierarchy.value as RuntimeHierarchy });
detail.onchange = () => { if (frame) inspect(selected ?? frame.root.node_id); };
filter.oninput = renderTree;
propertyFilter.oninput = filterProperties;
selectMode.onclick = () => setMode(false);
interactMode.onclick = () => setMode(true);
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

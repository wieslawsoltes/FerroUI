import type { DragGesture, FromOutline, OutlinePosition, OutlineRow, OutlineState, ToOutline } from '../src/protocol';
import { create, element } from './dom';
import { DragSource, finishDrop, matchDrag } from './drag';

declare function acquireVsCodeApi(): { postMessage(message: FromOutline): void };
const vscode = acquireVsCodeApi();
const tree = element<HTMLUListElement>('outline-tree');
const status = element('outline-message');
let state: OutlineState = { nodes: [] };
let rows = new Map<number, OutlineRow>();
let children = new Map<number | null, OutlineRow[]>();
let elements = new Map<number, HTMLLIElement>();
const collapsed = new Set<number>();
let key = '';
let structureKey = '';
let drag: DragGesture | undefined;
let marker: HTMLLIElement | undefined;
let expansion: ReturnType<typeof setTimeout> | undefined;
let hoverScroll = 0;
let error: string | undefined;

const dragSource = new DragSource(tree, event => {
    const item = rowAt(event);
    if (item?.row.canMove && state.frame && !(event.target as Element).closest('.twisty')) {
        return { frame: state.frame, origin: { type: 'node', node: item.row.id } };
    }
}, vscode.postMessage, gesture => { drag = gesture; });

tree.addEventListener('pointerdown', () => {
    if (drag) {
        vscode.postMessage({ type: 'endDrag', id: drag.id });
        drag = undefined;
        clearMarker();
    }
}, true);

window.addEventListener('message', ({ data }: MessageEvent<ToOutline>) => {
    if (data.type === 'drag') {
        drag = data.gesture;
        if (!drag) { dragSource.end(false); clearMarker(); }
        return;
    }
    if (data.type === 'state') {
        if (state.session !== data.state.session) {
            collapsed.clear();
            error = undefined;
        }
        const picked = state.selected !== data.state.selected || state.session !== data.state.session;
        state = data.state;
        const next = JSON.stringify(state.nodes);
        if (next !== key) {
            key = next;
            rows = new Map(state.nodes.map(row => [row.id, row]));
            children = new Map();
            for (const row of state.nodes) {
                const siblings = children.get(row.parent) ?? [];
                siblings.push(row);
                children.set(row.parent, siblings);
            }
        }
        if (picked) expandTo(state.selected);
        const openKey = `${state.session}:${key}:${[...collapsed].join(',')}`;
        if (structureKey !== openKey) {
            structureKey = openKey;
            render();
        }
        syncRows();
        if (picked && state.selected !== undefined && !drag) elements.get(state.selected)?.scrollIntoView({ block: 'nearest' });
    } else error = data.error;
    const text = error ?? state.message ?? (state.nodes.length ? '' : 'Open a FerroUI designer to browse its controls.');
    status.textContent = text;
    status.hidden = !text;
    status.classList.toggle('error', Boolean(error));
});

tree.addEventListener('click', event => {
    const item = rowAt(event);
    if (!item) return;
    if ((event.target as Element).closest('.twisty') && children.has(item.row.id)) {
        toggle(item.row.id);
        return;
    }
    select(item.row.id);
});
tree.addEventListener('dblclick', event => {
    const item = rowAt(event);
    if (item && !(event.target as Element).closest('.twisty')) select(item.row.id, true);
});
tree.addEventListener('keydown', event => {
    const item = rowAt(event);
    if (!item) return;
    const { id, parent, canMove } = item.row;
    const frame = state.frame;
    if (event.altKey && canMove && frame && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
        event.preventDefault();
        const above = event.key === 'ArrowUp';
        const siblings = (children.get(parent) ?? []).filter(row => row.canMove);
        const sibling = siblings[siblings.findIndex(row => row.id === id) + (above ? -1 : 1)];
        if (sibling) vscode.postMessage({ type: 'reorder', frame, node: id, anchor: sibling.id, position: above ? 'before' : 'after' });
        return;
    }
    const visible = [...elements.keys()];
    const index = visible.indexOf(id);
    let next: number | undefined;
    switch (event.key) {
        case 'ArrowDown': next = visible[index + 1]; break;
        case 'ArrowUp': next = visible[index - 1]; break;
        case 'Home': next = visible[0]; break;
        case 'End': next = visible.at(-1); break;
        case 'ArrowLeft': if (children.has(id) && !collapsed.has(id)) toggle(id); else next = parent ?? undefined; break;
        case 'ArrowRight': if (collapsed.has(id)) toggle(id); else next = children.get(id)?.[0]?.id; break;
        case 'Enter': select(id, true); break;
        case ' ': select(id); break;
        default: return;
    }
    event.preventDefault();
    if (next !== undefined) elements.get(next)?.focus();
});

tree.addEventListener('pointermove', event => {
    if (!(event.buttons & 1)) {
        if (drag) vscode.postMessage({ type: 'endDrag', id: drag.id });
        clearMarker();
        return;
    }
    const target = dropTarget(event);
    if (!drag || !target) { clearMarker(); return; }
    if (marker !== target.item || marker.dataset.drop !== target.position) {
        clearMarker();
        marker = target.item;
        marker.dataset.drop = target.position;
        if (target.position === 'inside' && collapsed.has(target.node)) {
            expansion = setTimeout(() => { collapsed.delete(target.node); render(); }, 480);
        }
    }
    cancelAnimationFrame(hoverScroll);
    const step = event.clientY < 28 ? -18 : event.clientY > innerHeight - 28 ? 18 : 0;
    if (step) hoverScroll = requestAnimationFrame(() => window.scrollBy(0, step));
});
tree.addEventListener('pointerleave', clearMarker);
tree.addEventListener('pointerup', event => {
    const target = dropTarget(event);
    if (target && event.button === 0) finishDrop(state.frame, drag, { type: 'outline', node: target.node, position: target.position }, vscode.postMessage);
    clearMarker();
});
vscode.postMessage({ type: 'ready' });

function render(focus = (document.activeElement as HTMLElement | null)?.dataset.node): void {
    elements = new Map();
    const visible: HTMLLIElement[] = [];
    function add(parent: number | null, level: number): void {
        for (const row of children.get(parent) ?? []) {
            const item = create('li');
            item.setAttribute('role', 'treeitem');
            item.setAttribute('aria-level', String(level + 1));
            item.dataset.node = String(row.id);
            item.style.setProperty('--level', String(level));
            const branch = children.has(row.id);
            const open = !collapsed.has(row.id);
            const twisty = glyph(branch ? (open ? 'chevron-down' : 'chevron-right') : '', 'twisty');
            if (branch) item.setAttribute('aria-expanded', String(open));
            item.append(twisty, glyph(row.canInsert || branch ? 'symbol-structure' : 'symbol-field'), create('span', row.label));
            if (row.name) item.append(Object.assign(create('span', row.name), { className: 'outline-name' }));
            visible.push(item);
            elements.set(row.id, item);
            if (open) add(row.id, level + 1);
        }
    }
    add(null, 0);
    tree.replaceChildren(...visible);
    syncRows();
    if (focus) elements.get(Number(focus))?.focus({ preventScroll: true });
}

function syncRows(): void {
    const tabbable = state.selected !== undefined && elements.has(state.selected) ? state.selected : elements.keys().next().value;
    for (const [id, element] of elements) {
        element.tabIndex = id === tabbable ? 0 : -1;
        element.toggleAttribute('data-movable', Boolean(state.frame && rows.get(id)?.canMove));
        element.setAttribute('aria-selected', String(state.selected === id));
    }
}

function glyph(name: string, variant = ''): HTMLSpanElement {
    const span = create('span');
    span.className = `codicon codicon-${name} ${variant}`;
    span.setAttribute('aria-hidden', 'true');
    return span;
}

function rowAt(event: Event) {
    const item = (event.target as HTMLElement).closest<HTMLLIElement>('[data-node]');
    const row = item && rows.get(Number(item.dataset.node));
    return item && row ? { item, row } : undefined;
}

function select(node: number, openSource = false): void {
    if (!state.frame) return;
    error = undefined;
    vscode.postMessage({ type: 'select', target: { ...state.frame, node }, openSource });
}

function toggle(id: number): void {
    if (!collapsed.delete(id)) collapsed.add(id);
    render(String(id));
}

function expandTo(node?: number): void {
    let row = node === undefined ? undefined : rows.get(node);
    while (row && row.parent !== null) {
        collapsed.delete(row.parent);
        row = rows.get(row.parent);
    }
}

function dropTarget(event: PointerEvent) {
    if (!matchDrag(state.frame, drag)) return undefined;
    const found = rowAt(event);
    if (!found) return undefined;
    const { row, item } = found;
    if (drag.origin.type === 'node') {
        for (let ancestor: OutlineRow | undefined = row; ancestor; ancestor = rows.get(ancestor.parent ?? -1)) {
            if (ancestor.id === drag.origin.node) return undefined;
        }
    }
    const rect = item.getBoundingClientRect();
    const fraction = (event.clientY - rect.top) / rect.height;
    const position: OutlinePosition = row.canInsert && (row.parent === null || fraction >= .25 && fraction <= .75)
        ? 'inside' : fraction < .5 ? 'before' : 'after';
    if (position !== 'inside' && !row.canMove) return undefined;
    return { node: row.id, position, item };
}

function clearMarker(): void {
    if (marker) delete marker.dataset.drop;
    marker = undefined;
    clearTimeout(expansion);
    cancelAnimationFrame(hoverScroll);
}

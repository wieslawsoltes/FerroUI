import type { DragGesture, FromOutline, OutlineMode, OutlinePosition, OutlineRow, OutlineState, ToOutline } from '../src/protocol';
import { create, element } from './dom';
import { DragSource, finishDrop, matchDrag } from './drag';

declare function acquireVsCodeApi(): {
    postMessage(message: FromOutline): void;
    getState(): { mode: OutlineMode } | undefined;
    setState(state: { mode: OutlineMode }): void;
};
const vscode = acquireVsCodeApi();
const tree = element<HTMLUListElement>('outline-tree');
const status = element('outline-message');
const mode = element<HTMLSelectElement>('outline-mode');
let state: OutlineState = { nodes: [] };
let rows = new Map<string, OutlineRow>();
let children = new Map<string | null, OutlineRow[]>();
let elements = new Map<string, HTMLLIElement>();
const collapsed = new Set<string>();
let key = '';
let structureKey = '';
let drag: DragGesture | undefined;
let marker: HTMLLIElement | undefined;
let expansion: ReturnType<typeof setTimeout> | undefined;
let hoverScroll = 0;
let error: string | undefined;

const dragSource = new DragSource(tree, event => {
    const item = rowAt(event);
    if (item?.row.canMove && item.row.source !== undefined && state.frame && !(event.target as Element).closest('.twisty')) {
        return { frame: state.frame, origin: { type: 'node', node: item.row.source } };
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
        if (state.session !== data.state.session || state.mode !== data.state.mode) {
            collapsed.clear();
            error = undefined;
        }
        const picked = state.selected !== data.state.selected || state.session !== data.state.session;
        state = data.state;
        mode.value = state.mode ?? 'source';
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
    const text = error ?? state.message ?? (state.nodes.length ? '' : state.session === undefined
        ? 'Open a FerroUI designer to browse its controls.' : 'This preview has no controls in this hierarchy.');
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
    const { id, parent, canMove, source } = item.row;
    const frame = state.frame;
    if (event.altKey && canMove && source !== undefined && frame && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
        event.preventDefault();
        const above = event.key === 'ArrowUp';
        const siblings = (children.get(parent) ?? []).filter(row => row.canMove);
        const sibling = siblings[siblings.findIndex(row => row.id === id) + (above ? -1 : 1)];
        if (sibling?.source !== undefined) vscode.postMessage({ type: 'reorder', frame, node: source, anchor: sibling.source, position: above ? 'before' : 'after' });
        return;
    }
    const visible = [...elements.keys()];
    const index = visible.indexOf(id);
    let next: string | undefined;
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
        if (target.position === 'inside' && collapsed.has(target.id)) {
            expansion = setTimeout(() => { collapsed.delete(target.id); render(); }, 480);
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
mode.addEventListener('change', () => {
    const value = mode.value as OutlineMode;
    vscode.setState({ mode: value });
    vscode.postMessage({ type: 'mode', mode: value });
});
const savedMode = vscode.getState()?.mode;
if (savedMode) vscode.postMessage({ type: 'mode', mode: savedMode });
vscode.postMessage({ type: 'ready' });

function render(focus = (document.activeElement as HTMLElement | null)?.dataset.node): void {
    elements = new Map();
    const visible: HTMLLIElement[] = [];
    function add(parent: string | null, level: number): void {
        for (const row of children.get(parent) ?? []) {
            const item = create('li');
            item.setAttribute('role', 'treeitem');
            item.setAttribute('aria-level', String(level + 1));
            item.dataset.node = row.id;
            item.style.setProperty('--level', String(level));
            const branch = children.has(row.id);
            const open = !collapsed.has(row.id);
            const twisty = glyph(branch ? (open ? 'chevron-down' : 'chevron-right') : '', 'twisty');
            if (branch) item.setAttribute('aria-expanded', String(open));
            item.append(twisty, glyph(row.canInsert || branch ? 'symbol-structure' : 'symbol-field'), create('span', row.label));
            if (row.name) item.append(Object.assign(create('span', row.name), { className: 'outline-name' }));
            if (row.runtime) {
                const runtime = row.runtime;
                const provenance = runtime.visual && runtime.logical ? 'Visual and logical' : runtime.visual ? 'Visual' : 'Logical';
                const origin = create('span', runtime.source_id === null ? 'Runtime' : 'Source');
                origin.className = 'outline-origin';
                origin.title = provenance;
                item.append(origin);
            }
            visible.push(item);
            elements.set(row.id, item);
            if (open) add(row.id, level + 1);
        }
    }
    add(null, 0);
    tree.replaceChildren(...visible);
    syncRows();
    if (focus) elements.get(focus)?.focus({ preventScroll: true });
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
    const row = item && rows.get(item.dataset.node ?? '');
    return item && row ? { item, row } : undefined;
}

function select(id: string, openSource = false): void {
    if (!state.frame) return;
    error = undefined;
    vscode.postMessage({ type: 'select', frame: state.frame, id, openSource });
}

function toggle(id: string): void {
    if (!collapsed.delete(id)) collapsed.add(id);
    render(String(id));
}

function expandTo(node?: string): void {
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
    if (row.source === undefined || state.mode !== 'source') return undefined;
    if (drag.origin.type === 'node') {
        for (let ancestor: OutlineRow | undefined = row; ancestor; ancestor = rows.get(ancestor.parent ?? '')) {
            if (ancestor.source === drag.origin.node) return undefined;
        }
    }
    const rect = item.getBoundingClientRect();
    const fraction = (event.clientY - rect.top) / rect.height;
    const position: OutlinePosition = row.canInsert && (row.parent === null || fraction >= .25 && fraction <= .75)
        ? 'inside' : fraction < .5 ? 'before' : 'after';
    if (position !== 'inside' && !row.canMove) return undefined;
    return { id: row.id, node: row.source, position, item };
}

function clearMarker(): void {
    if (marker) delete marker.dataset.drop;
    marker = undefined;
    clearTimeout(expansion);
    cancelAnimationFrame(hoverScroll);
}

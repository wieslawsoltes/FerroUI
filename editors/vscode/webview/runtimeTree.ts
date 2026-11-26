import { filterOutline } from '../src/outlineFilter';
import type { RuntimeNode, RuntimeTreeDisclosure } from '../src/runtimeProtocol';
import { collapsedRuntimeNodes, runtimeNodeLabel, walkRuntimeNodes } from '../src/runtimeTree';
import { create, icon } from './dom';

const rowLimit = 1200;

interface Row { id: number; parent: number | null; label: string; name: string }
interface VisibleRow extends Row {
    level: number;
    position: number;
    siblings: number;
    expanded?: boolean;
    firstChild?: number;
}

interface TreeItem { item: HTMLLIElement; twisty: HTMLElement; text: HTMLElement; children: HTMLUListElement }

/** Keep editor focus and inspection selection independent while native frames arrive. */
export class RuntimeTree {
    private source?: readonly RuntimeNode[];
    private readonly items = new Map<number, TreeItem>();
    private projected: Row[] = [];
    private visible = new Map<number, VisibleRow>();
    private selected?: number;
    private tabStop?: number;
    private search = '';
    private lastKey = 0;

    constructor(private readonly tree: HTMLUListElement, private readonly status: HTMLElement, private readonly filter: HTMLInputElement,
        private readonly inspect: (id: number) => void, private readonly disclose: (id: number, expanded: boolean) => void) {
        tree.addEventListener('keydown', event => this.keydown(event));
        tree.addEventListener('click', event => {
            const row = this.rowAt(event.target);
            if (!row) return;
            this.items.get(row.id)?.item.focus({ preventScroll: true });
            if ((event.target as Element).closest('.runtime-twisty') && row.expanded !== undefined) this.disclose(row.id, !row.expanded);
            else this.inspect(row.id);
        });
        tree.addEventListener('focusin', event => {
            const row = this.rowAt(event.target);
            if (row) this.setTabStop(row.id);
        });
    }

    update(roots: readonly RuntimeNode[] | undefined, disclosure: RuntimeTreeDisclosure): void {
        if (this.source !== roots) {
            this.source = roots;
            this.projected = Array.from(walkRuntimeNodes(roots ?? []), ({ node, parent }): Row => ({
                id: node.node_id, parent, label: runtimeNodeLabel(node), name: [node.name, node.text, node.role].filter(Boolean).join(' '),
            }));
        }
        const old = this.visible;
        const focused = this.rowAt(document.activeElement)?.id;
        const query = this.filter.value;
        const closed = new Set(collapsedRuntimeNodes(disclosure, query));
        const { rows, limited } = projectRows(filterOutline(this.projected, query), closed);
        this.visible = rows;

        this.reconcile();
        const tabStop = this.visibleAncestor(focused ?? this.tabStop, old) ?? this.entry();
        this.setTabStop(tabStop);
        if (focused !== undefined) {
            const current = tabStop === undefined ? this.filter : this.items.get(tabStop)?.item;
            if (current && document.activeElement !== current) current.focus({ preventScroll: tabStop === focused });
        }
        const empty = query.trim() ? 'No elements match the filter.' : 'This hierarchy has no elements.';
        const message = !roots ? '' : limited ? 'Showing the first 1,200 elements. Filter to find more.' : rows.size ? '' : empty;
        if (this.status.textContent !== message) this.status.textContent = message;
    }

    select(id?: number): void {
        if (id === this.selected) return;
        if (this.selected !== undefined) this.items.get(this.selected)?.item.setAttribute('aria-selected', 'false');
        this.selected = id;
        if (id !== undefined) this.items.get(id)?.item.setAttribute('aria-selected', 'true');
        if (!this.tree.contains(document.activeElement)) this.setTabStop(this.entry());
    }

    focus(): void {
        const entry = this.entry();
        if (entry !== undefined) this.items.get(entry)?.item.focus();
    }

    private entry(): number | undefined {
        return this.selected !== undefined && this.visible.has(this.selected) ? this.selected : this.visible.keys().next().value;
    }

    private visibleAncestor(id: number | undefined, old: ReadonlyMap<number, VisibleRow>): number | undefined {
        while (id !== undefined && !this.visible.has(id)) id = old.get(id)?.parent ?? undefined;
        return id;
    }

    private setTabStop(id: number | undefined): void {
        if (this.tabStop !== undefined && this.tabStop !== id) {
            const previous = this.items.get(this.tabStop);
            if (previous) previous.item.tabIndex = -1;
        }
        this.tabStop = id;
        const item = id === undefined ? undefined : this.items.get(id);
        if (item && item.item.tabIndex !== 0) item.item.tabIndex = 0;
    }

    private reconcile(): void {
        for (const [id, data] of this.items) if (!this.visible.has(id)) { data.item.remove(); this.items.delete(id); }
        const next = new Map<HTMLUListElement, Element | null>();
        for (const row of this.visible.values()) {
            let view = this.items.get(row.id);
            if (!view) this.items.set(row.id, view = createItem(row.id));
            const list = row.parent === null ? this.tree : this.items.get(row.parent)!.children;
            const anchor = next.has(list) ? next.get(list)! : list.firstElementChild;
            if (view.item !== anchor) list.insertBefore(view.item, anchor);
            next.set(list, view.item.nextElementSibling);
            showRow(view, row, row.id === this.selected);
        }
    }

    private rowAt(target: EventTarget | null): VisibleRow | undefined {
        const item = target instanceof Element ? target.closest<HTMLLIElement>('li[data-node]') : null;
        return item && this.tree.contains(item) ? this.visible.get(Number(item.dataset.node)) : undefined;
    }

    private keydown(event: KeyboardEvent): void {
        const row = this.rowAt(event.target);
        if (!row || event.altKey || event.ctrlKey || event.metaKey || event.isComposing) return;
        const order = [...this.visible.keys()];
        const index = order.indexOf(row.id);
        let next: number | undefined;
        switch (event.key) {
            case 'ArrowDown': next = order[index + 1]; break;
            case 'ArrowUp': next = order[index - 1]; break;
            case 'Home': next = order[0]; break;
            case 'End': next = order.at(-1); break;
            case 'ArrowRight': if (row.expanded === false) this.disclose(row.id, true); else next = row.firstChild; break;
            case 'ArrowLeft': if (row.expanded === true) this.disclose(row.id, false); else next = row.parent ?? undefined; break;
            case 'Enter': case ' ': this.inspect(row.id); break;
            default:
                if (Array.from(event.key).length !== 1) return;
                next = this.typeAhead(event.key, order, index);
        }
        event.preventDefault();
        event.stopPropagation();
        if (next !== undefined) this.items.get(next)?.item.focus();
    }

    private typeAhead(key: string, order: readonly number[], index: number): number | undefined {
        const now = performance.now();
        const character = key.toLowerCase();
        this.search = now - this.lastKey < 700 && this.search !== character ? this.search + character : character;
        this.lastKey = now;
        return [...order.slice(index + 1), ...order.slice(0, index + 1)].find(id => {
            const row = this.visible.get(id)!;
            return row.name.toLowerCase().startsWith(this.search) || row.label.toLowerCase().startsWith(this.search);
        });
    }
}

function projectRows(rows: readonly Row[], closed: ReadonlySet<number>) {
    const groups = new Map<number | null, Row[]>();
    for (const row of rows) {
        const children = groups.get(row.parent) ?? [];
        children.push(row);
        groups.set(row.parent, children);
    }
    const visible = new Map<number, VisibleRow>();
    let limited = false;
    const visit = (parent: number | null, level: number) => {
        const siblings = groups.get(parent) ?? [];
        for (const [index, row] of siblings.entries()) {
            if (visible.size >= rowLimit) { limited = true; return; }
            const firstChild = groups.get(row.id)?.[0]?.id;
            const expanded = firstChild === undefined ? undefined : !closed.has(row.id);
            visible.set(row.id, { ...row, level, expanded, firstChild, position: index + 1, siblings: siblings.length });
            if (expanded) visit(row.id, level + 1);
        }
    };
    visit(null, 1);
    return { rows: visible, limited };
}

function createItem(id: number): TreeItem {
    const item = create('li');
    item.tabIndex = -1;
    item.dataset.node = String(id);
    item.setAttribute('role', 'treeitem');
    const line = create('div');
    line.className = 'runtime-tree-row';
    const twisty = icon('chevron-down');
    twisty.classList.add('runtime-twisty');
    const text = create('span');
    text.className = 'runtime-tree-label';
    const children = create('ul');
    children.setAttribute('role', 'group');
    line.append(twisty, text);
    item.append(line, children);
    return { item, twisty, text, children };
}

function showRow({ item, twisty, text, children }: TreeItem, row: VisibleRow, selected: boolean): void {
    if (text.textContent !== row.label) { text.textContent = row.label; text.title = row.label; }
    const branch = row.expanded !== undefined;
    if (row.expanded === undefined) item.removeAttribute('aria-expanded');
    else attribute(item, 'aria-expanded', row.expanded);
    twisty.toggleAttribute('data-leaf', !branch);
    twisty.classList.toggle('codicon-chevron-right', row.expanded === false);
    twisty.classList.toggle('codicon-chevron-down', row.expanded !== false);
    children.hidden = !row.expanded;
    attribute(item, 'aria-label', row.label);
    attribute(item, 'aria-selected', selected);
    attribute(item, 'aria-level', row.level);
    attribute(item, 'aria-posinset', row.position);
    attribute(item, 'aria-setsize', row.siblings);
    const depth = String(row.level - 1);
    if (item.style.getPropertyValue('--depth') !== depth) item.style.setProperty('--depth', depth);
}

function attribute(item: HTMLElement, name: string, value: string | boolean | number): void {
    const text = String(value);
    if (item.getAttribute(name) !== text) item.setAttribute(name, text);
}

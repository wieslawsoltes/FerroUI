import * as vscode from 'vscode';
import type { NodeTarget, OutlineNode } from './protocol';

export const OUTLINE_VIEW = 'ferroui.outline';

interface OutlineState {
    session: number;
    file: string;
    nodes: readonly OutlineNode[];
    token: number | undefined;
    selected: number | undefined;
}

type Row = Pick<OutlineNode, 'id' | 'parent' | 'label' | 'name'>;

interface Entry {
    readonly session: number;
    readonly node: Row;
}

export class OutlineView implements vscode.TreeDataProvider<Entry>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    readonly view = vscode.window.createTreeView(OUTLINE_VIEW, {
        treeDataProvider: this,
        showCollapseAll: true,
    });
    private state?: OutlineState;
    private children = new Map<number | null, Entry[]>();
    private nodes = new Map<number, Entry>();
    private readonly revealing = new Set<Entry>();
    private readonly selectionListener: vscode.Disposable;

    constructor(select: (target: NodeTarget) => void) {
        this.selectionListener = this.view.onDidChangeSelection(({ selection: [entry] }) => {
            const state = this.state;
            if (!entry || state?.token === undefined || state.session !== entry.session || state.selected === entry.node.id) return;
            if (this.revealing.has(entry)) { this.revealSelection(); return; }
            if (this.nodes.get(entry.node.id) === entry) select({ session: entry.session, token: state.token, node: entry.node.id });
        });
    }

    show(state: OutlineState | undefined): void {
        const previous = this.state;
        this.state = state;
        const changedFile = previous?.session !== state?.session;
        const updated = changedFile || !sameStructure(previous?.nodes, state?.nodes);
        if (updated) {
            const entries = changedFile ? new Map<number, Entry>() : this.nodes;
            this.nodes = new Map();
            this.children = new Map();
            if (state) {
                for (const { id, parent, label, name } of state.nodes) {
                    const node = { id, parent, label, name };
                    const old = entries.get(id);
                    const entry = old && sameRow(old.node, node) ? old : { session: state.session, node };
                    this.nodes.set(id, entry);
                    const siblings = this.children.get(parent);
                    if (siblings) siblings.push(entry);
                    else this.children.set(parent, [entry]);
                }
            }
        }
        const file = state?.file ?? '';
        const message = state ? (this.nodes.size ? '' : 'Waiting for the current design.') :
            'Open a FerroUI designer to browse its controls.';
        if (this.view.description !== file) this.view.description = file;
        if (this.view.message !== message) this.view.message = message;
        if (updated) this.changed.fire();
        if (updated || previous?.selected !== state?.selected || previous?.token === undefined) this.revealSelection();
    }

    revealSelection(): void {
        const selected = this.state?.selected;
        if (!this.view.visible || selected === undefined || this.state?.token === undefined) return;
        const entry = this.nodes.get(selected);
        if (!entry || this.view.selection[0] === entry || this.revealing.has(entry)) return;
        // A programmatic reveal can finish after the user has already picked another row.
        this.revealing.add(entry);
        const done = () => this.revealing.delete(entry);
        void this.view.reveal(entry, { select: true, focus: false }).then(done, done);
    }

    getChildren(entry?: Entry): Entry[] {
        return this.children.get(entry?.node.id ?? null) ?? [];
    }

    getParent({ node }: Entry): Entry | undefined {
        return node.parent === null ? undefined : this.nodes.get(node.parent);
    }

    getTreeItem({ session, node }: Entry): vscode.TreeItem {
        const branch = this.children.has(node.id);
        const item = new vscode.TreeItem(node.label, branch
            ? (node.parent === null ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.Collapsed)
            : vscode.TreeItemCollapsibleState.None);
        item.id = `${session}:${node.id}`;
        item.description = node.name ?? undefined;
        item.iconPath = new vscode.ThemeIcon(branch ? 'symbol-structure' : 'symbol-field');
        item.tooltip = node.name ? `${node.label} · ${node.name}` : node.label;
        return item;
    }

    dispose(): void {
        this.selectionListener.dispose();
        this.view.dispose();
        this.changed.dispose();
    }
}

function sameRow(a: Row, b: Row): boolean {
    return a.id === b.id && a.parent === b.parent && a.label === b.label && a.name === b.name;
}

function sameStructure(a?: readonly OutlineNode[], b?: readonly OutlineNode[]): boolean {
    return a === b || (a !== undefined && b !== undefined && a.length === b.length && a.every((node, i) => sameRow(node, b[i])));
}

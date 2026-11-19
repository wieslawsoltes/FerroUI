import * as vscode from 'vscode';
import { ColumnEditor, columnKinds, type AuthoredColumn, type ColumnField, type TableColumns } from './columnEditor';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { StudioContext } from './studioContext';

export const COLUMNS_VIEW = 'ferroui.columns';
const treeMime = 'application/vnd.code.tree.ferroui.columns';
const dragMime = 'application/vnd.ferroui.columns';
const command = (name: string) => `${COLUMNS_VIEW}.${name}`;

interface ColumnRow {
    id: string;
    editor: ColumnEditor;
    column: AuthoredColumn;
    index?: number;
    field?: ColumnField;
}

interface ColumnDrag {
    model: string;
    column: number;
}

export class ColumnsView implements vscode.TreeDataProvider<ColumnRow>, vscode.TreeDragAndDropController<ColumnRow>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    readonly dragMimeTypes = [dragMime];
    readonly dropMimeTypes = [treeMime, dragMime];
    private readonly view: vscode.TreeView<ColumnRow>;
    private readonly subscriptions: vscode.Disposable[];
    private key?: string;
    private owner?: string;
    private panel?: DesignerPanel;
    private pending?: Promise<void>;
    private editor?: ColumnEditor;

    constructor(visibilityChanged: () => void) {
        this.view = vscode.window.createTreeView(COLUMNS_VIEW, { treeDataProvider: this, dragAndDropController: this });
        const run = (name: string, callback: (row: ColumnRow) => Promise<void>) =>
            vscode.commands.registerCommand(command(name), (row: ColumnRow | undefined) => this.run(async () => {
                if (row) await callback(row);
            }));
        this.subscriptions = [
            this.view, this.changed, this.view.onDidChangeVisibility(visibilityChanged),
            vscode.commands.registerCommand(command('add'), () => this.run(async () => {
                await this.pending;
                await this.editor?.add();
            })),
            run('edit', async ({ editor, column, field }) => { if (field) await editor.field(column, field); }),
            run('reveal', ({ editor, column, field }) => editor.context.revealRange(field?.range ?? column.range)),
            run('reset', async ({ editor, column, field }) => {
                if (field) await editor.change({ action: 'clear', column: column.node, field: field.name });
            }),
            run('duplicate', ({ editor, column }) => editor.change({ action: 'duplicate', column: column.node })),
            run('remove', ({ editor, column }) => editor.change({ action: 'remove', column: column.node })),
            run('earlier', row => this.move(row.editor, row.column.node, row.index === undefined ? undefined : row.index - 1)),
            run('later', row => this.move(row.editor, row.column.node, row.index === undefined ? undefined : row.index + 1)),
        ];
    }

    get visible(): boolean { return this.view.visible; }
    get available(): boolean { return this.owner !== undefined; }

    update(panel: DesignerPanel | undefined): void {
        this.panel = panel;
        const node = panel && enclosingTable(panel);
        const owner = panel && node ? `${panel.session}:${node.name ?? node.id}` : undefined;
        const changedOwner = owner !== this.owner;
        if (changedOwner) {
            this.owner = owner;
            this.editor = undefined;
            this.changed.fire();
            void vscode.commands.executeCommand('setContext', 'ferroui.hasColumnEditor', this.available);
        }
        const key = node && panel ? `${owner}:${panel.currentToken}:${panel.selectedIds.join(',')}:${this.visible}` : undefined;
        if (key === this.key) return;
        this.key = key;
        const token = panel?.currentToken;
        this.pending = undefined;
        void vscode.commands.executeCommand('setContext', 'ferroui.canAddColumn', false);
        if (!panel || !node || !this.visible) return;
        this.view.description = node.name ?? node.label;
        const unavailable = panel.snapshot?.renderError ? 'Repair the preview to edit columns.' : panel.status;
        this.view.message = unavailable;
        if (token === undefined || unavailable) return;
        const context = new StudioContext(panel, token, () => this.panel === panel && this.owner === owner);
        this.pending = context.read<TableColumns | { table: null }>('designer_columns', { node: node.id }).then(data => {
            if (this.key !== key) return;
            this.editor = data.table === null ? undefined : new ColumnEditor(context, data);
            void vscode.commands.executeCommand('setContext', 'ferroui.canAddColumn',
                data.table !== null && !data.reason && data.available.length > 0);
            this.view.message = data.table === null ? 'Select a native FerroUI table.'
                : data.reason ?? (data.columns.length ? undefined : 'Use Add Column to define the first column.');
            this.changed.fire();
        }, error => {
            if (this.key === key) this.view.message = errorMessage(error);
        });
    }

    getTreeItem(row: ColumnRow): vscode.TreeItem {
        const { id, editor, column, field, index } = row;
        const writable = !editor.table.reason;
        const item = new vscode.TreeItem(field?.name ?? column.label, field
            ? vscode.TreeItemCollapsibleState.None : vscode.TreeItemCollapsibleState.Collapsed);
        item.id = id;
        if (field) {
            item.description = field.value ?? (field.range ? 'Object' : undefined);
            const accessible = [field.name, item.description].filter(Boolean).join(' ');
            item.accessibilityInformation = { label: accessible };
            item.tooltip = accessible;
            const editable = field.editable && writable;
            item.iconPath = new vscode.ThemeIcon(field.kind === 'binding' ? 'link' : field.kind === 'object' ? 'symbol-structure' : 'symbol-property');
            item.contextValue = ['field', field.range && 'source', writable && field.canClear && 'reset'].filter(Boolean).join(' ');
            if (editable || field.range) item.command = { command: command('edit'), title: field.name, arguments: [row] };
        } else {
            const kind = columnKinds[column.kind];
            item.iconPath = new vscode.ThemeIcon(kind.icon);
            item.description = [column.kind !== 'table' && kind.label !== column.label ? kind.label : undefined,
                column.fields.find(field => field.name === 'Width')?.value].filter(Boolean).join(' · ');
            const reorderable = writable && index !== undefined;
            item.contextValue = ['column', 'source', reorderable && 'remove', reorderable && column.canDuplicate && 'duplicate',
                reorderable && index > 0 && 'earlier', reorderable && index < editor.table.columns.length - 1 && 'later'].filter(Boolean).join(' ');
        }
        return item;
    }

    async getChildren(row?: ColumnRow): Promise<ColumnRow[]> {
        if (row) return row.field ? [] : [
            ...rows(row.column.children, row.editor, row.id, false),
            ...row.column.fields.filter(field => field.editable || field.range).map(field => ({
                id: `${row.id}/field:${field.name}`, editor: row.editor, column: row.column, field,
            })),
        ];
        await this.pending;
        return this.editor ? rows(this.editor.table.columns, this.editor, this.owner!, true) : [];
    }

    handleDrag(source: readonly ColumnRow[], transfer: vscode.DataTransfer): void {
        const [row] = source;
        if (source.length === 1 && row.index !== undefined && row.editor === this.editor && !row.editor.table.reason) {
            const drag: ColumnDrag = { model: row.editor.table.model, column: row.column.node };
            transfer.set(dragMime, new vscode.DataTransferItem(drag));
        }
    }

    async handleDrop(target: ColumnRow | undefined, transfer: vscode.DataTransfer, token: vscode.CancellationToken): Promise<void> {
        const drag = transfer.get(dragMime)?.value as ColumnDrag | undefined;
        const editor = this.editor;
        if (!drag || !editor || token.isCancellationRequested || drag.model !== editor.table.model || (target && target.editor !== editor)) return;
        const index = target ? target.index : editor.table.columns.length - 1;
        await this.run(() => this.move(editor, drag.column, index));
    }

    private async move(editor: ColumnEditor, column: number, index?: number): Promise<void> {
        if (index === undefined || index < 0 || index >= editor.table.columns.length) return;
        const before = editor.table.columns.findIndex(item => item.node === column);
        if (before !== -1 && before !== index) await editor.change({ action: 'move', column, index });
    }

    private async run(action: () => Promise<void>): Promise<void> {
        try { await action(); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}

function rows(columns: AuthoredColumn[], editor: ColumnEditor, parent: string, top: boolean): ColumnRow[] {
    const counts = new Map<string, number>();
    return columns.map((column, index) => {
        const key = `${column.kind}:${column.label}`;
        const occurrence = counts.get(key) ?? 0;
        counts.set(key, occurrence + 1);
        return { id: `${parent}/${key}:${occurrence}`, editor, column, index: top ? index : undefined };
    });
}

function enclosingTable(panel: DesignerPanel) {
    if (panel.language !== 'xaml' || !panel.supportsColumnAuthoring || panel.selectedIds.length !== 1) return;
    let node = panel.selectedNode;
    while (node) {
        const type = node.label.split(':').at(-1);
        if (type === 'TableView' || type === 'TreeDataGrid') return node;
        const parent = node.parent;
        node = parent === null ? undefined : panel.nodes.find(item => item.id === parent);
    }
    return undefined;
}

import * as vscode from 'vscode';
import { BehaviorEditor, BEHAVIOR_QUERY, describeAction, type ActionRow, type AttachedBehavior, type BehaviorReport } from './behaviorEditor';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { triggerLabel } from './studioBehaviors';
import { StudioContext } from './studioContext';

export const BEHAVIORS_VIEW = 'ferroui.behaviors';
const command = (action: string) => `${BEHAVIORS_VIEW}.${action}`;
const treeMime = 'application/vnd.code.tree.ferroui.behaviors';
const dragMime = 'application/vnd.ferroui.behavior-action';

interface Row {
    editor: BehaviorEditor;
    behavior: AttachedBehavior;
    action?: ActionRow;
    index?: number;
}

interface ActionDrag { session: number; frame: number; behavior: number; action: number }

export class BehaviorsView implements vscode.TreeDataProvider<Row>, vscode.TreeDragAndDropController<Row>, vscode.Disposable {
    private readonly changes = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changes.event;
    readonly dragMimeTypes = [dragMime];
    readonly dropMimeTypes = [treeMime, dragMime];
    private readonly view: vscode.TreeView<Row>;
    private readonly subscriptions: vscode.Disposable[];
    private panel?: DesignerPanel;
    private owner?: string;
    private key?: string;
    private loading?: Promise<void>;
    private editor?: BehaviorEditor;

    constructor(visibilityChanged: () => void) {
        this.view = vscode.window.createTreeView(BEHAVIORS_VIEW, { treeDataProvider: this, dragAndDropController: this });
        const forRow = (name: string, run: (row: Row) => Promise<void> | undefined) =>
            vscode.commands.registerCommand(command(name), (row?: Row) => this.run(async () => {
                if (row) await run(row);
            }));
        this.subscriptions = [
            this.view, this.changes, this.view.onDidChangeVisibility(visibilityChanged),
            vscode.commands.registerCommand(command('attach'), () => this.run(async () => {
                await this.loading;
                await this.editor?.attach();
            })),
            vscode.commands.registerCommand(command('clear'), () => this.run(async () => {
                await this.loading;
                if (this.editor?.report.activePreview != null) await this.editor.clear();
            })),
            forRow('edit', ({ editor, behavior, action }) => action ? editor.action(behavior, action) : editor.trigger(behavior)),
            forRow('preview', ({ editor, behavior }) => editor.preview(behavior)),
            forRow('append', ({ editor, behavior }) => editor.append(behavior)),
            forRow('source', ({ editor, behavior, action }) => editor.context.revealRange(action?.range ?? behavior.range)),
            forRow('earlier', row => row.action && row.index !== undefined ? row.editor.move(row.behavior, row.action, row.index - 1) : undefined),
            forRow('later', row => row.action && row.index !== undefined ? row.editor.move(row.behavior, row.action, row.index + 1) : undefined),
            forRow('remove', ({ editor, behavior, action }) => editor.edit(behavior, action
                ? { action: 'remove_action', action_node: action.id } : { action: 'remove' })),
        ];
    }

    get visible(): boolean { return this.view.visible; }
    get available(): boolean { return this.owner !== undefined; }

    update(panel: DesignerPanel | undefined): void {
        const node = panel?.supportsBehaviorAuthoring && panel.selectedIds.length === 1 ? panel.selectedNode : undefined;
        const owner = panel && node?.bounds ? `${panel.session}:${node.id}` : undefined;
        this.panel = panel;
        if (owner !== this.owner) {
            this.owner = owner;
            this.editor = undefined;
            this.changes.fire();
            void vscode.commands.executeCommand('setContext', 'ferroui.hasBehaviorEditor', this.available);
        }
        const token = panel?.currentToken;
        const key = owner ? `${owner}:${token}:${this.visible}` : undefined;
        if (key === this.key) return;
        this.key = key;
        this.loading = undefined;
        void vscode.commands.executeCommand('setContext', 'ferroui.canAttachBehavior', false);
        void vscode.commands.executeCommand('setContext', 'ferroui.canResetBehavior', false);
        if (!owner || !node || !panel || !this.visible) return;
        this.view.description = node.name ?? node.label;
        this.view.message = panel.status;
        if (token === undefined || panel.status) return;
        const context = new StudioContext(panel, token, () => this.panel === panel && this.owner === owner && this.visible);
        this.loading = context.read<BehaviorReport>(BEHAVIOR_QUERY, { owner: node.id }).then(report => {
            if (this.key !== key) return;
            this.editor = new BehaviorEditor(context, report);
            this.view.message = report.behaviors.length ? undefined : report.canAttach
                ? 'Attach a trigger to this control.' : 'Edit the behavior collection in source.';
            void vscode.commands.executeCommand('setContext', 'ferroui.canAttachBehavior', report.canAttach);
            void vscode.commands.executeCommand('setContext', 'ferroui.canResetBehavior', report.activePreview !== null);
            this.changes.fire();
        }, error => {
            if (this.key === key) this.view.message = errorMessage(error);
        });
    }

    getTreeItem(row: Row): vscode.TreeItem {
        const { editor, behavior, action, index } = row;
        const info = action ? describeAction(action) : undefined;
        const item = new vscode.TreeItem(info?.label ?? triggerLabel(behavior.trigger),
            !action && behavior.actionRows.length ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.None);
        item.id = `${editor.context.panel.session}:${behavior.id}${action ? `:${action.id}` : ''}`;
        const kind = 'Event' in behavior.trigger ? 'Event' : 'Data' in behavior.trigger ? 'Condition' : 'Custom';
        item.iconPath = new vscode.ThemeIcon(info?.icon ?? (kind === 'Event' ? 'symbol-event' : kind === 'Condition' ? 'filter' : 'symbol-structure'));
        item.description = action ? info?.summary : [kind, editor.report.activePreview === behavior.id && 'Preview'].filter(Boolean).join(' · ');
        const accessible = [item.label, item.description].filter(Boolean).join(', ');
        item.accessibilityInformation = { label: accessible };
        item.tooltip = accessible;
        item.contextValue = ['behavior', action ? 'action' : 'trigger', 'source',
            editor.report.editable && (!action || behavior.canEditActions) && 'remove',
            !action && behavior.canPreview && 'preview',
            !action && behavior.canEditActions && 'append',
            action && behavior.canEditActions && index !== undefined && index > 0 && 'earlier',
            action && behavior.canEditActions && index !== undefined && index < behavior.actionRows.length - 1 && 'later',
        ].filter(Boolean).join(' ');
        item.command = { command: command('edit'), title: action ? 'Edit action' : 'Edit trigger', arguments: [row] };
        return item;
    }

    async getChildren(row?: Row): Promise<Row[]> {
        if (row) return row.action ? [] : row.behavior.actionRows.map((action, index) => ({ ...row, action, index }));
        await this.loading;
        const editor = this.editor;
        return editor?.report.behaviors.map(behavior => ({ editor, behavior })) ?? [];
    }

    handleDrag(rows: readonly Row[], transfer: vscode.DataTransfer, token: vscode.CancellationToken): void {
        const [row] = rows;
        if (token.isCancellationRequested || rows.length !== 1 || !row.action || row.editor !== this.editor
            || !row.behavior.canEditActions || row.editor.context.token !== this.panel?.currentToken) return;
        const drag: ActionDrag = { session: row.editor.context.panel.session, frame: row.editor.context.token,
            behavior: row.behavior.id, action: row.action.id };
        transfer.set(dragMime, new vscode.DataTransferItem(JSON.stringify(drag)));
    }

    async handleDrop(target: Row | undefined, transfer: vscode.DataTransfer, token: vscode.CancellationToken): Promise<void> {
        const item = transfer.get(dragMime);
        if (!item || !target || token.isCancellationRequested || target.editor !== this.editor) return;
        await this.run(async () => {
            const drag = JSON.parse(await item.asString()) as ActionDrag | null;
            const { editor, behavior } = target;
            if (!drag || token.isCancellationRequested || editor !== this.editor || drag.session !== editor.context.panel.session
                || drag.frame !== editor.context.token || drag.frame !== this.panel?.currentToken || drag.behavior !== behavior.id) return;
            const source = behavior.actionRows.find(row => row.id === drag.action);
            if (source && source !== target.action) await editor.move(behavior, source, target.index ?? behavior.actionRows.length - 1);
        });
    }

    dispose(): void { this.subscriptions.forEach(item => item.dispose()); }

    private async run(callback: () => Promise<void>): Promise<void> {
        try { await callback(); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }
}

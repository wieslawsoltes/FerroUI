import * as path from 'node:path';
import * as vscode from 'vscode';
import { DesignerPanel } from './designerPanel';
import { DesignerDrag } from './designerDrag';
import { DockView } from './dockView';
import { PROPERTIES_VIEW, PropertiesView } from './propertiesView';
import { errorMessage, type FrameTarget, type FromOutline, type FromToolbox, type NodeOperation, type NodeTarget, type OutlineState, type ToolboxState, type ToOutline, type ToToolbox } from './protocol';
import { languageOf } from './textDocument';
import { outlineHtml, toolboxHtml, webviewOptions } from './view';

const OUTLINE_VIEW = 'ferroui.outline';
const TOOLBOX_VIEW = 'ferroui.toolbox';

export class DesignerWorkbench implements vscode.Disposable {
    private readonly panels = new Map<string, DesignerPanel>();
    private readonly outline: DockView<OutlineState, FromOutline, ToOutline>;
    private readonly toolbox: DockView<ToolboxState, FromToolbox, ToToolbox>;
    private readonly properties: PropertiesView;
    private readonly drag = new DesignerDrag(update => {
        this.outline.post(update);
        this.toolbox.post(update);
        for (const panel of this.panels.values()) panel.post(update);
    });
    private readonly subscriptions: vscode.Disposable[];
    private nextSession = 0;
    private disposed = false;
    private active?: DesignerPanel;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel) {
        const changed = () => this.updateDocks();
        this.outline = new DockView(context.extensionUri, outlineHtml, changed, message => { void this.outlineMessage(message); });
        this.toolbox = new DockView(context.extensionUri, toolboxHtml, changed, message => { void this.toolboxMessage(message); });
        this.properties = new PropertiesView(context.extensionUri, changed, (target, operation) => this.edit(target, operation),
            () => { void this.showDock(TOOLBOX_VIEW); });
        this.subscriptions = [
            vscode.window.registerWebviewViewProvider(OUTLINE_VIEW, this.outline),
            vscode.window.registerWebviewViewProvider(TOOLBOX_VIEW, this.toolbox),
            vscode.window.registerWebviewViewProvider(PROPERTIES_VIEW, this.properties),
            vscode.window.onDidChangeActiveTextEditor(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabGroups(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabs(() => this.followFocus()),
            vscode.commands.registerCommand('ferroui.openDesigner', (uri?: vscode.Uri) => this.open(uri)),
            vscode.commands.registerCommand('ferroui.showOutline', (session?: number) => this.showDock(OUTLINE_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showProperties', (session?: number) => this.showDock(PROPERTIES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showToolbox', (session?: number) => this.showDock(TOOLBOX_VIEW, session)),
            vscode.commands.registerCommand('ferroui.revealDesignerSource', () => {
                const panel = this.active;
                const node = panel?.selectedNode;
                const token = panel?.currentToken;
                if (panel && node && token !== undefined) return this.select({ session: panel.session, token, node: node.id }, true);
            }),
        ];
        this.updateDocks();
    }

    private async open(uri?: vscode.Uri): Promise<void> {
        try {
            await this.openDocument(uri);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    async openDocument(uri?: vscode.Uri, caret?: vscode.Position): Promise<DesignerPanel> {
        const active = vscode.window.activeTextEditor;
        const document = uri ? await vscode.workspace.openTextDocument(uri) : active?.document;
        const language = document && languageOf(document);
        if (!document || !language) throw new Error('Open a local FerroUI XAML or Rust file.');
        const source = active?.document === document ? active : await vscode.window.showTextDocument(document);
        const position = caret ? document.validatePosition(caret) : source.selection.active;
        const key = document.uri.toString();
        const previous = this.panels.get(key);
        if (previous) {
            previous.show(position);
            this.setActive(previous);
            return previous;
        }
        const panel = vscode.window.createWebviewPanel('ferroui.designer', `Design · ${path.basename(document.fileName)}`, {
            viewColumn: vscode.ViewColumn.Beside, preserveFocus: true,
        }, { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
        const designer: DesignerPanel = new DesignerPanel(this.context, this.output, panel, document, ++this.nextSession, language,
            position, () => { this.panels.delete(key); this.followFocus(); }, message => {
                void this.drag.receive(designer, message).catch(error => designer.post({ type: 'edit', error: errorMessage(error) }));
            });
        designer.onDidChange(() => this.followFocus());
        this.panels.set(key, designer);
        this.setActive(designer);
        return designer;
    }

    designs(): readonly DesignerPanel[] { return [...this.panels.values()]; }
    get activeDesign(): DesignerPanel | undefined { return this.active; }

    async revealDesign(panel: DesignerPanel, view: 'canvas' | 'outline' | 'properties' | 'toolbox'): Promise<void> {
        if (view === 'canvas') {
            panel.panel.reveal(undefined, false);
            this.setActive(panel);
        } else await this.showDock({ outline: OUTLINE_VIEW, properties: PROPERTIES_VIEW, toolbox: TOOLBOX_VIEW }[view], panel.session);
    }

    private followFocus(): void {
        if (this.disposed) return;
        const editor = vscode.window.activeTextEditor;
        const source = editor && this.panels.get(editor.document.uri.toString());
        const panels = [...this.panels.values()];
        const canvas = panels.find(designer => designer.panel.active);
        const fallback = this.active && panels.includes(this.active) && this.active.panel.visible ? this.active : undefined;
        this.setActive(canvas ?? source ?? fallback ?? panels.reverse().find(designer => designer.panel.visible));
    }

    private setActive(panel: DesignerPanel | undefined): void {
        if (this.active !== panel) {
            this.active?.setInspected(false);
            this.active = panel;
        }
        this.updateDocks();
    }

    private updateDocks(): void {
        const panel = this.active;
        panel?.setInspected(this.outline.visible || this.properties.visible || this.toolbox.visible);
        const file = panel ? vscode.workspace.asRelativePath(panel.document.uri, false) : '';
        const token = panel?.currentToken;
        const node = panel?.selectedNode;
        const frame = panel && token !== undefined && !panel.status ? { session: panel.session, token } : undefined;
        this.outline.show({ frame, session: panel?.session, selected: node?.id, message: panel?.status,
            nodes: panel?.nodes.map(({ id, parent, label, name, canMove, canInsert }) => ({ id, parent, label, name, canMove, canInsert })) ?? [] }, file);
        this.toolbox.show({ frame, assets: panel ? [...panel.assets] : [], canInsert: Boolean(panel && this.insertionParent(panel)),
            message: panel?.status }, file);
        this.properties.show({
            session: panel?.session,
            node,
            target: panel && node && token !== undefined && !panel.status ? { session: panel.session, token, node: node.id } : undefined,
            message: panel?.status,
        }, file);
    }

    private insertionParent(panel: DesignerPanel): number | undefined {
        let node = panel.selectedNode ?? panel.nodes.at(0);
        while (node) {
            if (node.canInsert) return node.id;
            node = panel.nodes.find(candidate => candidate.id === node?.parent);
        }
        return undefined;
    }

    private panelFor(frame: FrameTarget): DesignerPanel {
        if (this.active?.canUse(frame)) return this.active;
        throw new Error('The active design changed. Wait for the current preview and try again.');
    }

    private async outlineMessage(message: FromOutline): Promise<void> {
        try {
            switch (message.type) {
                case 'select': return await this.select(message.target, message.openSource);
                case 'reorder': return await this.panelFor(message.frame).place(message.frame, {
                    origin: { type: 'node', node: message.node },
                    target: { type: 'outline', node: message.anchor, position: message.position },
                });
                case 'beginDrag': case 'endDrag': case 'drop':
                    if (this.active) await this.drag.receive(this.active, message);
                    break;
            }
        } catch (error) {
            this.outline.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private async toolboxMessage(message: FromToolbox): Promise<void> {
        try {
            if (message.type === 'insert') {
                const panel = this.panelFor(message.frame);
                const node = this.insertionParent(panel);
                if (node === undefined) throw new Error('Select a panel that accepts children or drag the control to a drop target.');
                if (!panel.assets.some(asset => asset.id === message.control)) throw new Error('This control is unavailable in the current design.');
                await panel.place(message.frame, { origin: { type: 'asset', control: message.control }, target: { type: 'outline', node, position: 'inside' } });
            } else if (message.type !== 'ready' && this.active) {
                await this.drag.receive(this.active, message);
            }
        } catch (error) {
            this.toolbox.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private showDock(id: string, session?: number): Thenable<void> {
        if (session !== undefined) {
            const panel = [...this.panels.values()].find(designer => designer.session === session);
            if (panel) this.setActive(panel);
        }
        return vscode.commands.executeCommand(`${id}.focus`);
    }

    private async select(target: NodeTarget, openSource = false): Promise<void> {
        try {
            await this.active?.select(target, openSource);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    private async edit(target: NodeTarget, operation: NodeOperation): Promise<void> {
        if (this.active?.session !== target?.session || !this.active) {
            throw new Error('The active design changed. Select a control in the current design.');
        }
        await this.active.editNode(target, operation);
    }

    dispose(): void {
        this.disposed = true;
        this.drag.clear();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.outline.dispose();
        this.toolbox.dispose();
        this.properties.dispose();
        const panels = [...this.panels.values()];
        this.panels.clear();
        this.active = undefined;
        for (const panel of panels) panel.dispose();
    }
}

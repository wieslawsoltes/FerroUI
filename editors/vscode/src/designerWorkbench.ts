import * as path from 'node:path';
import * as vscode from 'vscode';
import { DesignerPanel } from './designerPanel';
import { OUTLINE_VIEW, OutlineView } from './outlineView';
import { PROPERTIES_VIEW, PropertiesView } from './propertiesView';
import { errorMessage, type NodeOperation, type NodeTarget } from './protocol';
import { languageOf } from './textDocument';
import { webviewOptions } from './view';

export class DesignerWorkbench implements vscode.Disposable {
    private readonly panels = new Map<string, DesignerPanel>();
    private readonly outline = new OutlineView(target => { void this.select(target); });
    private readonly properties: PropertiesView;
    private readonly subscriptions: vscode.Disposable[];
    private nextSession = 0;
    private disposed = false;
    private active?: DesignerPanel;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel) {
        this.properties = new PropertiesView(context.extensionUri, () => this.updateDocks(), (target, operation) => this.edit(target, operation));
        this.subscriptions = [
            vscode.window.registerWebviewViewProvider(PROPERTIES_VIEW, this.properties),
            vscode.window.onDidChangeActiveTextEditor(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabGroups(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabs(() => this.followFocus()),
            this.outline.view.onDidChangeVisibility(() => {
                this.updateDocks();
                this.outline.revealSelection();
            }),
            vscode.commands.registerCommand('ferroui.openDesigner', (uri?: vscode.Uri) => this.open(uri)),
            vscode.commands.registerCommand('ferroui.showOutline', (session?: number) => this.showDock(OUTLINE_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showProperties', (session?: number) => this.showDock(PROPERTIES_VIEW, session)),
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
            const active = vscode.window.activeTextEditor;
            const document = uri ? await vscode.workspace.openTextDocument(uri) : active?.document;
            const language = document && languageOf(document);
            if (!document || !language) throw new Error('Open a local FerroUI XAML or Rust file.');
            const source = active?.document === document ? active : await vscode.window.showTextDocument(document);
            const key = document.uri.toString();
            const previous = this.panels.get(key);
            if (previous) {
                previous.show(source.selection.active);
                this.setActive(previous);
                return;
            }
            const panel = vscode.window.createWebviewPanel('ferroui.designer', `Design · ${path.basename(document.fileName)}`, {
                viewColumn: vscode.ViewColumn.Beside, preserveFocus: true,
            }, { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
            const designer = new DesignerPanel(this.context, this.output, panel, document, ++this.nextSession, language,
                source.selection.active, () => { this.panels.delete(key); this.followFocus(); });
            designer.onDidChange(() => this.followFocus());
            this.panels.set(key, designer);
            this.setActive(designer);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
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
        panel?.setInspected(this.outline.view.visible || this.properties.visible);
        const file = panel ? vscode.workspace.asRelativePath(panel.document.uri, false) : '';
        const token = panel?.currentToken;
        const node = panel?.selectedNode;
        this.outline.show(panel && { session: panel.session, file, nodes: panel.nodes, token, selected: node?.id });
        this.properties.show({
            session: panel?.session,
            node,
            target: panel && node && token !== undefined && !panel.status ? { session: panel.session, token, node: node.id } : undefined,
            message: panel?.status,
        }, file);
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
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.outline.dispose();
        this.properties.dispose();
        const panels = [...this.panels.values()];
        this.panels.clear();
        this.active = undefined;
        for (const panel of panels) panel.dispose();
    }
}

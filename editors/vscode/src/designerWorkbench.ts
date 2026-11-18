import * as path from 'node:path';
import * as vscode from 'vscode';
import { ASSETS_VIEW, AssetsView } from './assetsView';
import { DetailsViews, EVENTS_VIEW, RESOURCES_VIEW } from './detailsViews';
import { DesignerPanel } from './designerPanel';
import { DesignerDrag } from './designerDrag';
import { DESIGNER_VIEW, restoredDesigner } from './designerViewState';
import { DockView } from './dockView';
import { OutlineView } from './outlineView';
import { SelectionInspection } from './selectionInspection';
import { PROPERTIES_VIEW, PropertiesView } from './propertiesView';
import { chooseLayout } from './studioLayout';
import { errorMessage, type DesignerView, type DetailsView, type DragOrigin, type FrameTarget, type FromAssets, type FromDetails, type FromOutline, type FromToolbox, type Language, type NodeOperation, type NodeTarget, type OutlineMode, type ToolboxState, type ToToolbox, type Viewport } from './protocol';
import { languageOf } from './textDocument';
import { toolboxHtml, webviewOptions } from './view';

const OUTLINE_VIEW = 'ferroui.outline';
const TOOLBOX_VIEW = 'ferroui.toolbox';
const docks = { outline: OUTLINE_VIEW, properties: PROPERTIES_VIEW, toolbox: TOOLBOX_VIEW,
    assets: ASSETS_VIEW, events: EVENTS_VIEW, resources: RESOURCES_VIEW } as const;

export class DesignerWorkbench implements vscode.Disposable {
    private readonly panels = new Map<string, DesignerPanel>();
    private readonly outline: OutlineView;
    private readonly toolbox: DockView<ToolboxState, FromToolbox, ToToolbox>;
    private readonly properties: PropertiesView;
    private readonly inspection: SelectionInspection;
    private readonly details: DetailsViews;
    private readonly assets: AssetsView;
    private readonly drag = new DesignerDrag(update => {
        this.outline.post(update);
        this.toolbox.post(update);
        this.assets.view.post(update);
        for (const panel of this.panels.values()) panel.post(update);
    }, origin => this.assets.ownsImage(origin), async (panel, origin) => {
        const source = await this.assets.library.imageSource(panel.document.uri, origin.asset ?? '');
        if (source !== origin.source) throw new Error('The image changed during the drag. Start again from Assets.');
    });
    private readonly subscriptions: vscode.Disposable[];
    private nextSession = 0;
    private disposed = false;
    private active?: DesignerPanel;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel) {
        const changed = () => this.updateDocks();
        this.inspection = new SelectionInspection(changed);
        this.outline = new OutlineView(context.extensionUri, changed, message => { void this.outlineMessage(message); });
        this.toolbox = new DockView(context.extensionUri, toolboxHtml, changed, message => { void this.toolboxMessage(message); });
        this.details = new DetailsViews(context.extensionUri, changed, (view, message) => { void this.detailsMessage(view, message); });
        this.assets = new AssetsView(context.extensionUri, changed, message => { void this.assetsMessage(message); });
        this.properties = new PropertiesView(context.extensionUri, changed, (target, operation) => this.edit(target, operation),
            () => { void this.showDock(TOOLBOX_VIEW); },
            async (frame, range) => { await this.active?.revealSource(frame, range); });
        this.subscriptions = [
            vscode.window.registerWebviewPanelSerializer(DESIGNER_VIEW, {
                deserializeWebviewPanel: (panel, state) => this.restore(panel, state),
            }),
            vscode.window.registerWebviewViewProvider(OUTLINE_VIEW, this.outline),
            vscode.window.registerWebviewViewProvider(TOOLBOX_VIEW, this.toolbox),
            vscode.window.registerWebviewViewProvider(PROPERTIES_VIEW, this.properties),
            vscode.window.registerWebviewViewProvider(EVENTS_VIEW, this.details.events),
            vscode.window.registerWebviewViewProvider(RESOURCES_VIEW, this.details.resources),
            vscode.window.registerWebviewViewProvider(ASSETS_VIEW, this.assets.view),
            vscode.window.onDidChangeActiveTextEditor(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabGroups(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabs(() => this.followFocus()),
            vscode.commands.registerCommand('ferroui.openDesigner', (uri?: vscode.Uri) => this.open(uri)),
            vscode.commands.registerCommand('ferroui.showOutline', (session?: number) => this.showDock(OUTLINE_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showProperties', (session?: number) => this.showDock(PROPERTIES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showToolbox', (session?: number) => this.showDock(TOOLBOX_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showEvents', (session?: number) => this.showDock(EVENTS_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showResources', (session?: number) => this.showDock(RESOURCES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showAssets', (session?: number) => this.showDock(ASSETS_VIEW, session)),
            vscode.commands.registerCommand('ferroui.studio.layout', async () => {
                try {
                    if (!this.active) throw new Error('Open a FerroUI designer to arrange controls.');
                    await this.layout(this.active);
                } catch (error) {
                    void vscode.window.showErrorMessage(errorMessage(error));
                }
            }),
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
        const panel = vscode.window.createWebviewPanel(DESIGNER_VIEW, `Design · ${path.basename(document.fileName)}`, {
            viewColumn: vscode.ViewColumn.Beside, preserveFocus: true,
        }, { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
        const designer = this.attach(panel, document, language, position);
        this.setActive(designer);
        return designer;
    }

    private async restore(panel: vscode.WebviewPanel, state: unknown): Promise<void> {
        try {
            const saved = restoredDesigner(state);
            if (saved) {
                const document = await vscode.workspace.openTextDocument(saved.uri);
                const language = languageOf(document);
                if (language && !this.disposed && !this.panels.has(document.uri.toString())) {
                    this.attach(panel, document, language, document.validatePosition(saved.caret), saved.viewport);
                    this.followFocus();
                    return;
                }
            }
        } catch (error) {
            this.output.appendLine(`Could not restore the designer. ${errorMessage(error)}`);
        }
        panel.dispose();
    }

    private attach(panel: vscode.WebviewPanel, document: vscode.TextDocument, language: Language,
        caret: vscode.Position, viewport?: Viewport): DesignerPanel {
        const key = document.uri.toString();
        const designer: DesignerPanel = new DesignerPanel(this.context, this.output, panel, document, ++this.nextSession, language,
            caret, () => { this.panels.delete(key); this.followFocus(); }, message => {
                void this.drag.receive(designer, message).catch(error => designer.post({ type: 'edit', error: errorMessage(error) }));
            }, viewport);
        designer.onDidChange(() => this.followFocus());
        this.panels.set(key, designer);
        return designer;
    }

    designs(): readonly DesignerPanel[] { return [...this.panels.values()]; }
    get activeDesign(): DesignerPanel | undefined { return this.active; }

    async revealDesign(panel: DesignerPanel, view: DesignerView): Promise<void> {
        if (view === 'canvas') {
            panel.panel.reveal(undefined, false);
            this.setActive(panel);
        } else await this.showDock(docks[view], panel.session);
    }

    setOutlineMode(panel: DesignerPanel, mode: OutlineMode): void {
        this.setActive(panel);
        this.outline.switch(mode, panel);
    }

    async projectAssets(panel: DesignerPanel) {
        const folder = vscode.workspace.getWorkspaceFolder(panel.document.uri);
        return folder ? this.assets.library.list(folder, panel.document.uri, panel.language === 'xaml') : [];
    }

    async projectImage(panel: DesignerPanel, id: string): Promise<Extract<DragOrigin, { type: 'image' }>> {
        if (panel.language !== 'xaml') throw new Error('Project image placement is available in XAML.');
        return { type: 'image', source: await this.assets.library.imageSource(panel.document.uri, id) };
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
        panel?.setInspected(this.outline.visible || this.properties.visible || this.toolbox.visible
            || this.details.events.visible || this.details.resources.visible || this.assets.view.visible);
        const file = panel ? vscode.workspace.asRelativePath(panel.document.uri, false) : '';
        const token = panel?.currentToken;
        const node = panel?.selectedNode;
        const frame = panel && token !== undefined && !panel.status ? { session: panel.session, token } : undefined;
        const inspection = this.properties.visible || this.details.events.visible ? this.inspection.read(panel, frame) : {};
        this.outline.update(panel, frame, file);
        this.toolbox.show({ frame, assets: panel ? [...panel.assets] : [], canInsert: Boolean(panel && this.insertionParent(panel)),
            message: panel?.status }, file);
        this.properties.show({
            session: panel?.session,
            node,
            runtime: panel?.runtimeSelection,
            frame,
            fields: inspection.data?.properties,
            target: panel && node && token !== undefined && !panel.status ? { session: panel.session, token, node: node.id } : undefined,
            message: panel?.status ?? inspection.message,
        }, file);
        this.details.update(panel, frame, file, inspection);
        this.assets.update(panel, frame, Boolean(panel && this.insertionParent(panel)));
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
                case 'mode': return this.outline.switch(message.mode, this.active);
                case 'layout': return await this.layout(this.panelFor(message.frame));
                case 'select': return await this.outline.select(this.panelFor(message.frame), message.frame, message.id, message.openSource, message.extend);
                case 'reorder': return await this.panelFor(message.frame).place(message.frame, {
                    origin: message.origin,
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

    private layout(panel: DesignerPanel): Promise<void> {
        return chooseLayout(panel, () => this.active === panel);
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

    private async detailsMessage(view: DetailsView, message: FromDetails): Promise<void> {
        if (message.type === 'reveal') {
            await this.active?.revealSource(message.frame, message.range);
            return;
        }
        if (message.type !== 'edit') return;
        try {
            const correctOperation = view === 'events' ? message.operation.type === 'setEvent' : message.operation.type === 'applyResource';
            if (!correctOperation) throw new Error('Use the matching designer view for this edit.');
            await this.edit(message.target, message.operation);
            this.details[view].post({ type: 'edit', request: message.request });
        } catch (error) {
            this.details[view].post({ type: 'edit', request: message.request, error: errorMessage(error) });
        }
    }

    private async assetsMessage(message: FromAssets): Promise<void> {
        try {
            if (message.type === 'preview' || message.type === 'open' || message.type === 'copy') {
                await this.assets.browse(message.type, message.id);
            } else if (message.type === 'insert') {
                const panel = this.panelFor(message.frame);
                const node = this.insertionParent(panel);
                if (node === undefined) throw new Error('Select a panel that accepts children or drag the image onto the canvas.');
                const origin = await this.projectImage(panel, message.id);
                await panel.place(message.frame, { origin, target: { type: 'outline', node, position: 'inside' } });
            } else if ((message.type === 'beginDrag' || message.type === 'endDrag' || message.type === 'drop') && this.active) {
                await this.drag.receive(this.active, message);
            }
        } catch (error) {
            this.assets.view.post({ type: 'assetStatus', message: errorMessage(error), error: true });
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
        this.details.dispose();
        this.assets.dispose();
        const panels = [...this.panels.values()];
        this.panels.clear();
        this.active = undefined;
        for (const panel of panels) panel.dispose();
    }
}

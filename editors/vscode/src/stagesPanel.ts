import { Buffer } from 'node:buffer';
import { pathToFileURL } from 'node:url';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { NativePreview } from './nativePreview';
import { errorMessage } from './protocol';
import type { FromStages, StageChange, StageInfo, StageManifest, StageModel, StagesRequest, ToStages } from './stagesProtocol';
import { choose, editText } from './studioContext';
import { asRange, languageOf, sourceOf } from './textDocument';
import { stagesHtml, webviewOptions } from './view';
import { isWorkspaceFile } from './workspacePaths';

const stageView = 'ferroui.stages';

export class StagesPanel implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private readonly views = new Map<string, vscode.WebviewPanel>();

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel) {
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.openStages', () => {
                const editor = vscode.window.activeTextEditor;
                if (!editor || !languageOf(editor.document)) {
                    void vscode.window.showErrorMessage('Open a FerroUI XAML or Rust file to compare stages.');
                    return;
                }
                const key = editor.document.uri.toString();
                const previous = this.views.get(key);
                if (previous) { previous.reveal(); return; }
                const panel = vscode.window.createWebviewPanel(stageView, `Stages · ${path.basename(editor.document.fileName)}`,
                    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                    { ...webviewOptions(context.extensionUri), retainContextWhenHidden: true });
                this.attach(panel, editor.document, editor.selection.active);
            }),
            vscode.window.registerWebviewPanelSerializer(stageView, { deserializeWebviewPanel: async (panel, state) => {
                try {
                    const serialized = state as { uri?: unknown } | undefined;
                    if (typeof serialized?.uri !== 'string') { panel.dispose(); return; }
                    const uri = vscode.Uri.parse(serialized.uri, true);
                    if (!await isWorkspaceFile(uri) || this.views.has(uri.toString())) { panel.dispose(); return; }
                    this.attach(panel, await vscode.workspace.openTextDocument(uri), new vscode.Position(0, 0));
                } catch { panel.dispose(); }
            } }),
        ];
    }

    private attach(panel: vscode.WebviewPanel, document: vscode.TextDocument, caret: vscode.Position): void {
        const key = document.uri.toString();
        this.views.set(key, panel);
        panel.onDidDispose(() => this.views.delete(key));
        new StageSession(panel, document, caret, this.context, this.output);
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.views.forEach(panel => panel.dispose());
    }
}

class StageSession {
    private readonly subscriptions: vscode.Disposable[];
    private readonly cacheKey: string;
    private client?: Promise<NativePreview>;
    private model?: StageModel;
    private modelVersion?: number;
    private timer?: ReturnType<typeof setTimeout>;
    private generation = 0;
    private ready = false;
    private busy = false;
    private disposed = false;
    private manifest?: StageManifest;

    constructor(private readonly panel: vscode.WebviewPanel, private readonly document: vscode.TextDocument,
        private readonly caret: vscode.Position, private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel) {
        this.cacheKey = `ferroui.stages:${document.uri.toString()}`;
        this.manifest = context.workspaceState.get<StageManifest>(this.cacheKey);
        this.subscriptions = [
            panel.onDidDispose(() => this.dispose()),
            panel.onDidChangeViewState(() => this.schedule()),
            panel.webview.onDidReceiveMessage((message: StagesRequest) => { void this.receive(message); }),
            vscode.workspace.onDidChangeTextDocument(event => {
                const language = languageOf(event.document);
                if (!language || !event.contentChanges.length) return;
                const uri = pathToFileURL(event.document.uri.fsPath).toString();
                if (event.document === document || this.model?.manifest.stages.some(stage => stage.source_uri === uri)) this.schedule();
            }),
        ];
        panel.webview.options = webviewOptions(context.extensionUri);
        panel.webview.html = stagesHtml(panel.webview, context.extensionUri);
    }

    private schedule(): void {
        this.generation++;
        clearTimeout(this.timer);
        this.timer = undefined;
        if (this.ready && this.panel.visible && !this.disposed) this.timer = setTimeout(() => { this.timer = undefined; void this.sync(); }, 170);
    }

    private async sync(): Promise<void> {
        if (this.busy || this.disposed) return;
        const generation = this.generation;
        const language = languageOf(this.document);
        if (!language) return;
        this.busy = true;
        try {
            const client = await this.getClient();
            const manifest = this.manifest;
            const active = sourceOf(this.document, language, this.document.validatePosition(this.caret));
            const additional = await this.stageSources(manifest ?? this.model?.manifest, active.uri);
            const model = await client.syncStages(active, manifest, additional);
            if (manifest === this.manifest) this.manifest = undefined;
            await this.publish(model, client, generation);
        } catch (error) { this.report(error); }
        finally {
            this.busy = false;
            if (generation !== this.generation) this.schedule();
        }
    }

    private async publish(model: StageModel, client: NativePreview, generation: number): Promise<void> {
        if (generation !== this.generation || this.disposed) return;
        this.model = model;
        this.modelVersion = this.document.version;
        await this.context.workspaceState.update(this.cacheKey, model.manifest);
        this.post({ type: 'model', uri: this.document.uri.toString(), model });
        for (const stage of model.stages) {
            if (generation !== this.generation || this.disposed) break;
            if (stage.rendered_revision === null) continue;
            try { await this.image(client, stage); }
            catch (error) { this.report(error); }
        }
    }

    private async image(client: NativePreview, stage: StageInfo): Promise<void> {
        const target = { id: stage.id, epoch: stage.epoch };
        const image = await client.stageImage(target);
        if (this.model?.stages.some(entry => entry.id === stage.id && entry.epoch === stage.epoch)) this.post({ type: 'image', ...target, image });
    }

    private async receive(message: StagesRequest): Promise<void> {
        if (message.type === 'ready') { this.ready = true; this.schedule(); return; }
        if (this.busy || this.timer || !this.model || this.disposed || this.modelVersion !== this.document.version) return;
        const model = this.model;
        const generation = this.generation;
        try {
            const client = await this.getClient();
            if (message.type === 'prototype') {
                const command = message.command ?? await this.chooseFlow(model);
                if (command && generation === this.generation) {
                    const scene = await client.stagePrototype(model.epoch, command);
                    if (generation === this.generation) this.post({ type: 'prototype', scene });
                }
                return;
            }
            if (message.type === 'pointer') {
                const version = this.document.version;
                const result = await client.stagePointer(message);
                if (generation !== this.generation || this.document.version !== version) return;
                if (result.prototype) this.post({ type: 'prototype', scene: result.prototype });
                if (result.range && result.uri) {
                    const uri = vscode.Uri.parse(result.uri, true);
                    if (uri.toString() !== this.document.uri.toString() && !await isWorkspaceFile(uri)) return;
                    const document = await vscode.workspace.openTextDocument(uri);
                    const editor = vscode.window.visibleTextEditors.find(editor => editor.document === document);
                    const range = asRange(result.range);
                    await vscode.window.showTextDocument(document, { viewColumn: editor?.viewColumn ?? vscode.ViewColumn.Beside, preserveFocus: true, selection: range });
                }
                const stage = model.stages.find(stage => stage.id === message.id);
                if (result.interacted && stage) await this.image(client, stage);
                return;
            }
            if (message.type === 'saveManifest') {
                const uri = await vscode.window.showSaveDialog({ filters: { 'FerroUI stage set': ['ferrostages', 'json'] } });
                if (uri) await vscode.workspace.fs.writeFile(uri, Buffer.from(JSON.stringify(model.manifest, null, 2) + '\n'));
                return;
            }
            if (message.type === 'loadManifest') {
                const [uri] = await vscode.window.showOpenDialog({ canSelectMany: false, filters: { 'FerroUI stage set': ['ferrostages', 'json'] } }) ?? [];
                if (!uri) return;
                if ((await vscode.workspace.fs.stat(uri)).size > 1024 * 1024) throw new Error('The stage set exceeds 1 MiB.');
                const manifest = JSON.parse(Buffer.from(await vscode.workspace.fs.readFile(uri)).toString('utf8')) as StageManifest;
                const language = languageOf(this.document);
                if (!language || !Array.isArray(manifest.stages)) throw new Error('Open a valid Studio stage set.');
                await this.stageSources(manifest, sourceOf(this.document, language, this.caret).uri);
                this.manifest = manifest;
                this.schedule();
                return;
            }
            const change = message.type === 'add' ? await this.add(model) : message.type === 'edit'
                ? await this.change(client, model, message) : undefined;
            if (!change || generation !== this.generation || model !== this.model) return;
            await this.publish(await client.editStages(change), client, generation);
        } catch (error) { this.report(error); }
    }

    private async stageSources(manifest: StageManifest | undefined, active: string) {
        const paths = [...new Set((manifest?.stages ?? []).map(stage => stage.source_uri))].filter(uri => uri !== active);
        if (paths.length > 15) throw new Error('A Stages board supports up to 16 source documents.');
        return Promise.all(paths.map(async path => {
            const uri = vscode.Uri.parse(path, true);
            if (!await isWorkspaceFile(uri)) throw new Error('Open a stage set whose source files are inside this workspace.');
            const document = await vscode.workspace.openTextDocument(uri);
            const language = languageOf(document);
            if (!language) throw new Error('A stage source must be a FerroUI XAML or Rust file.');
            return { ...sourceOf(document, language, new vscode.Position(0, 0)), uri: path };
        }));
    }

    private async chooseFlow(model: StageModel) {
        const flows = model.manifest.prototype?.flows ?? [];
        if (!flows.length) throw new Error('Open a Studio stage set with a prototype flow.');
        const flow = await choose('Prototype flow', flows.map(flow => ({ label: flow.name, description: model.stages[flow.start]?.configuration.name })));
        return flow ? { action: 'start' as const, flow: flow.label } : undefined;
    }

    private async add(model: StageModel): Promise<StageChange | undefined> {
        const name = await editText('Stage name', 'Comparison');
        if (!name) return;
        const bounds = await dimensions(800, 520);
        if (bounds) return { action: 'add', epoch: model.epoch, name, ...bounds, dark: false };
    }

    private async change(client: NativePreview, model: StageModel, message: Extract<FromStages, { type: 'edit' }>): Promise<StageChange | undefined> {
        const { id, field, epoch } = message;
        const stage = model.stages.find(stage => stage.id === id && stage.epoch === epoch);
        if (!stage) throw new Error('The stage changed. Use its current controls.');
        if (field === 'remove' || field === 'refresh') return { id, epoch: model.epoch, action: field };
        if (field === 'export') {
            const image = await client.stageImage({ id, epoch });
            const uri = await vscode.window.showSaveDialog({ filters: { 'PNG image': ['png'] } });
            if (uri) await vscode.workspace.fs.writeFile(uri, Buffer.from(image.png, 'base64'));
            return;
        }
        const configuration = structuredClone(stage.configuration);
        switch (field) {
            case 'name': {
                const value = await editText('Stage name', configuration.name);
                if (!value) return;
                configuration.name = value;
                break;
            }
            case 'mode': configuration.mode = configuration.mode === 'select' ? 'live' : 'select'; break;
            case 'pause': configuration.auto_update = !configuration.auto_update; break;
            case 'layout': {
                const layout = await choose('Stage sizing', [
                    { label: 'Fixed width and height', value: 'fixed' }, { label: 'Fit content', value: 'size_that_fits' },
                    { label: 'Fixed width, fit height', value: 'fixed_width' }, { label: 'Fixed height, fit width', value: 'fixed_height' },
                ] as const);
                if (!layout) return;
                configuration.layout = layout.value;
                break;
            }
            case 'environment': {
                const environment = configuration.environment;
                const size = await dimensions(environment.width, environment.height);
                if (!size) return;
                const theme = await choose('Theme', [{ label: 'Light', value: 'light' }, { label: 'Dark', value: 'dark' }, { label: 'Inherit', value: 'inherit' }] as const);
                if (!theme) return;
                const direction = await choose('Direction', [{ label: 'Inherit', value: 'inherit' }, { label: 'Left to right', value: 'left_to_right' }, { label: 'Right to left', value: 'right_to_left' }] as const);
                if (!direction) return;
                configuration.environment = { ...size, theme: theme.value, flow_direction: direction.value };
                break;
            }
        }
        return { id, epoch: model.epoch, action: 'update', configuration };
    }

    private getClient(): Promise<NativePreview> {
        this.client ??= NativePreview.start(this.context, this.output);
        return this.client;
    }

    private report(error: unknown): void { this.post({ type: 'status', message: errorMessage(error) }); }
    private post(message: ToStages): void { if (!this.disposed) void this.panel.webview.postMessage(message); }
    private dispose(): void {
        this.disposed = true;
        clearTimeout(this.timer);
        void this.client?.then(client => client.dispose(), () => {});
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

async function dimensions(width: number, height: number): Promise<{ width: number; height: number } | undefined> {
    const value = await editText('Logical size, for example 800 × 520', `${width} × ${height}`);
    if (value === undefined) return;
    const parts = /^\s*(\d{1,4})\s*[x×]\s*(\d{1,4})\s*$/i.exec(value);
    if (!parts || parts.slice(1).some(part => Number(part) < 64 || Number(part) > 2048)) throw new Error('Choose dimensions from 64 to 2048.');
    return { width: Number(parts[1]), height: Number(parts[2]) };
}

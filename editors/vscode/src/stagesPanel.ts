import { Buffer } from 'node:buffer';
import { setTimeout as delay } from 'node:timers/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { BuiltProjects } from './builtProjects';
import { DesignerDependencies, dirtyXamlSources, xamlPath } from './designerDependencies';
import { NativePreview } from './nativePreview';
import { errorMessage, type Range, type Source } from './protocol';
import type { FromStages, NativeStageChange, PrototypeCommand, PrototypeEdit, StageChange, StageDocument, StageInfo, StageKeyboard, StageManifest, StageModel, StageTarget, StagesRequest, ToStages } from './stagesProtocol';
import { editPrototype } from './stagePrototypeEditor';
import { chooseStageState } from './stageStatePicker';
import { sameStageSource, stageUsesSource, retargetStageManifest, type StageFileMove } from './stageSourceMoves';
import { choose, editText } from './studioContext';
import { asRange, languageOf, sourceOf } from './textDocument';
import { stagesHtml, webviewOptions } from './view';
import { isWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';

const stageView = 'ferroui.stages';

export class StagesPanel implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private readonly views = new Set<StageSession>();

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly projects: BuiltProjects) {
        const xaml = vscode.workspace.createFileSystemWatcher('**/*.[xX][aA][mM][lL]');
        const fileChanged = (uri: vscode.Uri) => this.views.forEach(session => session.fileChanged(uri));
        this.subscriptions = [
            xaml, xaml.onDidChange(fileChanged), xaml.onDidCreate(fileChanged), xaml.onDidDelete(fileChanged),
            vscode.workspace.onDidRenameFiles(event => {
                for (const move of event.files) {
                    if (!xamlPath(move.oldUri) || !xamlPath(move.newUri) || !vscode.workspace.getWorkspaceFolder(move.newUri)) continue;
                    this.views.forEach(session => session.followMove(move));
                }
            }),
            vscode.commands.registerCommand('ferroui.openStages', () => {
                const editor = vscode.window.activeTextEditor;
                if (!editor || !languageOf(editor.document)) {
                    void vscode.window.showErrorMessage('Open a FerroUI XAML or Rust file to compare stages.');
                    return;
                }
                this.open(editor.document, editor.selection.active);
            }),
            vscode.window.registerWebviewPanelSerializer(stageView, { deserializeWebviewPanel: async (panel, state) => {
                try {
                    const serialized = state as { uri?: unknown } | undefined;
                    if (typeof serialized?.uri !== 'string') { panel.dispose(); return; }
                    const uri = vscode.Uri.parse(serialized.uri, true);
                    if (!await isWorkspaceFile(uri) || this.find(uri)) { panel.dispose(); return; }
                    this.attach(panel, await vscode.workspace.openTextDocument(uri), new vscode.Position(0, 0));
                } catch { panel.dispose(); }
            } }),
        ];
    }

    open(document: vscode.TextDocument, caret: vscode.Position): StageSession {
        const previous = this.find(document.uri);
        if (previous) { previous.panel.reveal(); return previous; }
        const panel = vscode.window.createWebviewPanel(stageView, `Stages · ${path.basename(document.fileName)}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
        return this.attach(panel, document, caret);
    }

    board(uri: vscode.Uri): StageSession {
        const board = this.find(uri);
        if (!board) throw new Error('Open Stages for this design first.');
        return board;
    }

    private attach(panel: vscode.WebviewPanel, document: vscode.TextDocument, caret: vscode.Position): StageSession {
        const session = new StageSession(panel, document, caret, this.context, this.output, this.projects);
        this.views.add(session);
        panel.onDidDispose(() => this.views.delete(session));
        return session;
    }

    private find(uri: vscode.Uri): StageSession | undefined {
        return [...this.views].find(session => sameStageSource(session.uri, uri));
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.views.forEach(session => session.panel.dispose());
    }
}

class StageSession {
    private readonly dependencies = new DesignerDependencies();
    private readonly inputImages = new Map<number, ReturnType<typeof setTimeout>>();
    private readonly subscriptions: vscode.Disposable[];
    private cacheKey: string;
    private primary: vscode.Uri;
    private readonly sourceMoves: StageFileMove[] = [];
    private nativeManifest?: StageManifest;
    private client?: Promise<NativePreview>;
    private model?: StageModel;
    private modelVersion?: number;
    private modelGeneration?: number;
    private timer?: ReturnType<typeof setTimeout>;
    private generation = 0;
    private ready = false;
    private busy = false;
    private disposed = false;
    private manifest?: StageManifest;

    constructor(readonly panel: vscode.WebviewPanel, private document: vscode.TextDocument,
        private readonly caret: vscode.Position, private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly projects: BuiltProjects) {
        this.primary = document.uri;
        this.cacheKey = `ferroui.stages:${this.primary.toString()}`;
        this.manifest = context.workspaceState.get<StageManifest>(this.cacheKey);
        this.subscriptions = [
            panel.onDidDispose(() => this.dispose()),
            panel.onDidChangeViewState(() => {
                if (panel.visible && this.modelGeneration !== this.generation) this.schedule();
            }),
            panel.webview.onDidReceiveMessage((message: StagesRequest) => { void this.receive(message); }),
            projects.onDidChange(() => this.schedule()),
            vscode.workspace.onDidChangeTextDocument(event => {
                const language = languageOf(event.document);
                if (!language || !event.contentChanges.length) return;
                this.fileChanged(event.document.uri);
            }),
            vscode.workspace.onDidCloseTextDocument(closed => { if (closed.isDirty) this.fileChanged(closed.uri); }),
        ];
        panel.webview.options = webviewOptions(context.extensionUri);
        panel.webview.html = stagesHtml(panel.webview, context.extensionUri);
    }

    get uri(): vscode.Uri { return this.primary; }

    followMove(move: StageFileMove): void {
        const primary = sameStageSource(this.primary, move.oldUri);
        if (!primary && !this.busy && !this.usesSource(move.oldUri)) return;
        if (primary) {
            this.primary = move.newUri;
            this.panel.title = `Stages · ${path.basename(this.primary.fsPath)}`;
        }
        this.sourceMoves.push(move);
        this.schedule();
    }

    private usesSource(uri: vscode.Uri): boolean {
        return stageUsesSource(this.manifest ?? this.nativeManifest, uri, this.sourceMoves);
    }

    fileChanged(uri: vscode.Uri): void {
        if (this.dependencies.change(uri) || sameStageSource(this.primary, uri) || this.usesSource(uri)) this.schedule();
    }

    async wait(signal: AbortSignal): Promise<StageModel> {
        const timeout = AbortSignal.any([signal, AbortSignal.timeout(15_000)]);
        for (;;) {
            timeout.throwIfAborted();
            if (this.disposed) throw new Error('The Stages board was closed.');
            if (!this.busy && !this.timer && this.model && this.modelGeneration === this.generation && this.modelVersion === this.document.version) return this.model;
            if (!this.busy && !this.timer) await this.sync();
            await delay(80, undefined, { signal: timeout });
        }
    }

    capture(id: number, epoch: number, signal: AbortSignal) {
        return this.readStage({ id, epoch }, signal, (client, stage) => client.stageImage(stage));
    }

    hotspots(id: number, epoch: number, signal: AbortSignal) {
        return this.readStage({ id, epoch }, signal, (client, stage) => client.stagePrototypeElements(stage));
    }

    checks(id: number, epoch: number, signal: AbortSignal) {
        return this.readStage({ id, epoch }, signal, (client, stage) => client.stageChecks(stage));
    }

    stateOptions(id: number, epoch: number, signal: AbortSignal) {
        return this.readStage({ id, epoch }, signal, (client, stage) => client.stageStateOptions(stage));
    }

    fixtureOptions(id: number, epoch: number, signal: AbortSignal) {
        return this.readStage({ id, epoch }, signal, (client, stage) => client.stageFixtureOptions(stage));
    }

    async keyboard(target: StageKeyboard, signal: AbortSignal) {
        const model = await this.wait(signal);
        return this.applyKeyboard(await this.getClient(), model, target, signal);
    }

    private async applyKeyboard(client: NativePreview, model: StageModel, target: StageKeyboard, signal: AbortSignal) {
        signal.throwIfAborted();
        const stage = this.currentInputStage(model, target);
        // Pointer and keyboard requests enter the same native stdio queue. A
        // click sent just before a key can establish the native focus target.
        const focus = await client.stageKeyboard(target);
        this.currentInputStage(model, target);
        this.post({ type: 'keyboardFocus', id: stage.id, epoch: stage.epoch, focus });
        this.scheduleInputImage(client, model, stage);
        signal.throwIfAborted();
        return focus;
    }

    private currentInputStage(model: StageModel, target: StageTarget): StageInfo {
        if (this.busy || this.timer || this.disposed || this.model !== model
            || this.modelGeneration !== this.generation || this.modelVersion !== this.document.version) {
            throw new Error('The stage changed. Use its current frame.');
        }
        const stage = model.stages.find(entry => entry.id === target.id && entry.epoch === target.epoch);
        if (!stage) throw new Error('Read the current stage ID and epoch.');
        return stage;
    }

    private scheduleInputImage(client: NativePreview, model: StageModel, stage: StageInfo): void {
        if (this.inputImages.has(stage.id)) return;
        this.inputImages.set(stage.id, setTimeout(() => {
            this.inputImages.delete(stage.id);
            if (this.disposed || this.model !== model || this.modelGeneration !== this.generation) return;
            void this.image(client, stage).catch(error => {
                if (this.model === model && this.modelGeneration === this.generation) this.report(error);
            });
        }, 32));
    }

    private async readStage<T>(target: StageTarget, signal: AbortSignal, read: (client: NativePreview, target: StageTarget) => Promise<T>): Promise<T> {
        const model = await this.wait(signal);
        if (!model.stages.some(stage => stage.id === target.id && stage.epoch === target.epoch)) throw new Error('Read the current stage ID and epoch.');
        const generation = this.generation;
        const value = await read(await this.getClient(), target);
        signal.throwIfAborted();
        if (this.model !== model || generation !== this.generation || this.modelVersion !== this.document.version || this.disposed) {
            throw new Error('The stage changed. Open its current frame.');
        }
        return value;
    }

    async applyChange(change: StageChange, signal: AbortSignal): Promise<StageModel> {
        const before = await this.wait(signal);
        if (before.epoch !== change.epoch || this.busy) throw new Error('Read the current Stages board first.');
        const generation = this.generation;
        this.busy = true;
        this.dependencies.start();
        try {
            const client = await this.getClient();
            const request = await this.nativeChange(change, client);
            signal.throwIfAborted();
            if (generation !== this.generation || this.disposed) throw new Error('The stage set changed. Repeat the change on its current board.');
            const model = await client.editStages(request);
            this.nativeManifest = model.manifest;
            if (this.dependencies.accept(model.dependencies ?? [])) this.schedule();
            await this.publish(model, client, generation);
            return model;
        } finally {
            this.dependencies.stop();
            this.busy = false;
            if (generation !== this.generation) this.schedule();
        }
    }

    private async nativeChange(change: StageChange, client: NativePreview): Promise<NativeStageChange> {
        if (change.action !== 'add') return change;
        const { action, epoch, name, width, height, dark, sourcePath } = change;
        if (sourcePath === undefined) return { action, epoch, name, width, height, dark };
        if (typeof sourcePath !== 'string') throw new Error('Choose a source file inside the workspace.');
        const document = await vscode.workspace.openTextDocument(await resolveWorkspaceFile(sourcePath));
        const language = languageOf(document);
        if (!language) throw new Error('Choose a FerroUI XAML or Rust file.');
        const caret = vscode.window.visibleTextEditors.find(editor => editor.document === document)?.selection.active
            ?? new vscode.Position(0, 0);
        const source = await client.inProject(sourceOf(document, language, caret), this.projects.forDocument(document.uri));
        const localSources = client.capabilities.stageIncludes ? dirtyXamlSources([document.uri]) : undefined;
        return { action, epoch, name, width, height, dark, source: stageDocument(source, document.uri), localSources };
    }

    async controlPrototype(epoch: number, command: PrototypeCommand, signal: AbortSignal) {
        const model = await this.wait(signal);
        const generation = this.generation;
        if (model.epoch !== epoch) throw new Error('Read the current stage set before controlling its flow.');
        const scene = await (await this.getClient()).stagePrototype(epoch, command);
        if (generation !== this.generation) throw new Error('The stage set changed during playback.');
        model.prototype = scene;
        this.post({ type: 'prototype', scene });
        return scene;
    }

    private schedule(): void {
        this.generation++;
        if (this.model?.keyboardInput) this.post({ type: 'keyboardUnavailable' });
        clearTimeout(this.timer);
        this.timer = undefined;
        if (this.ready && this.panel.visible && !this.disposed) this.timer = setTimeout(() => { this.timer = undefined; void this.sync(); }, 170);
    }

    private async sync(): Promise<void> {
        if (this.busy || this.disposed) return;
        const generation = this.generation;
        const uri = this.primary;
        this.busy = true;
        this.dependencies.start();
        try {
            const client = await this.getClient();
            const retargeted = this.document.uri.toString() !== uri.toString();
            if (retargeted && !await isWorkspaceFile(uri)) throw new Error('Open the moved stage source inside its workspace.');
            const document = !retargeted && !this.document.isClosed
                ? this.document : await vscode.workspace.openTextDocument(uri);
            if (generation !== this.generation || this.disposed) return;
            const language = languageOf(document);
            if (!language) throw new Error('Open a FerroUI XAML or Rust source for this Stages board.');
            const pending = this.manifest;
            const moveCount = this.sourceMoves.length;
            const { manifest, bindings } = retargetStageManifest(pending ?? this.nativeManifest, this.sourceMoves);
            const replaceBoard = pending !== undefined || (bindings !== undefined && !client.capabilities.stageSourceMoves);
            const source = await client.inProject(sourceOf(document, language, document.validatePosition(this.caret)),
                this.projects.forDocument(uri));
            const active = stageDocument(source, uri);
            const additional = await this.stageSources(manifest, active.uri, client);
            const localSources = client.capabilities.stageIncludes
                ? dirtyXamlSources([uri, ...additional.map(source => vscode.Uri.parse(source.uri, true))]) : undefined;
            if (generation !== this.generation || this.disposed) return;
            const model = await client.syncStages(active, replaceBoard ? manifest : undefined, additional, localSources,
                replaceBoard ? undefined : bindings);
            this.document = document;
            this.nativeManifest = model.manifest;
            this.sourceMoves.splice(0, moveCount);
            if (pending === this.manifest) this.manifest = undefined;
            if (this.dependencies.accept(model.dependencies ?? [])) this.schedule();
            await this.publish(model, client, generation);
        } catch (error) { if (generation === this.generation) this.report(error); }
        finally {
            this.dependencies.stop();
            this.busy = false;
            if (generation !== this.generation) this.schedule();
        }
    }

    private async publish(model: StageModel, client: NativePreview, generation: number): Promise<void> {
        if (generation !== this.generation || this.disposed) return;
        model.keyboardInput = client.capabilities.stageKeyboard === true;
        this.model = model;
        this.modelVersion = this.document.version;
        this.modelGeneration = generation;
        const key = `ferroui.stages:${this.document.uri.toString()}`;
        await this.context.workspaceState.update(key, model.manifest);
        if (this.cacheKey !== key) {
            await this.context.workspaceState.update(this.cacheKey, undefined);
            this.cacheKey = key;
        }
        if (generation !== this.generation || this.model !== model || this.disposed) return;
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
        if (this.modelGeneration === this.generation && this.model?.stages.some(entry => entry.id === stage.id && entry.epoch === stage.epoch)) {
            this.post({ type: 'image', ...target, image });
        }
    }

    private async receive(message: StagesRequest): Promise<void> {
        if (message.type === 'ready') { this.ready = true; this.schedule(); return; }
        if (this.busy || this.timer || !this.model || this.disposed || this.modelGeneration !== this.generation || this.modelVersion !== this.document.version) return;
        const model = this.model;
        const generation = this.generation;
        try {
            const client = await this.getClient();
            if (message.type === 'prototype') {
                const command = message.command ?? await this.chooseFlow(model);
                if (command && generation === this.generation) {
                    await this.controlPrototype(model.epoch, command, AbortSignal.timeout(15_000));
                }
                return;
            }
            if (message.type === 'editPrototype' || message.type === 'revealTrigger') {
                if (message.epoch !== model.epoch) throw new Error('The prototype changed. Use its current graph.');
                if (message.type === 'revealTrigger') {
                    await this.revealPrototypeTrigger(model, message.index, generation);
                } else {
                    const change = await this.editPrototype(model, message.edit);
                    if (change && generation === this.generation && model === this.model) {
                        await this.applyChange(change, AbortSignal.timeout(15_000));
                    }
                }
                return;
            }
            if (message.type === 'keyboard') {
                await this.applyKeyboard(client, model, message, AbortSignal.timeout(15_000));
                return;
            }
            if (message.type === 'pointer') {
                const stage = this.currentInputStage(model, message);
                const result = await client.stagePointer(message);
                this.currentInputStage(model, message);
                if (result.prototype) {
                    model.prototype = result.prototype;
                    this.post({ type: 'prototype', scene: result.prototype });
                }
                if (result.range && result.uri) {
                    await this.revealStageSource(result.uri, result.range, generation);
                }
                if (result.interacted) this.scheduleInputImage(client, model, stage);
                return;
            }
            if (message.type === 'checks') {
                const { uri, findings } = await this.checks(message.id, message.epoch, AbortSignal.timeout(15_000));
                if (!findings.length) { void vscode.window.showInformationMessage('No findings in this stage.'); return; }
                const issue = await choose('Stage Preview Checks', findings.map(finding => ({
                    label: `$(${finding.code === 'ineffectiveLayoutValue' ? 'info' : 'warning'}) ${finding.summary ?? finding.message}`,
                    description: finding.name ?? finding.control,
                    detail: finding.summary ? finding.message : undefined,
                    range: finding.range,
                })));
                if (issue?.range) await this.revealStageSource(uri, issue.range, generation);
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
            await this.applyChange(change, AbortSignal.timeout(15_000));
        } catch (error) { this.report(error); }
    }

    private async revealStageSource(source: string, selection: Range, generation: number): Promise<void> {
        const uri = vscode.Uri.parse(source, true);
        if (uri.toString() !== this.document.uri.toString() && !await isWorkspaceFile(uri)) return;
        const document = await vscode.workspace.openTextDocument(uri);
        if (generation !== this.generation || this.disposed) return;
        const editor = vscode.window.visibleTextEditors.find(editor => editor.document === document);
        const sourceGroup = vscode.window.tabGroups.all.find(group => group.viewColumn !== this.panel.viewColumn
            && group.tabs.some(tab => tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === this.document.uri.toString()));
        await vscode.window.showTextDocument(document, { viewColumn: editor?.viewColumn ?? sourceGroup?.viewColumn ?? vscode.ViewColumn.Beside,
            preserveFocus: true, selection: asRange(selection) });
    }

    private async editPrototype(model: StageModel, edit: PrototypeEdit): Promise<StageChange | undefined> {
        const graph = await editPrototype(model, stage => this.hotspots(stage.id, stage.epoch, AbortSignal.timeout(15_000)), edit);
        if (graph) return { action: 'setPrototype', graph, epoch: model.epoch };
    }

    private async revealPrototypeTrigger(model: StageModel, index: number, generation: number): Promise<void> {
        const link = model.manifest.prototype?.links[index];
        const stage = link && model.stages[link.source];
        if (!stage) throw new Error('Choose an interaction with a current source stage.');
        const key = JSON.stringify(link.trigger);
        const trigger = (await this.hotspots(stage.id, stage.epoch, AbortSignal.timeout(15_000)))
            .find(candidate => JSON.stringify(candidate.anchor) === key);
        if (!trigger?.uri || !trigger.range) throw new Error('The trigger is unavailable in the current native preview.');
        await this.revealStageSource(trigger.uri, trigger.range, generation);
    }

    private async stageSources(manifest: StageManifest | undefined, active: string, client?: NativePreview) {
        const paths = [...new Set((manifest?.stages ?? []).map(stage => stage.source_uri))].filter(uri => uri !== active);
        if (paths.length > 15) throw new Error('A Stages board supports up to 16 source documents.');
        return Promise.all(paths.map(async path => {
            const uri = vscode.Uri.parse(path, true);
            if (!await isWorkspaceFile(uri)) throw new Error('Open a stage set whose source files are inside this workspace.');
            const document = await vscode.workspace.openTextDocument(uri);
            const language = languageOf(document);
            if (!language) throw new Error('A stage source must be a FerroUI XAML or Rust file.');
            const source = { ...sourceOf(document, language, new vscode.Position(0, 0)), uri: path };
            return stageDocument(client ? await client.inProject(source, this.projects.forDocument(uri)) : source, uri);
        }));
    }

    private async chooseFlow(model: StageModel) {
        const flows = model.manifest.prototype?.flows ?? [];
        if (!flows.length) throw new Error('Add a flow with Edit Prototype first.');
        const flow = await choose('Prototype flow', flows.map(flow => ({ label: flow.name, description: model.stages[flow.start]?.configuration.name })));
        return flow ? { action: 'start' as const, flow: flow.label } : undefined;
    }

    private async add(model: StageModel): Promise<StageChange | undefined> {
        const source = await choose('Stage source', [{ label: 'Current design', other: false }, { label: 'Another workspace file…', other: true }]);
        if (!source) return;
        let sourcePath: string | undefined;
        if (source.other) {
            const [uri] = await vscode.window.showOpenDialog({ canSelectMany: false, filters: { 'FerroUI source': ['xaml', 'rs'] } }) ?? [];
            if (!uri) return;
            sourcePath = uri.fsPath;
        }
        const name = await editText('Stage name', 'Comparison');
        if (!name) return;
        const bounds = await dimensions(800, 520);
        if (bounds) return { action: 'add', epoch: model.epoch, name, ...bounds, dark: false, sourcePath };
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
            case 'designData': {
                if (!client.capabilities.designFixtures) {
                    configuration.design_time_data = !configuration.design_time_data;
                    break;
                }
                const fixtures = await client.stageFixtureOptions({ id, epoch });
                const choice = await choose('Stage design data', [
                    { label: 'Application values', enabled: false, fixture: undefined },
                    { label: 'View default', enabled: true, fixture: undefined },
                    ...fixtures.map(fixture => ({ label: fixture, enabled: true, fixture })),
                ]);
                if (!choice) return;
                configuration.design_time_data = choice.enabled;
                if (choice.enabled) configuration.design_fixture = choice.fixture;
                break;
            }
            case 'state': {
                const states = await chooseStageState(await client.stageStateOptions({ id, epoch }), configuration.states);
                if (!states) return;
                configuration.states = states;
                break;
            }
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
        this.inputImages.forEach(timer => clearTimeout(timer));
        this.inputImages.clear();
        void this.client?.then(client => client.dispose(), () => {});
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

function stageDocument(source: Source, uri: vscode.Uri): StageDocument {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    return { ...source, workspaceRoot: folder?.uri.scheme === 'file' ? folder.uri.fsPath : undefined };
}

async function dimensions(width: number, height: number): Promise<{ width: number; height: number } | undefined> {
    const value = await editText('Logical size, for example 800 × 520', `${width} × ${height}`);
    if (value === undefined) return;
    const parts = /^\s*(\d{1,4})\s*[x×]\s*(\d{1,4})\s*$/i.exec(value);
    if (!parts || parts.slice(1).some(part => Number(part) < 64 || Number(part) > 2048)) throw new Error('Choose dimensions from 64 to 2048.');
    return { width: Number(parts[1]), height: Number(parts[2]) };
}

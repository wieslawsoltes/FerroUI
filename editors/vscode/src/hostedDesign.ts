import { randomBytes } from 'node:crypto';
import * as path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import type { BuiltProject } from './builtProjects';
import { type CargoSelection, type CargoTarget, discoverCargoTargets, selectCargoTarget } from './cargoTarget';
import type { HostedAnimationControl, HostedAnimationFrame, HostedAnimationRequest } from './hostedAnimationProtocol';
import { HostedAnimations } from './hostedAnimations';
import type { FromHosted, HostedAnimationSample, HostedFixtureCatalog, HostedFrame, ToHosted } from './hostedProtocol';
import { NativePreview } from './nativePreview';
import { errorMessage, type Language } from './protocol';
import { buildCargo, launchArtifact, type ProjectMetadata, type ReloadProcess } from './reloadProcess';
import { languageOf, sourceOf } from './textDocument';
import { hostedHtml, webviewOptions } from './view';
import { isInside, isWorkspaceFile } from './workspacePaths';

export class HostedDesign implements vscode.Disposable {
    private readonly command: vscode.Disposable;
    private readonly sessions = new Map<string, HostedSession>();
    private readonly retiring = new Set<Promise<void>>();
    private nextSession = 0;
    private stopped?: Promise<void>;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly built: (metadata: ProjectMetadata) => void) {
        this.command = vscode.commands.registerCommand('ferroui.openHostedDesign', async () => {
            try {
                const editor = vscode.window.activeTextEditor;
                const document = editor?.document;
                const language = document && languageOf(document);
                if (!editor || !document || !language || !vscode.workspace.isTrusted || !await isWorkspaceFile(document.uri)) {
                    throw new Error('Open a FerroUI source file from a trusted Cargo workspace.');
                }
                const current = this.sessions.get(document.uri.toString());
                if (current) { current.panel.reveal(); return; }
                const target = await selectCargoTarget('Select a Cargo binary that supports --ferroui-preview');
                if (target) this.mount(editor, language, target);
            } catch (error) {
                void vscode.window.showErrorMessage(errorMessage(error));
            }
        });
    }

    async open(document: vscode.TextDocument, selection: CargoSelection, signal: AbortSignal): Promise<HostedFixtureCatalog> {
        signal.throwIfAborted();
        const existing = this.sessions.get(document.uri.toString());
        if (existing) {
            existing.panel.reveal(existing.panel.viewColumn, true);
            return existing.fixtures();
        }
        const language = languageOf(document);
        if (!language || !await isWorkspaceFile(document.uri)) throw new Error('Open a FerroUI source file in the Cargo package.');
        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        const targets = await discoverCargoTargets({ ...selection, folder: selection.folder ?? folder?.uri.toString() });
        const local = targets.filter(target => isInside(path.dirname(target.manifest), document.uri.fsPath));
        if (local.length !== 1) throw new Error('Choose one configured Cargo target for the source document.');
        signal.throwIfAborted();
        const editor = vscode.window.visibleTextEditors.find(editor => editor.document === document)
            ?? await vscode.window.showTextDocument(document, { preview: false });
        signal.throwIfAborted();
        return this.mount(editor, language, local[0]).fixtures();
    }

    private mount(editor: vscode.TextEditor, language: Language, target: CargoTarget): HostedSession {
        const document = editor.document;
        if (document.isClosed || this.stopped) throw new Error('The source editor was closed.');
        if (!isInside(path.dirname(target.manifest), document.uri.fsPath)) throw new Error('Select the Cargo package containing this source file.');
        const key = document.uri.toString();
        const existing = this.sessions.get(key);
        if (existing) { existing.panel.reveal(); return existing; }
        const panel = vscode.window.createWebviewPanel('ferroui.hostedDesign', `App Preview · ${path.basename(document.fileName)}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
        const session = new HostedSession(this.context, this.output, panel, editor, language, target, ++this.nextSession, this.built, closing => {
            this.sessions.delete(key);
            this.retiring.add(closing);
            void closing.then(() => this.retiring.delete(closing));
        });
        this.sessions.set(key, session);
        return session;
    }

    fixtures(uri: vscode.Uri): HostedFixtureCatalog {
        return this.session(uri).fixtures();
    }

    animations(uri: vscode.Uri) {
        const session = this.session(uri);
        return { generation: session.generation, status: session.message, ...session.animations.state };
    }

    image(uri: vscode.Uri) {
        const session = this.session(uri);
        const frame = session.currentFrame;
        if (!frame) throw new Error('Wait for the current application preview.');
        const { image: { png, width, height }, revision, sequence } = frame;
        return { mimeType: 'image/png', data: png, width, height, revision, sequence, generation: session.generation, uri: uri.toString() };
    }

    async animate(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame,
        control: HostedAnimationControl | { action: 'reveal'; program: number }, signal: AbortSignal) {
        signal.throwIfAborted();
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        if (control.action === 'reveal') await session.revealAnimation(frame, control.program);
        else {
            session.panel.reveal(session.panel.viewColumn, true);
            await session.animations.control(frame, control, signal);
        }
        return this.animations(uri);
    }

    async chooseFixture(uri: vscode.Uri, generation: number, revision: number, name: string | null, signal: AbortSignal): Promise<HostedFixtureCatalog> {
        signal.throwIfAborted();
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        session.panel.reveal(session.panel.viewColumn, true);
        const requested = session.selectFixture(revision, name);
        return session.waitForFixture(requested, signal);
    }

    private session(uri: vscode.Uri): HostedSession {
        const session = this.sessions.get(uri.toString());
        if (!session) throw new Error('Open the application preview for the current design.');
        return session;
    }

    dispose(): Promise<void> {
        if (this.stopped) return this.stopped;
        this.command.dispose();
        const sessions = [...this.sessions.values()];
        this.sessions.clear();
        this.stopped = Promise.all([...this.retiring, ...sessions.map(session => session.dispose())]).then(() => undefined);
        for (const session of sessions) session.panel.dispose();
        return this.stopped;
    }
}

class HostedSession {
    readonly animations: HostedAnimations;
    private readonly subscriptions: vscode.Disposable[];
    private readonly startup: Promise<void>;
    private compiler?: ReloadProcess;
    private application?: ReloadProcess;
    private retiring?: Promise<void>;
    private stopped?: Promise<void>;
    private client?: NativePreview;
    private project?: BuiltProject;
    private frame?: HostedFrame;
    private fixture: string | null = null;
    private failed?: { revision: number; message: string };
    private timer?: ReturnType<typeof setTimeout>;
    private revision = 1;
    private requested = 0;
    private pending = false;
    private ready = false;
    private disposed = false;
    private status = 'Building the selected Cargo target…';

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        readonly panel: vscode.WebviewPanel, private readonly editor: vscode.TextEditor, private readonly language: Language,
        private readonly target: CargoTarget, readonly generation: number, built: (metadata: ProjectMetadata) => void, remove: (closing: Promise<void>) => void) {
        this.animations = new HostedAnimations((request, signal) => this.sampleAnimation(request, signal),
            state => this.post({ type: 'animations', state }), error => this.report(errorMessage(error), Boolean(this.currentFrame)));
        this.subscriptions = [
            vscode.workspace.onDidChangeTextDocument(event => { if (event.document === editor.document) this.update(); }),
            vscode.window.onDidChangeTextEditorSelection(event => {
                if (language === 'rust' && event.textEditor === editor) this.update();
            }),
            panel.onDidChangeViewState(() => this.schedule(0)),
            panel.onDidDispose(() => remove(this.dispose())),
            panel.webview.onDidReceiveMessage((message: FromHosted) => { void this.receive(message); }),
        ];
        panel.webview.html = hostedHtml(panel.webview, context.extensionUri);
        this.startup = this.start(built);
    }

    private async start(built: (metadata: ProjectMetadata) => void): Promise<void> {
        try {
            const build = buildCargo(this.target, this.output);
            this.compiler = build.process;
            const artifact = await build.artifact;
            if (this.disposed) return;
            this.compiler = undefined;
            const metadata = build.metadata();
            this.project = { metadata, workspaceRoot: this.target.folder.uri.fsPath };
            built(metadata);
            const client = await NativePreview.start(this.context, this.output);
            if (this.disposed) { client.dispose(); return; }
            this.client = client;
            const nonce = randomBytes(32).toString('hex');
            const { port, animationQuery } = await client.listenHosted(nonce);
            if (this.disposed) return;
            const endpoint = new URL(`tcp-bson://127.0.0.1:${port}/`);
            if (animationQuery) endpoint.search = animationQuery;
            const args = ['--ferroui-preview', '--transport', endpoint.toString(), '--session-id', nonce,
                '--method', 'ferroui-remote', artifact];
            const launch = await launchArtifact(artifact, { ...this.target, args }, this.output, () => {}, () => {});
            if (this.disposed) { await launch.process.stop(); return; }
            this.application = launch.process;
            void launch.process.closed.then(code => this.ended(`The design host stopped (${code ?? 'terminated'}).`), error => this.ended(errorMessage(error)));
            this.report('Waiting for the application preview…');
            this.schedule(0);
        } catch (error) { this.ended(errorMessage(error)); }
    }

    private ended(message: string) {
        if (this.disposed) return;
        this.client?.dispose();
        this.client = undefined;
        this.retiring = this.application?.stop();
        this.application = undefined;
        this.animations.invalidate();
        this.report(message);
    }

    private update(): void {
        ++this.revision;
        this.failed = undefined;
        this.animations.invalidate();
        if (!this.application) return;
        this.report('Rendering the current source…');
        this.schedule(180);
    }

    private schedule(delay: number): void {
        clearTimeout(this.timer);
        if (this.ready && this.panel.visible && this.client && this.application && !this.disposed) {
            this.timer = setTimeout(() => { void this.tick(); }, delay);
        }
    }

    private async tick(): Promise<void> {
        const client = this.client;
        if (!client || this.pending || this.animations.driving) return;
        const revision = this.revision;
        this.pending = true;
        try {
            const document = this.editor.document;
            if (document.isClosed) { this.panel.dispose(); return; }
            const update = revision !== this.requested;
            this.requested = revision;
            const assembly = vscode.workspace.getConfiguration('ferroui', document.uri)
                .get<string>('hostedPreview.assembly') || this.target.packageName;
            const projectPath = '/' + path.relative(path.dirname(this.target.manifest), document.uri.fsPath).split(path.sep).map(encodeURIComponent).join('/');
            let frame: HostedFrame | null;
            if (update) {
                const source = await client.inProject(sourceOf(document, this.language, this.editor.selection.active), this.project);
                if (this.disposed || revision !== this.revision) return;
                frame = await client.renderHosted({ revision, assembly, projectPath, document: source, fixture: this.fixture });
            } else frame = await client.hostedFrame();
            if (!this.disposed && client === this.client && frame?.revision === this.revision) {
                this.accept(frame);
            }
        } catch (error) {
            if (client === this.client && revision === this.revision) {
                const message = errorMessage(error);
                this.failed = { revision, message };
                this.report(message);
            }
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    get currentFrame(): HostedFrame | undefined {
        if (this.client && this.frame?.revision === this.revision) return this.frame;
    }

    get message(): string { return this.status; }

    private accept(frame: HostedFrame): void {
        this.frame = frame;
        this.post({ type: 'frame', frame });
        this.animations.accept(frame);
        this.report(`${this.target.label} · App-hosted design${frame.fixture ? ` · ${frame.fixture}` : ''}`, true);
    }

    private async sampleAnimation(request: HostedAnimationRequest, signal: AbortSignal): Promise<HostedAnimationSample> {
        const client = this.client;
        signal.throwIfAborted();
        if (!client || this.pending || request.revision !== this.revision || request.sequence !== this.currentFrame?.sequence) {
            throw new Error('Wait for the current application preview frame.');
        }
        clearTimeout(this.timer);
        this.pending = true;
        try {
            const sample = await client.sampleHostedAnimation(request);
            if (this.disposed || client !== this.client || sample.frame.revision !== this.revision) {
                throw new Error('The application preview changed while sampling the animation.');
            }
            this.accept(sample.frame);
            return sample;
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    async revealAnimation(frame: HostedAnimationFrame, id: number): Promise<void> {
        if (this.language !== 'xaml') throw new Error('The application animation has no mapped Rust declaration.');
        const program = this.animations.program(frame, id);
        const document = this.editor.document;
        const position = document.positionAt(program.sourceOffset);
        const editor = await vscode.window.showTextDocument(document, { viewColumn: this.editor.viewColumn });
        if (frame.revision !== this.revision) throw new Error('The source changed before revealing the animation.');
        editor.selection = new vscode.Selection(position, position);
        editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
    }

    private async chooseAnimationTargets(frame: HostedAnimationFrame, id: number): Promise<void> {
        const program = this.animations.program(frame, id);
        const state = this.animations.state;
        const chosen = state.overrides[id] ?? program.defaultTargets;
        const controls = state.targets.filter(target => program.compatibleTargets.includes(target.id));
        const choice = await vscode.window.showQuickPick(controls.map(target => ({ label: target.label,
            description: target.label === target.typeName ? undefined : target.typeName, target: target.id, picked: chosen.includes(target.id) })), {
            canPickMany: true, title: `Preview targets · ${program.name}`, placeHolder: 'Choose compatible controls',
        });
        if (!choice) return;
        const current = this.animations.state.frame;
        if (!current || current.revision !== frame.revision) throw new Error('The application source changed while choosing targets.');
        await this.animations.control(current, { action: 'targets', program: id, targets: choice.map(item => item.target) }, AbortSignal.timeout(15_000));
    }

    private async receive(message: FromHosted): Promise<void> {
        if (message.type === 'ready') {
            this.ready = true;
            if (this.frame) this.post({ type: 'frame', frame: this.frame });
            this.post({ type: 'animations', state: this.animations.state });
            this.report(this.status, this.frame?.revision === this.revision && Boolean(this.client));
            this.schedule(0);
            return;
        }
        if (message.type === 'refresh') { this.update(); return; }
        if (message.type === 'animation' || message.type === 'animationTargets' || message.type === 'animationSource') {
            try {
                if (message.type === 'animation') await this.animations.control(message.frame, message.control, AbortSignal.timeout(15_000));
                else if (message.type === 'animationTargets') await this.chooseAnimationTargets(message.frame, message.program);
                else await this.revealAnimation(message.frame, message.program);
            } catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            if (message.type === 'animation') this.post({ type: 'animationReply', request: message.request });
            return;
        }
        if (message.type === 'fixture') {
            try { this.selectFixture(message.revision, message.name); }
            catch (error) { this.report(errorMessage(error), this.frame?.revision === this.revision); }
            return;
        }
        if (!('revision' in message) || message.revision !== this.revision || message.sequence !== this.frame?.sequence) return;
        try { await this.client?.hostedInput(message); }
        catch (error) { this.report(errorMessage(error)); }
    }

    private report(message: string, interactive = false): void {
        this.status = message;
        this.post({ type: 'status', message, interactive, fixtures: this.fixtures() });
    }

    fixtures(): HostedFixtureCatalog {
        return { generation: this.generation, revision: this.revision, available: this.frame?.fixtures ?? [],
            selected: this.frame?.fixture ?? null, pending: this.failed?.revision !== this.revision && this.frame?.revision !== this.revision };
    }

    selectFixture(revision: number, name: string | null): number {
        const catalog = this.fixtures();
        if (revision !== catalog.revision || !this.client || !this.application || catalog.pending) {
            throw new Error('Read the current application preview fixture choices.');
        }
        if (name !== null && !catalog.available.includes(name)) throw new Error('Choose a registered fixture for this source.');
        if (this.frame?.revision === revision && name === catalog.selected) return revision;
        this.fixture = name;
        this.update();
        return this.revision;
    }

    async waitForFixture(revision: number, signal: AbortSignal): Promise<HostedFixtureCatalog> {
        const timeout = AbortSignal.any([signal, AbortSignal.timeout(15_000)]);
        while (!this.disposed && this.client && this.revision === revision) {
            timeout.throwIfAborted();
            if (this.failed?.revision === revision) throw new Error(this.failed.message);
            if (this.frame?.revision === revision) return this.fixtures();
            await delay(60, undefined, { signal: timeout });
        }
        throw new Error('The application preview changed before the fixture was accepted.');
    }

    private post(message: ToHosted): void {
        if (this.ready && !this.disposed) void this.panel.webview.postMessage(message);
    }

    dispose(): Promise<void> {
        if (this.stopped) return this.stopped;
        this.disposed = true;
        clearTimeout(this.timer);
        this.animations.invalidate();
        this.client?.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.stopped = Promise.all([this.startup, this.compiler?.stop(), this.application?.stop(), this.retiring]).then(() => undefined);
        return this.stopped;
    }
}

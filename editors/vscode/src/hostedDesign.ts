import { randomBytes } from 'node:crypto';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { BuiltProject } from './builtProjects';
import { type CargoTarget, selectCargoTarget } from './cargoTarget';
import type { FromHosted, HostedFrame, ToHosted } from './hostedProtocol';
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
    private stopped?: Promise<void>;

    constructor(context: vscode.ExtensionContext, output: vscode.OutputChannel, built: (metadata: ProjectMetadata) => void) {
        this.command = vscode.commands.registerCommand('ferroui.openHostedDesign', async () => {
            try {
                const editor = vscode.window.activeTextEditor;
                const document = editor?.document;
                const language = document && languageOf(document);
                if (!editor || !document || !language || !vscode.workspace.isTrusted || !await isWorkspaceFile(document.uri)) {
                    throw new Error('Open a FerroUI source file from a trusted Cargo workspace.');
                }
                const key = document.uri.toString();
                const current = this.sessions.get(key);
                if (current) { current.panel.reveal(); return; }
                const target = await selectCargoTarget('Select a Cargo binary that supports --ferroui-preview');
                if (!target || document.isClosed || this.stopped) return;
                const opened = this.sessions.get(key);
                if (opened) { opened.panel.reveal(); return; }
                if (!isInside(path.dirname(target.manifest), document.uri.fsPath)) throw new Error('Select the Cargo package containing this source file.');
                const panel = vscode.window.createWebviewPanel('ferroui.hostedDesign', `App Preview · ${path.basename(document.fileName)}`,
                    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                    { ...webviewOptions(context.extensionUri), retainContextWhenHidden: true });
                const session = new HostedSession(context, output, panel, editor, language, target, built, closing => {
                    this.sessions.delete(key);
                    this.retiring.add(closing);
                    void closing.then(() => this.retiring.delete(closing));
                });
                this.sessions.set(key, session);
            } catch (error) {
                void vscode.window.showErrorMessage(errorMessage(error));
            }
        });
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
    private readonly subscriptions: vscode.Disposable[];
    private readonly startup: Promise<void>;
    private compiler?: ReloadProcess;
    private application?: ReloadProcess;
    private retiring?: Promise<void>;
    private stopped?: Promise<void>;
    private client?: NativePreview;
    private project?: BuiltProject;
    private frame?: HostedFrame;
    private timer?: ReturnType<typeof setTimeout>;
    private revision = 1;
    private requested = 0;
    private pending = false;
    private ready = false;
    private disposed = false;
    private status = 'Building the selected Cargo target…';

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        readonly panel: vscode.WebviewPanel, private readonly editor: vscode.TextEditor, private readonly language: Language,
        private readonly target: CargoTarget, built: (metadata: ProjectMetadata) => void, remove: (closing: Promise<void>) => void) {
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
            const { port } = await client.listenHosted(nonce);
            if (this.disposed) return;
            const args = ['--ferroui-preview', '--transport', `tcp-bson://127.0.0.1:${port}/`, '--session-id', nonce,
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
        this.report(message);
    }

    private update(): void {
        ++this.revision;
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
        if (!client || this.pending) return;
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
            const frame = update ? await client.renderHosted({ revision, assembly, projectPath,
                document: await client.inProject(sourceOf(document, this.language, this.editor.selection.active), this.project),
            }) : await client.hostedFrame();
            if (!this.disposed && client === this.client && frame?.revision === this.revision) {
                this.frame = frame;
                this.post({ type: 'frame', frame });
                this.report(`${this.target.label} · App-hosted design`, true);
            }
        } catch (error) {
            if (client === this.client && revision === this.revision) this.report(errorMessage(error));
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    private async receive(message: FromHosted): Promise<void> {
        if (message.type === 'ready') {
            this.ready = true;
            if (this.frame) this.post({ type: 'frame', frame: this.frame });
            this.report(this.status, this.frame?.revision === this.revision && Boolean(this.client));
            this.schedule(0);
            return;
        }
        if (message.type === 'refresh') { this.update(); return; }
        if (!('revision' in message) || message.revision !== this.revision || message.sequence !== this.frame?.sequence) return;
        try { await this.client?.hostedInput(message); }
        catch (error) { this.report(errorMessage(error)); }
    }

    private report(message: string, interactive = false): void {
        this.status = message;
        this.post({ type: 'status', message, interactive });
    }

    private post(message: ToHosted): void {
        if (this.ready && !this.disposed) void this.panel.webview.postMessage(message);
    }

    dispose(): Promise<void> {
        if (this.stopped) return this.stopped;
        this.disposed = true;
        clearTimeout(this.timer);
        this.client?.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.stopped = Promise.all([this.startup, this.compiler?.stop(), this.application?.stop(), this.retiring]).then(() => undefined);
        return this.stopped;
    }
}

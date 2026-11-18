import { randomBytes } from 'node:crypto';
import * as path from 'node:path';
import * as vscode from 'vscode';
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
                if (!target || document.isClosed) return;
                if (!isInside(path.dirname(target.manifest), document.uri.fsPath)) throw new Error('Select the Cargo package containing this source file.');
                const panel = vscode.window.createWebviewPanel('ferroui.hostedDesign', `App Preview · ${path.basename(document.fileName)}`,
                    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                    { ...webviewOptions(context.extensionUri), retainContextWhenHidden: true });
                const session = new HostedSession(context, output, panel, editor, language, target, built, () => this.sessions.delete(key));
                this.sessions.set(key, session);
            } catch (error) {
                void vscode.window.showErrorMessage(errorMessage(error));
            }
        });
    }

    dispose(): void {
        this.command.dispose();
        for (const session of this.sessions.values()) session.panel.dispose();
    }
}

class HostedSession {
    private readonly subscriptions: vscode.Disposable[];
    private compiler?: ReloadProcess;
    private application?: ReloadProcess;
    private client?: NativePreview;
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
        private readonly target: CargoTarget, built: (metadata: ProjectMetadata) => void, remove: () => void) {
        this.subscriptions = [
            vscode.workspace.onDidChangeTextDocument(event => { if (event.document === editor.document) this.update(); }),
            vscode.window.onDidChangeTextEditorSelection(event => {
                if (language === 'rust' && event.textEditor === editor) this.update();
            }),
            panel.onDidChangeViewState(() => this.schedule(0)),
            panel.onDidDispose(() => { this.dispose(); remove(); }),
            panel.webview.onDidReceiveMessage((message: FromHosted) => { void this.receive(message); }),
        ];
        panel.webview.html = hostedHtml(panel.webview, context.extensionUri);
        void this.start(built);
    }

    private async start(built: (metadata: ProjectMetadata) => void): Promise<void> {
        try {
            const build = buildCargo(this.target, this.output);
            this.compiler = build.process;
            const artifact = await build.artifact;
            if (this.disposed) return;
            this.compiler = undefined;
            built(build.metadata());
            const client = await NativePreview.start(this.context, this.output);
            if (this.disposed) { client.dispose(); return; }
            this.client = client;
            const nonce = randomBytes(32).toString('hex');
            const { port } = await client.listenHosted(nonce);
            if (this.disposed) return;
            const args = ['--ferroui-preview', '--transport', `tcp-bson://127.0.0.1:${port}/`, '--session-id', nonce,
                '--method', 'ferroui-remote', artifact];
            const launch = await launchArtifact(artifact, { ...this.target, args }, this.output, () => {}, () => {});
            if (this.disposed) { launch.process.stop(); return; }
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
        this.application?.stop();
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
                document: sourceOf(document, this.language, this.editor.selection.active) }) : await client.hostedFrame();
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

    private dispose(): void {
        this.disposed = true;
        clearTimeout(this.timer);
        this.compiler?.stop();
        this.application?.stop();
        this.client?.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

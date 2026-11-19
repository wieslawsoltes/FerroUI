import { randomBytes } from 'node:crypto';
import * as path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import type { BuiltProject } from './builtProjects';
import { type CargoSelection, type CargoTarget, discoverCargoTargets, selectCargoTarget } from './cargoTarget';
import type { HostedAnimationAction, HostedAnimationFrame, HostedAnimationRequest, HostedAnimationState, HostedClipDraft, HostedClipEdit, HostedCurveQuery, HostedKeyframeDraft, HostedKeyframeGroupDraft, HostedSourcePlan, HostedStateDraft } from './hostedAnimationProtocol';
import { HostedAnimations } from './hostedAnimations';
import { chooseHostedClipEdit } from './hostedClipActions';
import { chooseHostedStateAction } from './hostedStateActions';
import { chooseHostedKeyframe } from './hostedKeyframes';
import { prepareHostedKeyframeGroup } from './hostedKeyframeGroup';
import type { FromHosted, HostedAnimationSample, HostedFixtureCatalog, HostedFrame, ToHosted } from './hostedProtocol';
import { applyHostedSourcePatches } from './hostedSourceEdit';
import { editHostedStateRoute } from './hostedStateRoutes';
import type { StateGroupEdit } from './stateGroupProtocol';
import { NativePreview } from './nativePreview';
import { errorMessage, type Language } from './protocol';
import { buildCargo, launchArtifact, type ProjectMetadata, type ReloadProcess } from './reloadProcess';
import { chooseAnimationIssue } from './studioPrograms';
import { asRange, languageOf, sourceOf } from './textDocument';
import { writeTimelineClipboard } from './timelineKeyframeClipboard';
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
        control: HostedAnimationAction, signal: AbortSignal) {
        signal.throwIfAborted();
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        if (control.action === 'reveal') await session.revealAnimation(frame, control.program);
        else if (control.action === 'reveal_issue') {
            if (!Number.isSafeInteger(control.index)) throw new Error('Choose a source issue by its index.');
            await session.revealAnimationIssue(frame, control.index);
        }
        else {
            session.panel.reveal(session.panel.viewColumn, true);
            await session.animations.control(frame, control, signal);
        }
        return this.animations(uri);
    }

    keyframe(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, draft: HostedKeyframeDraft,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.keyframe(frame, draft, apply, signal);
    }

    keyframeGroup(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, draft: HostedKeyframeGroupDraft,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.keyframeGroup(frame, draft, apply, signal);
    }

    stateRoute(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, draft: HostedStateDraft,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.stateRoute(frame, draft, apply, signal);
    }

    stateGroup(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, command: StateGroupEdit,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.stateGroup(frame, command, apply, signal);
    }

    clipPlacement(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, draft: HostedClipDraft,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.clipPlacement(frame, draft, apply, signal);
    }

    clipEdit(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, command: HostedClipEdit,
        apply: boolean, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.clipEdit(frame, command, apply, signal);
    }

    curve(uri: vscode.Uri, generation: number, frame: HostedAnimationFrame, query: HostedCurveQuery, signal: AbortSignal) {
        const session = this.session(uri);
        if (session.generation !== generation) throw new Error('Read the current application preview generation.');
        return session.curve(frame, query, signal);
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
    private clipSelection?: { version: number; key: string };
    private failed?: { revision: number; message: string };
    private timer?: ReturnType<typeof setTimeout>;
    private revision = 1;
    private requested = 0;
    private pending = false;
    private polling?: Promise<void>;
    private ready = false;
    private disposed = false;
    private status = 'Building the selected Cargo target…';

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        readonly panel: vscode.WebviewPanel, private readonly editor: vscode.TextEditor, private readonly language: Language,
        private readonly target: CargoTarget, readonly generation: number, built: (metadata: ProjectMetadata) => void, remove: (closing: Promise<void>) => void) {
        this.animations = new HostedAnimations((request, signal) => this.sampleAnimation(request, signal), frame => this.accept(frame),
            state => this.showAnimations(state),
            error => this.report(errorMessage(error), Boolean(this.currentFrame)),
            () => Boolean(this.client?.capabilities.hostedPreviewRanges));
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
        this.animations.beforeSourceUpdate();
        if (!this.application) return;
        this.report('Rendering the current source…');
        this.schedule(180);
    }

    private schedule(delay: number): void {
        clearTimeout(this.timer);
        if (this.ready && this.panel.visible && this.client && this.application && !this.disposed) {
            this.timer = setTimeout(() => {
                this.polling = this.tick().finally(() => { this.polling = undefined; });
            }, delay);
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
                frame = await client.renderHosted({ revision, assembly, projectPath, document: source, fixture: this.fixture,
                    restoreAnimation: client.capabilities.hostedAnimationRestoration ? this.animations.restore : undefined,
                    selectClipKey: this.clipSelection?.version === document.version ? this.clipSelection.key : undefined });
            } else frame = await client.hostedFrame();
            if (!this.disposed && client === this.client && frame?.revision === this.revision) {
                if (update) this.clipSelection = undefined;
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
        await this.polling;
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
            return sample;
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    async revealAnimation(frame: HostedAnimationFrame, id: number): Promise<void> {
        const program = this.animations.program(frame, id);
        if (program.sourceOffset == null) throw new Error('The application animation has no mapped declaration.');
        const document = this.editor.document;
        const position = document.positionAt(program.sourceOffset);
        const range = program.stateRoute ? asRange(program.stateRoute.sourceRange) : new vscode.Range(position, position);
        await this.revealSource(frame, range);
    }

    private async revealSource(frame: HostedAnimationFrame, range: vscode.Range): Promise<void> {
        if (frame.revision !== this.revision) throw new Error('Read the current application preview frame.');
        const editor = await vscode.window.showTextDocument(this.editor.document, { viewColumn: this.editor.viewColumn });
        if (frame.revision !== this.revision) throw new Error('The source changed before revealing the declaration.');
        editor.selection = new vscode.Selection(range.start, range.end);
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
    }

    async revealAnimationIssue(frame: HostedAnimationFrame, index?: number): Promise<void> {
        if (this.currentFrame?.sequence !== frame.sequence || frame.revision !== this.revision) {
            throw new Error('Read the current application preview frame.');
        }
        const issues = this.animations.state.sourceIssues;
        const issue = index === undefined ? await chooseAnimationIssue(issues) : issues?.[index];
        if (index !== undefined && !issue) throw new Error('Choose an unsupported animation from the current source.');
        if (!issue) return;
        const editor = await vscode.window.showTextDocument(this.editor.document, { viewColumn: this.editor.viewColumn });
        if (frame.revision !== this.revision) throw new Error('The source changed before revealing the animation.');
        const { start, end } = issue.range;
        const range = new vscode.Range(start.line, start.character, end.line, end.character);
        editor.selection = new vscode.Selection(range.start, range.end);
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
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

    async keyframe(frame: HostedAnimationFrame, draft: HostedKeyframeDraft, apply: boolean, signal: AbortSignal) {
        await this.polling;
        signal.throwIfAborted();
        const program = this.animations.program(frame, draft.program);
        if (!program.authoring) throw new Error('Choose an editable animation in the current application preview.');
        const client = this.client;
        if (!client || this.pending || this.animations.driving || this.currentFrame?.sequence !== frame.sequence) {
            throw new Error('Wait for the current application preview frame.');
        }
        const document = this.editor.document;
        const version = document.version;
        clearTimeout(this.timer);
        this.pending = true;
        try {
            const plan = await client.planHostedKeyframe(frame, draft);
            signal.throwIfAborted();
            if (client !== this.client || this.disposed || frame.revision !== this.revision || version !== document.version) {
                throw new Error('The application preview source changed while planning the keyframe.');
            }
            if (apply && plan.patch) await applyHostedSourcePatches(document, version, [plan.patch]);
            return { ...plan, applied: apply && Boolean(plan.patch), version: document.version };
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    stateRoute(frame: HostedAnimationFrame, draft: HostedStateDraft, apply: boolean, signal: AbortSignal) {
        return this.editAnimationSource(frame, apply, signal, client => {
            const route = this.animations.program(frame, draft.program).stateRoute;
            if (!route || route.endpoint) {
                throw new Error('Choose an authored state transition in the current application preview.');
            }
            return client.planHostedState(frame, draft);
        });
    }

    stateGroup(frame: HostedAnimationFrame, command: StateGroupEdit, apply: boolean, signal: AbortSignal) {
        return this.editAnimationSource(frame, apply, signal, client => client.planHostedStateGroup(frame, command));
    }

    clipPlacement(frame: HostedAnimationFrame, draft: HostedClipDraft, apply: boolean, signal: AbortSignal) {
        return this.editAnimationSource(frame, apply, signal, client => client.planHostedClipPlacement(frame, draft));
    }

    async clipEdit(frame: HostedAnimationFrame, command: HostedClipEdit, apply: boolean, signal: AbortSignal) {
        const result = await this.editAnimationSource(frame, apply, signal, client => client.planHostedClipEdit(frame, command));
        if (result.applied && result.selectedKey) this.clipSelection = { version: result.version, key: result.selectedKey };
        return result;
    }

    keyframeGroup(frame: HostedAnimationFrame, draft: HostedKeyframeGroupDraft, apply: boolean, signal: AbortSignal) {
        return this.editAnimationSource(frame, apply, signal, client => {
            if (!this.animations.state.keyframeSelection || !this.animations.program(frame, draft.program).authoring) {
                throw new Error('Choose an editable animation in an app with keyframe selection support.');
            }
            return client.planHostedKeyframeGroup(frame, draft);
        });
    }

    private async editAnimationSource<T extends HostedSourcePlan>(frame: HostedAnimationFrame, apply: boolean, signal: AbortSignal,
        planEdit: (client: NativePreview) => Promise<T>) {
        await this.polling;
        signal.throwIfAborted();
        const client = this.client;
        const current = this.currentFrame;
        if (!client || this.pending || this.animations.driving || current?.revision !== frame.revision || current.sequence !== frame.sequence) {
            throw new Error('Wait for the current application preview or pause playback.');
        }
        const document = this.editor.document;
        const version = document.version;
        clearTimeout(this.timer);
        this.pending = true;
        try {
            const plan = await planEdit(client);
            signal.throwIfAborted();
            if (client !== this.client || this.disposed || frame.revision !== this.revision || version !== document.version) {
                throw new Error('The application preview source changed while planning the animation edit.');
            }
            if (apply) await applyHostedSourcePatches(document, version, plan.patches);
            return { ...plan, applied: apply && plan.patches.length > 0, version: document.version };
        } finally {
            this.pending = false;
            this.schedule(this.requested === this.revision ? 160 : 0);
        }
    }

    private async promptStateRoute(frame: HostedAnimationFrame, id: number): Promise<void> {
        const draft = await editHostedStateRoute(this.animations.program(frame, id), this.language === 'rust');
        if (!draft) return;
        const current = this.animations.state.frame;
        if (!current || current.revision !== frame.revision) {
            throw new Error('The application source changed while editing the state route.');
        }
        await this.stateRoute(current, draft, true, AbortSignal.timeout(15_000));
    }

    private async promptStateGroup(frame: HostedAnimationFrame, id?: number): Promise<void> {
        const { stateAuthoring, programs } = this.animations.state;
        if (!stateAuthoring) throw new Error('Use a current app with visual-state authoring support.');
        if (frame.revision !== this.animations.state.frame?.revision) {
            throw new Error('The application source changed. Open Visual state actions again.');
        }
        const action = await chooseHostedStateAction(stateAuthoring, programs, id);
        if (!action) return;
        const current = this.animations.state.frame;
        if (!current || current.revision !== frame.revision || current.revision !== this.revision) {
            throw new Error('The application source changed while choosing a visual state action.');
        }
        const signal = AbortSignal.timeout(15_000);
        if (action.type === 'edit') await this.stateGroup(current, action.command, true, signal);
        else if (action.type === 'preview') await this.animations.control(current, { action: 'seek', program: action.program.id, elapsed: 0 }, signal);
        else await this.revealSource(current, asRange(action.range));
    }

    async curve(frame: HostedAnimationFrame, query: HostedCurveQuery, signal: AbortSignal) {
        await this.polling;
        signal.throwIfAborted();
        const program = this.animations.program(frame, query.program);
        if (!program.curveTracks?.some(track => track.property === query.property
            && (track.curve_channels ?? ['value']).includes(query.channel ?? 'value'))
            || !program.compatibleTargets.includes(query.target)) {
            throw new Error('Choose a curve channel and a compatible live control.');
        }
        const client = this.client;
        if (!client || this.pending || this.animations.driving || this.currentFrame?.sequence !== frame.sequence) {
            throw new Error('Wait for the current application preview or pause playback.');
        }
        const points = await client.hostedCurve(frame, query);
        signal.throwIfAborted();
        if (client !== this.client || this.disposed || frame.revision !== this.revision
            || frame.sequence !== this.currentFrame?.sequence) {
            throw new Error('The application preview changed while sampling the curve.');
        }
        return points;
    }

    private async openKeyframe(frame: HostedAnimationFrame, id: number): Promise<void> {
        const program = this.animations.program(frame, id);
        const choice = await chooseHostedKeyframe(program);
        if (!choice) return;
        const current = this.animations.state.frame;
        if (!current || current.revision !== frame.revision || this.revision !== current.revision) {
            throw new Error('The application source changed while choosing a keyframe.');
        }
        const targetName = this.animations.state.targets.find(target => target.id === choice.target)?.label ?? program.name;
        this.post({ type: 'openKeyframe', key: {
            frame: current, version: this.editor.document.version, program: program.id, targetName,
            target: choice.target, property: choice.label, duration: program.duration, keyframe: choice.keyframe,
        } });
    }

    private showAnimations(state: HostedAnimationState): void {
        this.post({ type: 'animations', version: this.editor.document.version,
            state: { ...state, canShowVelocity: Boolean(this.client?.capabilities.timelineCurveVelocity) } });
    }

    private async receive(message: FromHosted): Promise<void> {
        if (message.type === 'ready') {
            this.ready = true;
            if (this.frame) this.post({ type: 'frame', frame: this.frame });
            this.showAnimations(this.animations.state);
            this.report(this.status, this.frame?.revision === this.revision && Boolean(this.client));
            this.schedule(0);
            return;
        }
        if (message.type === 'refresh') { this.update(); return; }
        if (message.type === 'animationIssues') {
            try { await this.revealAnimationIssue(message.frame); }
            catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            return;
        }
        if (message.type === 'curve') {
            const reply = { type: 'curveReply' as const, request: message.request, frame: message.frame };
            try {
                const points = await this.curve(message.frame, message.query, AbortSignal.timeout(12_000));
                this.post({ ...reply, points });
            } catch (error) { this.post({ ...reply, error: errorMessage(error) }); }
            return;
        }
        if (message.type === 'keyframeDraft') {
            const reply = { type: 'keyframeReply' as const, frame: message.frame, request: message.request };
            try {
                const { valueKind, samples } = await this.keyframe(message.frame, message.draft, message.apply, AbortSignal.timeout(15_000));
                this.post({ ...reply, preview: { valueKind, samples }, applied: message.apply });
            } catch (error) { this.post({ ...reply, error: errorMessage(error) }); }
            return;
        }
        if (message.type === 'keyframeGroup') {
            await this.receiveKeyframeGroup(message);
            return;
        }
        if (message.type === 'chooseKeyframe') {
            try { await this.openKeyframe(message.frame, message.program); }
            catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            return;
        }
        if (message.type === 'editStateRoute') {
            try { await this.promptStateRoute(message.frame, message.program); }
            catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            return;
        }
        if (message.type === 'clipPlacement') {
            try { await this.clipPlacement(message.frame, message.draft, true, AbortSignal.timeout(15_000)); }
            catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            this.post({ type: 'animationReply', request: message.request });
            return;
        }
        if (message.type === 'clipActions') {
            try {
                const { clipAuthoring, programs } = this.animations.state;
                if (!clipAuthoring) throw new Error('Use a current app with clip authoring support.');
                const command = await chooseHostedClipEdit(clipAuthoring, programs, message.program);
                if (command) await this.clipEdit(message.frame, command, true, AbortSignal.timeout(15_000));
            } catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            this.post({ type: 'animationReply', request: message.request });
            return;
        }
        if (message.type === 'stateActions') {
            try { await this.promptStateGroup(message.frame, message.program); }
            catch (error) { this.report(errorMessage(error), Boolean(this.currentFrame)); }
            this.post({ type: 'animationReply', request: message.request });
            return;
        }
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

    private async receiveKeyframeGroup(message: Extract<FromHosted, { type: 'keyframeGroup' }>): Promise<void> {
        const { action, frame, request } = message;
        const reply = { type: 'keyframeGroupReply' as const, frame, request };
        const ensure = () => {
            const current = this.currentFrame;
            if (!current || current.revision !== frame.revision || current.sequence !== frame.sequence || this.animations.driving) {
                throw new Error('The application preview changed while choosing a keyframe action.');
            }
        };
        try {
            ensure();
            const program = this.animations.program(frame, action.program);
            const command = await prepareHostedKeyframeGroup(action, program, this.animations.state.targets, ensure);
            if (!command) { this.post(reply); return; }
            const plan = await this.keyframeGroup(frame, { program: program.id, command }, true, AbortSignal.timeout(15_000));
            if (plan.clipboard) await writeTimelineClipboard(plan.clipboard, action.keys.length);
            this.post(plan.applied ? { ...reply, keys: plan.keys ?? [], version: plan.version } : reply);
        } catch (error) {
            this.post({ ...reply, error: errorMessage(error) });
        }
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

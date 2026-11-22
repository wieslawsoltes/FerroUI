import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';
import type * as vscode from 'vscode';
import type { BuiltProject } from './builtProjects';
import type { CanvasSnapAdjustment, CanvasSnapGesture, GuideCatalog, GuideEdit, GuideSettings, GuideUpdate } from './canvasGuidesProtocol';
import type { TokenPlan, TokenRequest } from './designTokenProtocol';
import type { LayerViewInput } from './layerViews';
import type { CurvePoint, TimelinePreviewPose, TimelineTargetOverrides } from './timelineProtocol';
import type { NativeProjectInventory } from './projectInventory';
import type { Asset, CanvasInteraction, CanvasPresentation, DesignDataPresentation, DropHint, EditResult, Inspection, InspectionTarget, Operation, Placement, PreviewSourceBuffer, ResourceRow, Snapshot, Source, Viewport } from './protocol';
import type { RsxLanguageDocument } from './rsxLanguageTypes';
import type { HostedAnimationFrame, HostedAnimationRequest, HostedClipDraft, HostedClipEdit, HostedClipPlan, HostedCurveQuery, HostedKeyframeDraft, HostedKeyframeGroupDraft, HostedKeyframeGroupPlan, HostedKeyframePlan, HostedSourcePatch, HostedSourcePlan, HostedStateDraft } from './hostedAnimationProtocol';
import type { HostedAnimationSample, HostedFrame, HostedInput, HostedSource, HostedWorkspaceFrame, HostedWorkspaceSources } from './hostedProtocol';
import type { NativePaletteRequest } from './hostedPaletteProtocol';
import type { NativeStyleRequest } from './hostedStylesProtocol';
import type { NativeTemplateRequest } from './hostedTemplates';
import type { StateGroupEdit } from './stateGroupProtocol';
import type { NativeStageChange, PrototypeCommand, PrototypeHotspot, PrototypeScene, StageChecks, StageDocument, StageFocus, StageImage, StageKeyboard, StageManifest, StageModel, StagePick, StagePointer, StageStateOptions, StageTarget } from './stagesProtocol';
import { commandFor } from './tools';
import type { RustModuleProject } from './cargoTarget';

const timelineActions = new Set(['scrub', 'play_program', 'play_range', 'advance_program', 'pause_program', 'stop_program']);

interface RenderContext {
    workspaceRoot?: string;
    localSources?: PreviewSourceBuffer[];
    rustProject?: RustModuleProject;
    designData?: DesignDataPresentation;
    layerView?: LayerViewInput;
    timelinePose?: TimelinePreviewPose;
    timelineTargets?: TimelineTargetOverrides;
}

export class NativePreview implements vscode.Disposable {
    private readonly child: ChildProcessWithoutNullStreams;
    private readonly connection;
    private stopped = false;
    private failure?: Error;
    private readonly projects = new WeakMap<BuiltProject, Promise<number>>();
    private nextProject = 0;
    assets: readonly Asset[] = [];
    canvasGuides?: GuideCatalog;
    capabilities: {
        isolatedDesign?: boolean; appHostedDesign?: boolean; hostedSourceSnapshots?: boolean; hostedPalette?: boolean; hostedStyles?: boolean;
        hostedTemplates?: boolean;
        hostedAnimationRestoration?: boolean; hostedPreviewRanges?: boolean; hostedClipAuthoring?: boolean; hostedStateAuthoring?: boolean; rsxLanguage?: boolean;
        stages?: boolean; stageKeyboard?: boolean; prototypeEditing?: boolean; stageIncludes?: boolean; stageSourceMoves?: boolean; projectCatalog?: boolean;
        canvasInteraction?: boolean; canvasFocusOrder?: boolean; canvasHighContrast?: boolean; spacingHelpers?: boolean; selectionWrapping?: boolean;
        columnAuthoring?: boolean; rsxColumnAuthoring?: boolean; itemTemplateAuthoring?: boolean; rsxItemTemplateAuthoring?: boolean;
        behaviorAuthoring?: boolean;
        libraryScopes?: boolean;
        controlTemplateAuthoring?: boolean; propertyTrace?: boolean; includedPropertyTrace?: boolean;
        styleComposition?: boolean; rustStyleComposition?: boolean;
        hostedRustStyles?: boolean; hostedRustModules?: boolean; rustModuleSources?: boolean;
        projectInventory?: boolean; tokenHandoff?: boolean;
        designFixtures?: boolean; timelineTransport?: boolean; timelineRecording?: boolean; timelineGeometryRecording?: boolean;
        timelineKeyframeDrafts?: boolean; timelineKeyframeScale?: boolean; timelineCurves?: boolean; timelineCurveVelocity?: boolean;
        timelineCurveChannels?: boolean; timelineCombinedPlayback?: boolean; timelineClipPlacement?: boolean;
        timelineKeyframeTransfer?: boolean;
        timelineProgramTargets?: boolean;
        stateTransitionPreview?: boolean;
        rsxStateAuthoring?: boolean;
        rsxStateStructure?: boolean;
        rsxKeyframeStructure?: boolean;
        rsxAnimationPrograms?: boolean;
    } = {};

    private constructor(command: string, channel: vscode.OutputChannel) {
        this.child = spawn(command, [], { windowsHide: true });
        this.connection = createMessageConnection(
            new StreamMessageReader(this.child.stdout), new StreamMessageWriter(this.child.stdin),
        );
        this.child.stderr.on('data', (data: Buffer) => channel.append(data.toString()));
        this.child.on('error', error => {
            this.failure = new Error(`${error.message}. Set ferroui.preview.path to the preview executable.`);
            this.dispose();
        });
        this.connection.onClose(() => this.dispose());
        this.connection.listen();
    }

    static async start(context: Pick<vscode.ExtensionContext, 'extensionPath'>, channel: vscode.OutputChannel): Promise<NativePreview> {
        const command = await commandFor('preview', context);
        const client = new NativePreview(command, channel);
        try {
            const hello = await client.send<{ version: number; assets: Asset[]; capabilities?: NativePreview['capabilities']; canvasGuides?: GuideCatalog }>('initialize');
            if (hello.version !== 4) throw new Error(`Preview protocol ${hello.version} is incompatible with this extension.`);
            client.assets = hello.assets;
            client.capabilities = hello.capabilities ?? {};
            client.canvasGuides = hello.canvasGuides;
            return client;
        } catch (error) {
            client.dispose();
            throw error;
        }
    }

    get closed(): boolean { return this.stopped; }

    editCanvasGuides(settings: GuideSettings, edit: GuideEdit): Promise<GuideUpdate> {
        if (!this.canvasGuides) throw new Error('Update the FerroUI preview helper to edit canvas guides.');
        return this.send('canvasGuideEdit', { settings, edit });
    }

    canvasSnap(frame: number, settings: GuideSettings, zoom: number, gesture: CanvasSnapGesture): Promise<CanvasSnapAdjustment> {
        if (!this.canvasGuides?.snapping) throw new Error('Update the FerroUI preview helper to use canvas snapping.');
        return this.send('canvasSnap', { frame, settings, zoom, gesture });
    }

    async inProject(source: Source, project?: BuiltProject): Promise<Source> {
        if (!project || !this.capabilities.projectCatalog) return source;
        let configured = this.projects.get(project);
        if (!configured) {
            const id = ++this.nextProject;
            configured = this.send('configureProject', {
                id, workspaceRoot: project.workspaceRoot, resourceMoves: project.resourceMoves, ...project.metadata,
            }).then(() => id);
            this.projects.set(project, configured);
            void configured.catch(() => this.projects.delete(project));
        }
        return { ...source, project: await configured };
    }

    rsxLanguage(source: Source): Promise<RsxLanguageDocument | undefined> {
        return this.capabilities.rsxLanguage ? this.send('rsxLanguage', source) : Promise.resolve(undefined);
    }

    projectInventory(workspaceRoot: string): Promise<NativeProjectInventory> {
        if (!this.capabilities.projectInventory) throw new Error('Update the FerroUI preview helper to browse project files.');
        return this.send('projectInventory', { workspaceRoot });
    }

    designTokens(request: TokenRequest): Promise<TokenPlan> {
        if (!this.capabilities.tokenHandoff) throw new Error('Update the FerroUI preview helper to convert design tokens.');
        return this.send('designTokens', request);
    }

    listenHosted(nonce: string): Promise<{ port: number; animationQuery?: string }> {
        if (!this.capabilities.appHostedDesign) throw new Error('Update the FerroUI preview helper to use app-hosted design.');
        return this.send('hostedListen', { nonce });
    }

    renderHosted(source: HostedSource): Promise<HostedFrame> {
        // The first request covers process startup, authentication and the first native frame.
        return this.send('hostedRender', source, 35_000);
    }
    async renderHostedWorkspace(source: HostedSource, workspace: HostedWorkspaceSources): Promise<HostedWorkspaceFrame> {
        if (!this.capabilities.hostedSourceSnapshots) {
            return { frame: await this.renderHosted(source), error: null, dependencies: null };
        }
        return this.send('hostedRenderWorkspace', { ...source, workspace }, 35_000);
    }
    hostedFrame(): Promise<HostedFrame | null> { return this.send('hostedFrame'); }
    hostedPalette<T>(frame: HostedAnimationFrame, request: NativePaletteRequest): Promise<T> {
        if (!this.capabilities.hostedPalette) throw new Error('Update the FerroUI helper to edit an App Preview palette.');
        return this.send('hostedPalette', { ...frame, ...request });
    }
    hostedStyles<T>(frame: HostedAnimationFrame, request: NativeStyleRequest): Promise<T> {
        if (!this.capabilities.hostedStyles) throw new Error('Update the FerroUI helper to edit styles in App Preview.');
        return this.send('hostedStyles', { ...frame, ...request });
    }
    hostedTemplates<T>(frame: HostedAnimationFrame, request: NativeTemplateRequest): Promise<T> {
        if (!this.capabilities.hostedTemplates) throw new Error('Update the FerroUI helper to use App Preview templates.');
        return this.send('hostedTemplates', { ...frame, ...request });
    }
    hostedInput(input: HostedInput): Promise<void> { return this.send('hostedInput', input); }
    sampleHostedAnimation(request: HostedAnimationRequest): Promise<HostedAnimationSample> {
        return this.send('hostedAnimate', request, 12_000);
    }

    planHostedKeyframe(frame: HostedAnimationFrame, draft: HostedKeyframeDraft): Promise<HostedKeyframePlan> {
        return this.send('hostedKeyframe', { ...frame, ...draft }, 12_000);
    }
    async planHostedKeyframeGroup(frame: HostedAnimationFrame, draft: HostedKeyframeGroupDraft): Promise<HostedKeyframeGroupPlan> {
        const { patch, ...plan } = await this.send<Omit<HostedKeyframeGroupPlan, 'patches'> & { patch: HostedSourcePatch | null }>(
            'hostedKeyframeSelection', { revision: frame.revision, sequence: frame.sequence, ...draft }, 12_000);
        return { ...plan, patches: patch ? [patch] : [] };
    }
    planHostedState(frame: HostedAnimationFrame, draft: HostedStateDraft): Promise<HostedSourcePlan> {
        return this.send('hostedState', { revision: frame.revision, sequence: frame.sequence, ...draft });
    }
    planHostedStateGroup(frame: HostedAnimationFrame, command: StateGroupEdit): Promise<HostedSourcePlan> {
        if (!this.capabilities.hostedStateAuthoring) throw new Error('Update the FerroUI preview helper to edit visual states.');
        return this.send('hostedStateGroup', { revision: frame.revision, sequence: frame.sequence, command });
    }
    planHostedClipPlacement(frame: HostedAnimationFrame, draft: HostedClipDraft): Promise<HostedSourcePlan> {
        return this.send('hostedClipPlacement', { ...frame, ...draft });
    }
    planHostedClipEdit(frame: HostedAnimationFrame, command: HostedClipEdit): Promise<HostedClipPlan> {
        if (!this.capabilities.hostedClipAuthoring) throw new Error('Update the FerroUI preview helper to edit native clips.');
        return this.send('hostedClipEdit', { revision: frame.revision, sequence: frame.sequence, command });
    }
    hostedCurve(frame: HostedAnimationFrame, query: HostedCurveQuery): Promise<CurvePoint[]> {
        const { metric, channel, ...track } = query;
        if (metric === 'velocity' && !this.capabilities.timelineCurveVelocity) {
            throw new Error('Update the FerroUI preview helper to sample velocity.');
        }
        if (channel && channel !== 'value' && !this.capabilities.timelineCurveChannels) {
            throw new Error('Update the FerroUI preview helper to sample a property component.');
        }
        return this.send('hostedCurve', { ...frame, ...track,
            ...(metric === 'velocity' && { metric }), ...(channel && channel !== 'value' && { channel }) }, 12_000);
    }

    syncStages(source: StageDocument, manifest?: StageManifest, additional: StageDocument[] = [], localSources?: PreviewSourceBuffer[],
        sourceMoves?: Record<string, string>): Promise<StageModel> {
        if (!this.capabilities.stages) throw new Error('Update the FerroUI preview helper to use Stages.');
        const params = { source, manifest, additional, sourceMoves };
        const drafts = localSources?.length ? fitStageDrafts(params, localSources) : undefined;
        return this.send('stagesSync', { ...params, localSources: drafts });
    }

    editStages(change: NativeStageChange): Promise<StageModel> {
        if (!this.capabilities.prototypeEditing && (change.action === 'setPrototype' || (change.action === 'add' && change.source))) {
            throw new Error('Update the FerroUI preview helper to edit prototypes and add other documents.');
        }
        return this.send('stagesEdit', change);
    }
    stageImage(target: StageTarget): Promise<StageImage> { return this.send('stagesImage', target); }
    stageChecks(target: StageTarget): Promise<StageChecks> { return this.send('stagesChecks', target); }
    stageStateOptions(target: StageTarget): Promise<StageStateOptions[]> { return this.send('stagesStateOptions', target); }
    stageFixtureOptions(target: StageTarget): Promise<string[]> {
        if (!this.capabilities.designFixtures) throw new Error('Update the FerroUI preview helper to select named fixtures.');
        return this.send('stagesFixtureOptions', target);
    }
    stagePointer(input: StagePointer): Promise<StagePick> { return this.send('stagesPointer', input); }
    stageKeyboard(input: StageKeyboard): Promise<StageFocus | null> {
        if (!this.capabilities.stageKeyboard) throw new Error('Update the FerroUI preview helper to use stage keyboard input.');
        return this.send('stagesKeyboard', input);
    }
    stagePrototypeElements(target: StageTarget): Promise<PrototypeHotspot[]> {
        if (!this.capabilities.prototypeEditing) throw new Error('Update the FerroUI preview helper to edit prototypes.');
        return this.send('stagesPrototypeElements', target);
    }
    stagePrototype(epoch: number, command: PrototypeCommand): Promise<PrototypeScene | null> {
        return this.send('stagesPrototype', { epoch, ...command });
    }

    render(source: Source, viewport: Viewport, frame = 0, context: RenderContext = {}): Promise<Snapshot> {
        const { timelinePose, timelineTargets, ...presentation } = context;
        return this.send('render', { ...source, ...viewport, frame, ...presentation,
            timelinePose: timelinePose?.playback && !this.capabilities.timelineCombinedPlayback ? undefined : timelinePose,
            timelineTargets: this.capabilities.timelineProgramTargets ? timelineTargets : undefined });
    }

    placement(frame: number, placement: Placement, recording = false): Promise<DropHint> {
        return this.send('placement', { frame, placement, ...(recording && { recording }) });
    }

    inspect(source: Source, frame: number, target: InspectionTarget): Promise<Inspection> {
        return this.send('inspect', { ...source, frame, target });
    }

    resources(source: Source, frame: number, target?: number): Promise<ResourceRow[]> {
        return this.send('resources', { ...source, frame, target });
    }

    studioTools(): Promise<{ tools: object[]; xamlEdits: object[]; rsxEdits?: object[]; presentations?: object[] }> { return this.send('studioTools'); }

    studioQuery(frame: number, name: string, args: object, selection?: number[]): Promise<unknown> {
        return this.send('studioQuery', { frame, name, arguments: args, selection });
    }

    studioEdit(source: Source, frame: number, name: string, args: object, selection: number[]): Promise<EditResult> {
        if (name === 'designer_record_property' && !this.capabilities.timelineRecording) {
            throw new Error('Update the FerroUI preview helper to record property edits.');
        }
        if (name === 'designer_program' && (args as { action?: string }).action === 'place_clips' && !this.capabilities.timelineClipPlacement) {
            throw new Error('Update the FerroUI preview helper to place animation clips.');
        }
        return this.send('studioEdit', { ...source, frame, name, arguments: args, selection });
    }

    studioPresent(frame: number, nextFrame: number, name: string, args: object, selection: number[]): Promise<CanvasPresentation> {
        const { action } = args as { action?: string };
        if (name === 'designer_program' && action === 'play_all' && !this.capabilities.timelineCombinedPlayback) {
            throw new Error('Update the FerroUI preview helper to play all animations.');
        }
        if (name === 'designer_program' && action === 'scrub_all' && !this.capabilities.timelineClipPlacement) {
            throw new Error('Update the FerroUI preview helper to scrub animation clips together.');
        }
        if (name === 'designer_state' && action === 'preview_transition' && !this.capabilities.stateTransitionPreview) {
            throw new Error('Update the FerroUI preview helper to preview explicit state transitions.');
        }
        if (name === 'designer_program' && action && timelineActions.has(action) && !this.capabilities.timelineTransport) {
            throw new Error('Update the FerroUI preview helper to use the Timeline playback controls.');
        }
        return this.send('studioPresent', { frame, nextFrame, name, arguments: args, selection });
    }

    canvasInput(frame: number, nextFrame: number, action: CanvasInteraction): Promise<CanvasPresentation> {
        if (!this.capabilities.canvasInteraction) throw new Error('The native preview helper does not support canvas input.');
        return this.send('canvasInput', { frame, nextFrame, action });
    }

    edit(source: Source, operation: Operation, frame?: number): Promise<EditResult> {
        return this.send('edit', { ...source, operation, frame });
    }

    verifyRustModule(frame: number, uri: string, source: string): Promise<void> {
        if (!this.capabilities.rustModuleSources) throw new Error('Update the FerroUI preview helper to use Rust module sources.');
        return this.send('rustModuleSource', { frame, uri, source });
    }

    private async send<T>(method: string, params?: object, timeoutMs = 20_000): Promise<T> {
        if (this.stopped) throw new Error('The FerroUI preview process exited.');
        let timer: ReturnType<typeof setTimeout> | undefined;
        try {
            return await Promise.race([
                this.connection.sendRequest<T>(method, params),
                new Promise<never>((_, reject) => {
                    timer = setTimeout(() => {
                        this.failure = new Error('The FerroUI preview process timed out.');
                        this.dispose();
                        reject(this.failure);
                    }, timeoutMs);
                }),
            ]);
        } catch (error) {
            throw this.failure ?? (this.stopped ? new Error('The FerroUI preview process exited. See the FerroUI Designer output.') : error);
        } finally {
            clearTimeout(timer);
        }
    }

    dispose(): void {
        if (this.stopped) return;
        this.stopped = true;
        this.connection.dispose();
        this.child.kill();
    }
}

function fitStageDrafts(params: object, drafts: PreviewSourceBuffer[]): PreviewSourceBuffer[] {
    // The native JSON-RPC reader limits the entire message to 16 MiB. Reserve
    // every draft identity so an omitted draft cannot fall back to its saved file.
    const packed: PreviewSourceBuffer[] = drafts.map(({ uri }) => ({ uri, source: null }));
    let available = 16 * 1024 * 1024 - Buffer.byteLength(JSON.stringify({
        jsonrpc: '2.0', id: Number.MAX_SAFE_INTEGER, method: 'stagesSync', params: { ...params, localSources: packed },
    }));
    if (available < 0) throw new Error('The stage sources and include paths exceed the native request size.');
    for (const [index, { source }] of drafts.entries()) {
        if (source === null) continue;
        const extra = Buffer.byteLength(JSON.stringify(source)) - 4;
        if (extra > available) continue;
        packed[index].source = source;
        available -= extra;
    }
    return packed;
}

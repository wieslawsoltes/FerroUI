import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';
import type * as vscode from 'vscode';
import type { BuiltProject } from './builtProjects';
import type { TokenPlan, TokenRequest } from './designTokenProtocol';
import type { LayerViewInput } from './layerViews';
import type { NativeProjectInventory } from './projectInventory';
import type { Asset, CanvasInteraction, CanvasPresentation, DesignDataPresentation, DropHint, EditResult, Inspection, InspectionTarget, Operation, Placement, PreviewSourceBuffer, ResourceRow, Snapshot, Source, Viewport } from './protocol';
import type { RsxLanguageDocument } from './rsxLanguageTypes';
import type { HostedFrame, HostedInput, HostedSource } from './hostedProtocol';
import type { NativeStageChange, PrototypeCommand, PrototypeHotspot, PrototypeScene, StageChecks, StageDocument, StageImage, StageManifest, StageModel, StagePick, StagePointer, StageStateOptions, StageTarget } from './stagesProtocol';
import { commandFor } from './tools';

interface RenderContext {
    workspaceRoot?: string;
    localSources?: PreviewSourceBuffer[];
    designData?: DesignDataPresentation;
    layerView?: LayerViewInput;
}

export class NativePreview implements vscode.Disposable {
    private readonly child: ChildProcessWithoutNullStreams;
    private readonly connection;
    private stopped = false;
    private failure?: Error;
    private readonly projects = new WeakMap<BuiltProject, Promise<number>>();
    private nextProject = 0;
    assets: readonly Asset[] = [];
    capabilities: {
        isolatedDesign?: boolean; appHostedDesign?: boolean; rsxLanguage?: boolean;
        stages?: boolean; prototypeEditing?: boolean; stageIncludes?: boolean; projectCatalog?: boolean;
        canvasInteraction?: boolean; canvasFocusOrder?: boolean; projectInventory?: boolean; tokenHandoff?: boolean;
        designFixtures?: boolean;
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
            const hello = await client.send<{ version: number; assets: Asset[]; capabilities?: NativePreview['capabilities'] }>('initialize');
            if (hello.version !== 4) throw new Error(`Preview protocol ${hello.version} is incompatible with this extension.`);
            client.assets = hello.assets;
            client.capabilities = hello.capabilities ?? {};
            return client;
        } catch (error) {
            client.dispose();
            throw error;
        }
    }

    get closed(): boolean { return this.stopped; }

    async inProject(source: Source, project?: BuiltProject): Promise<Source> {
        if (!project || !this.capabilities.projectCatalog) return source;
        let configured = this.projects.get(project);
        if (!configured) {
            const id = ++this.nextProject;
            configured = this.send('configureProject', { id, workspaceRoot: project.workspaceRoot, ...project.metadata }).then(() => id);
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

    listenHosted(nonce: string): Promise<{ port: number }> {
        if (!this.capabilities.appHostedDesign) throw new Error('Update the FerroUI preview helper to use app-hosted design.');
        return this.send('hostedListen', { nonce });
    }

    renderHosted(source: HostedSource): Promise<HostedFrame> {
        // The first request covers process startup, authentication and the first native frame.
        return this.send('hostedRender', source, 35_000);
    }
    hostedFrame(): Promise<HostedFrame | null> { return this.send('hostedFrame'); }
    hostedInput(input: HostedInput): Promise<void> { return this.send('hostedInput', input); }

    syncStages(source: StageDocument, manifest?: StageManifest, additional: StageDocument[] = [], localSources?: PreviewSourceBuffer[]): Promise<StageModel> {
        if (!this.capabilities.stages) throw new Error('Update the FerroUI preview helper to use Stages.');
        const params = { source, manifest, additional };
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
    stagePrototypeElements(target: StageTarget): Promise<PrototypeHotspot[]> {
        if (!this.capabilities.prototypeEditing) throw new Error('Update the FerroUI preview helper to edit prototypes.');
        return this.send('stagesPrototypeElements', target);
    }
    stagePrototype(epoch: number, command: PrototypeCommand): Promise<PrototypeScene | null> {
        return this.send('stagesPrototype', { epoch, ...command });
    }

    render(source: Source, viewport: Viewport, frame = 0, context: RenderContext = {}): Promise<Snapshot> {
        return this.send('render', { ...source, ...viewport, frame, ...context });
    }

    placement(frame: number, placement: Placement): Promise<DropHint> {
        return this.send('placement', { frame, placement });
    }

    inspect(source: Source, frame: number, target: InspectionTarget): Promise<Inspection> {
        return this.send('inspect', { ...source, frame, target });
    }

    resources(source: Source, frame: number, target?: number): Promise<ResourceRow[]> {
        return this.send('resources', { ...source, frame, target });
    }

    studioTools(): Promise<{ tools: object[]; xamlEdits: object[]; presentations?: object[] }> { return this.send('studioTools'); }

    studioQuery(frame: number, name: string, args: object, selection?: number[]): Promise<unknown> {
        return this.send('studioQuery', { frame, name, arguments: args, selection });
    }

    studioEdit(source: Source, frame: number, name: string, args: object, selection: number[]): Promise<EditResult> {
        return this.send('studioEdit', { ...source, frame, name, arguments: args, selection });
    }

    studioPresent(frame: number, nextFrame: number, name: string, args: object, selection: number[]): Promise<CanvasPresentation> {
        return this.send('studioPresent', { frame, nextFrame, name, arguments: args, selection });
    }

    canvasInput(frame: number, nextFrame: number, action: CanvasInteraction): Promise<CanvasPresentation> {
        if (!this.capabilities.canvasInteraction) throw new Error('The native preview helper does not support canvas input.');
        return this.send('canvasInput', { frame, nextFrame, action });
    }

    edit(source: Source, operation: Operation): Promise<EditResult> {
        return this.send('edit', { ...source, operation });
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

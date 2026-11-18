import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';
import type * as vscode from 'vscode';
import type { Asset, DropHint, EditResult, Inspection, InspectionTarget, Operation, Placement, ResourceRow, Snapshot, Source, StudioPresentation, Viewport } from './protocol';
import type { RsxLanguageDocument } from './rsxLanguageTypes';
import type { HostedFrame, HostedInput, HostedSource } from './hostedProtocol';
import type { PrototypeCommand, PrototypeScene, StageChange, StageImage, StageManifest, StageModel, StagePick, StagePointer, StageTarget } from './stagesProtocol';
import { commandFor } from './tools';

export class NativePreview implements vscode.Disposable {
    private readonly child: ChildProcessWithoutNullStreams;
    private readonly connection;
    private stopped = false;
    private failure?: Error;
    assets: readonly Asset[] = [];
    capabilities: { isolatedDesign?: boolean; appHostedDesign?: boolean; rsxLanguage?: boolean; stages?: boolean } = {};

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

    rsxLanguage(source: string): Promise<RsxLanguageDocument | undefined> {
        return this.capabilities.rsxLanguage ? this.send('rsxLanguage', { source }) : Promise.resolve(undefined);
    }

    listenHosted(nonce: string): Promise<{ port: number }> {
        if (!this.capabilities.appHostedDesign) throw new Error('Update the FerroUI preview helper to use app-hosted design.');
        return this.send('hostedListen', { nonce });
    }

    renderHosted(source: HostedSource): Promise<HostedFrame> { return this.send('hostedRender', source); }
    hostedFrame(): Promise<HostedFrame | null> { return this.send('hostedFrame'); }
    hostedInput(input: HostedInput): Promise<void> { return this.send('hostedInput', input); }

    syncStages(source: Source, manifest?: StageManifest, additional: Source[] = []): Promise<StageModel> {
        if (!this.capabilities.stages) throw new Error('Update the FerroUI preview helper to use Stages.');
        return this.send('stagesSync', { source, manifest, additional });
    }

    editStages(change: StageChange): Promise<StageModel> { return this.send('stagesEdit', change); }
    stageImage(target: StageTarget): Promise<StageImage> { return this.send('stagesImage', target); }
    stagePointer(input: StagePointer): Promise<StagePick> { return this.send('stagesPointer', input); }
    stagePrototype(epoch: number, command: PrototypeCommand): Promise<PrototypeScene | null> {
        return this.send('stagesPrototype', { epoch, ...command });
    }

    render(source: Source, viewport: Viewport, frame = 0, workspaceRoot?: string): Promise<Snapshot> {
        return this.send('render', { ...source, ...viewport, frame, workspaceRoot });
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

    studioPresent(frame: number, nextFrame: number, name: string, args: object, selection: number[]): Promise<StudioPresentation> {
        return this.send('studioPresent', { frame, nextFrame, name, arguments: args, selection });
    }

    edit(source: Source, operation: Operation): Promise<EditResult> {
        return this.send('edit', { ...source, operation });
    }

    private async send<T>(method: string, params?: object): Promise<T> {
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
                    }, 20_000);
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

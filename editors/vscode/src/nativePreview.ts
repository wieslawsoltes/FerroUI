import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';
import type * as vscode from 'vscode';
import type { Asset, DropHint, Operation, Placement, Snapshot, Source, TextEdit, Viewport } from './protocol';
import { commandFor } from './tools';

export class NativePreview implements vscode.Disposable {
    private readonly child: ChildProcessWithoutNullStreams;
    private readonly connection;
    private stopped = false;
    private failure?: Error;
    assets: readonly Asset[] = [];

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
            const hello = await client.send<{ version: number; assets: Asset[] }>('initialize');
            if (hello.version !== 2) throw new Error(`Preview protocol ${hello.version} is incompatible with this extension.`);
            client.assets = hello.assets;
            return client;
        } catch (error) {
            client.dispose();
            throw error;
        }
    }

    get closed(): boolean { return this.stopped; }

    render(source: Source, viewport: Viewport, frame = 0): Promise<Snapshot> {
        return this.send('render', { ...source, ...viewport, frame });
    }

    placement(frame: number, placement: Placement): Promise<DropHint> {
        return this.send('placement', { frame, placement });
    }

    edit(source: Source, operation: Operation): Promise<TextEdit[]> {
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

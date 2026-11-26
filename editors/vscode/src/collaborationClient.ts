import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node';
import type * as vscode from 'vscode';
import type { RoomCommand } from './collaborationProtocol';
import { commandFor } from './tools';

export class CollaborationClient {
    private readonly connection;
    private readonly exited: Promise<void>;
    private stopped = false;
    private failure?: Error;

    private constructor(private readonly child: ChildProcessWithoutNullStreams, output: vscode.OutputChannel) {
        this.connection = createMessageConnection(new StreamMessageReader(child.stdout), new StreamMessageWriter(child.stdin));
        this.exited = new Promise(resolve => child.once('close', () => resolve()));
        child.stderr.on('data', (chunk: Buffer) => output.append(chunk.toString()));
        child.on('error', error => {
            this.failure = error;
            this.terminate();
        });
        this.connection.onClose(() => this.terminate());
        this.connection.listen();
    }

    static async start(context: vscode.ExtensionContext, output: vscode.OutputChannel): Promise<CollaborationClient> {
        const command = await commandFor('preview', context);
        const client = new CollaborationClient(spawn(command, ['--collaboration'], { windowsHide: true }), output);
        try {
            const hello = await client.request<{ version: number }>('initialize', undefined, 5_000);
            if (hello.version !== 1) throw new Error('Update the FerroUI native helper to use collaboration.');
            return client;
        } catch (error) {
            client.terminate();
            throw error;
        }
    }

    get closed(): boolean { return this.stopped; }

    call<T>(command: RoomCommand): Promise<T> { return this.request('room', command, 55_000); }

    private async request<T>(method: string, params: object | undefined, milliseconds: number): Promise<T> {
        if (this.stopped) throw this.failure ?? new Error('The collaboration worker disconnected. Resume the saved room.');
        let timer: NodeJS.Timeout | undefined;
        try {
            return await Promise.race([
                this.connection.sendRequest<T>(method, params),
                new Promise<never>((_, reject) => {
                    timer = setTimeout(() => {
                        this.failure = new Error('The collaboration worker timed out. Resume the saved room to recover any queued changes.');
                        reject(this.failure);
                        this.terminate();
                    }, milliseconds);
                }),
            ]);
        } catch (error) {
            throw this.failure ?? error;
        } finally { clearTimeout(timer); }
    }

    async stop(): Promise<void> {
        // EOF lets the worker release its private session and server locks. Submissions are
        // persisted before acknowledgement, so a stuck network call can be interrupted.
        if (!this.stopped) this.child.stdin.end();
        const timer = setTimeout(() => this.terminate(), 1500);
        try { await this.exited; }
        finally { clearTimeout(timer); }
    }

    private terminate(): void {
        if (this.stopped) return;
        this.stopped = true;
        this.connection.dispose();
        this.child.kill();
    }
}

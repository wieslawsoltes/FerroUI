import * as vscode from 'vscode';
import { trackBrowserPause } from './browserDebugTracker';
import type { BrowserPause } from './browserPause';
import type { BrowserApp } from './browserTargets';
import { errorMessage } from './protocol';
import { ReloadDebugger } from './reloadDebugger';
import type { ProcessOutput } from './reloadLog';
import { workspaceLocation } from './workspacePaths';

interface BrowserDebugFrame {
    generation: number;
    port: number;
    url: string;
    hostPage?: string;
    pause: BrowserPause;
    onPause(paused: boolean): void;
}

/** Attach JS Debug to accepted browser documents and follow their generation on reload. */
export class BrowserDebugger {
    private readonly debug: ReloadDebugger;
    private readonly tracker: vscode.Disposable;
    private frame?: BrowserDebugFrame;
    private stopWatching?: () => void;
    private startOnFirstFrame = true;

    constructor(private readonly app: BrowserApp, private readonly output: ProcessOutput) {
        this.debug = new ReloadDebugger(output, 'VS Code JavaScript Debug could not attach to the browser app.',
            () => this.frame?.pause.clear());
        this.tracker = vscode.debug.registerDebugAdapterTrackerFactory('*', { createDebugAdapterTracker: session => {
            const frame = this.frame;
            const current = () => this.frame === frame && frame !== undefined && this.debug.owns(session, frame.generation);
            return frame && current() ? trackBrowserPause(frame.pause, current) : undefined;
        } });
    }

    get attached(): boolean { return this.debug.attached; }

    accepted(frame: BrowserDebugFrame): void {
        this.stopWatching?.();
        this.frame = frame;
        this.stopWatching = frame.pause.onDidChange(frame.onPause);
        if (this.startOnFirstFrame) {
            this.startOnFirstFrame = false;
            void this.attach(frame.generation).catch(error => this.output.appendLine(errorMessage(error)));
        } else {
            this.debug.activated(frame.generation, this.configuration(frame));
        }
    }

    async attach(generation: number): Promise<void> {
        const frame = this.acceptedFrame(generation);
        if (this.debug.follows(generation)) return;
        const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.parse(this.app.folder, true));
        if (!folder) throw new Error('Open the workspace that owns this browser app.');
        await this.debug.attach(folder, this.configuration(frame), generation);
    }

    async detach(generation: number): Promise<void> {
        const frame = this.acceptedFrame(generation);
        await this.debug.detach();
        frame.pause.clear();
    }

    private acceptedFrame(generation: number): BrowserDebugFrame {
        const frame = this.frame;
        if (frame?.generation !== generation) throw new Error('Use the current accepted browser generation.');
        return frame;
    }

    private configuration(frame: BrowserDebugFrame): vscode.DebugConfiguration {
        const host = frame.hostPage && (workspaceLocation(frame.hostPage)?.uri.fsPath ?? frame.hostPage);
        const prefix = new URL(frame.url).pathname.replace(/\/$/, '') || '/';
        return {
            type: 'chrome', request: 'attach', name: `Debug ${this.app.label}`,
            address: '127.0.0.1', port: frame.port, url: frame.url, restart: false,
            browserAttachLocation: 'workspace', timeout: 15_000,
            ...(host ? { webRoot: host, pathMapping: { [prefix]: host } } : {}),
        };
    }

    retire(): void {
        this.stopWatching?.();
        this.stopWatching = undefined;
        this.frame = undefined;
        this.debug.detach();
    }

    close(): Promise<void> {
        this.retire();
        this.tracker.dispose();
        return this.debug.close();
    }
}

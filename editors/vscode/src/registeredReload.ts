import { spawn } from 'node:child_process';
import * as path from 'node:path';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';
import type { ProcessOutput } from './reloadLog';
import { ReloadProcess } from './reloadProcess';
import { isWorkspaceFile } from './workspacePaths';

const prefix = 'FERROUI_SUPERVISOR_V1=';
const states = {
    building: 'Building registered target', ready: 'Watching registered edits', restored: 'Rebuilt and restored state',
    xaml_applied: 'XAML reloaded', patch_applied: 'Registered Rust patch applied',
    rejected: 'Update rejected · app still running', superseded: 'A newer save superseded the build', stopped: 'Application stopped',
} as const;

export class RegisteredReload implements vscode.Disposable {
    pid?: number;
    private disposed = false;
    private constructor(readonly folder: vscode.WorkspaceFolder, private readonly process: ReloadProcess) {}

    static async start(folder: vscode.WorkspaceFolder, output: ProcessOutput,
        changed: (status: string, pid?: number, error?: boolean) => void): Promise<RegisteredReload> {
        if (!vscode.workspace.isTrusted || folder.uri.scheme !== 'file') throw new Error('Open a trusted FerroUI workspace.');
        const settings = vscode.workspace.getConfiguration('ferroui.hotReload', folder.uri);
        const script = vscode.Uri.file(path.resolve(folder.uri.fsPath, settings.get('registeredSupervisor', 'scripts/hot-reload/supervisor.py')));
        if (!await isWorkspaceFile(script) || vscode.workspace.getWorkspaceFolder(script)?.uri.toString() !== folder.uri.toString()) {
            throw new Error('Choose a registered patch supervisor script in this workspace.');
        }
        const python = settings.get<string>('pythonPath') || (process.platform === 'win32' ? 'python' : 'python3');
        const dx = settings.get<string>('patchCompilerPath');
        const child = spawn(python, [script.fsPath, '--source', folder.uri.fsPath, '--editor-events'], {
            cwd: folder.uri.fsPath, detached: process.platform !== 'win32', windowsHide: true,
            env: { ...process.env, ...(dx ? { FERROUI_DX: dx } : {}) }, stdio: ['ignore', 'pipe', 'pipe'],
        });
        const processHandle = new ReloadProcess(child);
        const session = new RegisteredReload(folder, processHandle);
        const lines = createInterface({ input: child.stdout });
        lines.on('line', line => {
            if (session.disposed) return;
            if (!line.startsWith(prefix)) { output.appendLine(line); return; }
            try {
                const event = JSON.parse(line.slice(prefix.length)) as { status: keyof typeof states; pid?: number | null };
                if (!Object.hasOwn(states, event.status)) return;
                session.pid = Number.isSafeInteger(event.pid) && event.pid! > 0 ? event.pid! : undefined;
                changed(states[event.status], session.pid, event.status === 'rejected');
            } catch { output.appendLine('The registered supervisor returned an invalid lifecycle event.'); }
        });
        child.stderr.on('data', data => output.append(String(data)));
        void processHandle.closed.then(code => {
            session.pid = undefined;
            if (!session.disposed) changed(code ? `Registered supervisor exited (${code})` : 'Application stopped', undefined, Boolean(code));
        }, error => { if (!session.disposed) changed(String(error), undefined, true); });
        return session;
    }

    dispose(): Promise<void> {
        this.disposed = true;
        return this.process.stop();
    }
}

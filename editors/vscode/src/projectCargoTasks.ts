import * as vscode from 'vscode';
import { describeCargoTarget, type CargoTarget } from './cargoTarget';

interface Launch {
    target: CargoTarget;
    execution: vscode.TaskExecution;
    stopRequested: boolean;
}

/** Track the FerroUI runs launched through VS Code's task service. */
export class ProjectCargoTasks implements vscode.Disposable {
    private next = 0;
    private readonly running = new Map<number, Launch>();
    private readonly started = new WeakSet<vscode.TaskExecution>();
    private readonly finished = new WeakSet<vscode.TaskExecution>();
    private readonly start = vscode.tasks.onDidStartTaskProcess(({ execution }) => {
        this.started.add(execution);
        for (const entry of this.running.values()) {
            if (entry.execution === execution && entry.stopRequested) execution.terminate();
        }
    });
    private readonly end = vscode.tasks.onDidEndTask(({ execution }) => {
        this.finished.add(execution);
        for (const [id, entry] of this.running) {
            if (entry.execution === execution) this.running.delete(id);
        }
    });

    async run(target: CargoTarget, signal?: AbortSignal) {
        if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before running a Cargo binary.');
        signal?.throwIfAborted();
        const { packageName, manifest, binary, features, cargo, cargoDirectory } = target;
        const args = ['run', '--manifest-path', manifest, '--package', packageName, '--bin', binary];
        if (features.length) args.push('--features', features.join(','));
        if (target.args.length) args.push('--', ...target.args);
        const id = ++this.next;
        const task = new vscode.Task({ type: 'ferroui', manifest, binary }, target.folder, `Run ${target.label}`, 'FerroUI',
            new vscode.ProcessExecution(cargo, args, { cwd: cargoDirectory }));
        const execution = await vscode.tasks.executeTask(task);
        if (!this.finished.has(execution)) this.running.set(id, { target, execution, stopRequested: false });
        return { id, ...describeCargoTarget(target) };
    }

    list() {
        return Array.from(this.running, ([id, { target, execution, stopRequested }]) => ({ id, ...describeCargoTarget(target),
            state: stopRequested ? 'stopping' : this.started.has(execution) ? 'running' : 'starting' }));
    }

    stop(id: number) {
        const entry = this.running.get(id);
        if (!entry) throw new Error('Choose a running FerroUI Cargo task.');
        entry.stopRequested = true;
        if (this.started.has(entry.execution)) entry.execution.terminate();
        return { id };
    }

    dispose(): void {
        this.start.dispose();
        this.end.dispose();
        this.running.clear();
    }
}

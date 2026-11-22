import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { commandFor } from './tools';
import { isWorkspaceFile } from './workspacePaths';

const exec = promisify(execFile);
export const scenarioPattern = '**/*.ferroui-scenario.json';

export interface ScenarioRunOptions {
    accept?: boolean;
    review?: boolean;
    reference?: string;
    approval?: string;
}

interface StepReport {
    step: number;
    passed: boolean;
    message: string;
    artifacts: string[];
}

export interface ImageSize {
    width: number;
    height: number;
}

export interface BaselineComparison {
    status: 'match' | 'changed' | 'missing' | 'resized';
    actual: ImageSize;
    expected: ImageSize | null;
    changedPixels: number | null;
    maxChangedPixels: number;
}

export interface VisualReview {
    step: number;
    baseline: string;
    workspace: BaselineComparison;
    comparison: BaselineComparison;
    actual: string;
    expected: string | null;
    difference: string | null;
    approval: string | null;
}

export interface ScenarioReport {
    name: string;
    document: string;
    passed: boolean;
    notices: string[];
    steps: StepReport[];
    reference?: { name: string; commit: string };
    visuals?: VisualReview[];
    approved?: string;
    artifactDirectory: string;
}

export function scenarioBaselineRef(uri: vscode.Uri): string | undefined {
    return vscode.workspace.getConfiguration('ferroui.scenarios', uri).get<string>('baselineRef')?.trim() || undefined;
}

export class ScenarioRunner implements vscode.Disposable {
    private root?: Promise<string>;
    private cleanup?: Promise<void>;
    private sequence = 0;
    private readonly stop = new AbortController();
    private readonly pending = new Set<Promise<ScenarioReport>>();

    constructor(private readonly context: vscode.ExtensionContext) {}

    run(uri: vscode.Uri, signal: AbortSignal, options: ScenarioRunOptions = {}): Promise<ScenarioReport> {
        const pending = this.start(uri, AbortSignal.any([signal, this.stop.signal]), options);
        this.pending.add(pending);
        void pending.then(() => this.pending.delete(pending), () => this.pending.delete(pending));
        return pending;
    }

    private async start(uri: vscode.Uri, signal: AbortSignal, options: ScenarioRunOptions): Promise<ScenarioReport> {
        if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before running native scenarios.');
        signal.throwIfAborted();
        const workspace = vscode.workspace.getWorkspaceFolder(uri);
        if (!workspace || !await isWorkspaceFile(uri) || !uri.fsPath.endsWith('.ferroui-scenario.json')) {
            throw new Error('Choose a .ferroui-scenario.json file in the current workspace.');
        }
        const binary = await commandFor('preview', this.context);
        signal.throwIfAborted();
        this.root ??= mkdtemp(path.join(tmpdir(), 'ferroui-scenarios-'));
        const artifactDirectory = path.join(await this.root, String(++this.sequence));
        const args = ['--scenario', uri.fsPath, '--workspace', workspace.uri.fsPath, '--artifacts', artifactDirectory];
        if (options.approval) args.push('--approve-baseline', options.approval);
        else if (options.review) args.push('--review-baselines');
        else if (options.accept) args.push('--accept-baselines');
        if (options.reference) args.push('--baseline-ref', options.reference);
        let stdout: string;
        try {
            ({ stdout } = await exec(binary, args, {
                cwd: path.dirname(uri.fsPath), encoding: 'utf8', signal, windowsHide: true, maxBuffer: 4 * 1024 * 1024,
            }));
        } catch (error) {
            const failure = error as { stdout?: string; stderr?: string };
            if (signal.aborted || !failure.stdout?.trim()) {
                await rm(artifactDirectory, { recursive: true, force: true }).catch(() => {});
                signal.throwIfAborted();
                throw new Error(failure.stderr?.trim() || String(error));
            }
            stdout = failure.stdout;
        }
        const report = JSON.parse(stdout) as ScenarioReport;
        if (!Array.isArray(report.steps) || !Array.isArray(report.notices) || typeof report.passed !== 'boolean') {
            throw new Error('The native runner returned an invalid scenario report.');
        }
        // Empty review results still need a resource root for their error view.
        if (options.review || options.approval) await mkdir(artifactDirectory, { recursive: true });
        return { ...report, artifactDirectory };
    }

    dispose(): void {
        void this.shutdown();
    }

    shutdown(): Promise<void> {
        this.stop.abort();
        this.cleanup ??= this.cleanArtifacts();
        return this.cleanup;
    }

    private async cleanArtifacts(): Promise<void> {
        await Promise.allSettled(this.pending);
        if (!this.root) return;
        const root = await this.root.catch(() => undefined);
        if (root) await rm(root, { recursive: true, force: true, maxRetries: 3 }).catch(() => {});
    }
}

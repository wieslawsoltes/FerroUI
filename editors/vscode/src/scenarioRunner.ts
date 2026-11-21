import { execFile } from 'node:child_process';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { commandFor } from './tools';
import { isWorkspaceFile } from './workspacePaths';

const exec = promisify(execFile);
export const scenarioPattern = '**/*.ferroui-scenario.json';

interface StepReport {
    step: number;
    passed: boolean;
    message: string;
    artifacts: string[];
}

export interface ScenarioReport {
    name: string;
    document: string;
    passed: boolean;
    notices: string[];
    steps: StepReport[];
}

export class ScenarioRunner {
    constructor(private readonly context: vscode.ExtensionContext) {}

    async run(uri: vscode.Uri, signal: AbortSignal, accept = false): Promise<ScenarioReport> {
        signal.throwIfAborted();
        const workspace = vscode.workspace.getWorkspaceFolder(uri);
        if (!workspace || !await isWorkspaceFile(uri) || !uri.fsPath.endsWith('.ferroui-scenario.json')) {
            throw new Error('Choose a .ferroui-scenario.json file in the current workspace.');
        }
        const binary = await commandFor('preview', this.context);
        const args = ['--scenario', uri.fsPath, '--workspace', workspace.uri.fsPath];
        if (accept) args.push('--accept-baselines');
        let stdout: string;
        try {
            ({ stdout } = await exec(binary, args, {
                cwd: path.dirname(uri.fsPath), encoding: 'utf8', signal, windowsHide: true, maxBuffer: 4 * 1024 * 1024,
            }));
        } catch (error) {
            signal.throwIfAborted();
            const failure = error as { stdout?: string; stderr?: string };
            if (!failure.stdout?.trim()) throw new Error(failure.stderr?.trim() || String(error));
            stdout = failure.stdout;
        }
        const report = JSON.parse(stdout) as ScenarioReport;
        if (!Array.isArray(report.steps) || !Array.isArray(report.notices) || typeof report.passed !== 'boolean') {
            throw new Error('The native runner returned an invalid scenario report.');
        }
        return report;
    }
}

import * as path from 'node:path';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import { ScenarioRunner, scenarioBaselineRef, scenarioPattern, type ScenarioReport } from './scenarioRunner';
import type { ScenarioReview } from './scenarioReview';

export class ScenarioTesting implements vscode.Disposable {
    private readonly controller = vscode.tests.createTestController('ferroui.scenarios', 'FerroUI Scenarios');
    private readonly subscriptions: vscode.Disposable[];
    private readonly revisions = new Map<string, number>();
    private revision = 0;
    private disposed = false;

    constructor(private readonly runner: ScenarioRunner, private readonly reviews: ScenarioReview) {
        const watcher = vscode.workspace.createFileSystemWatcher(scenarioPattern);
        this.subscriptions = [
            this.controller, watcher,
            watcher.onDidCreate(uri => { void this.load(uri); }),
            watcher.onDidChange(uri => { void this.load(uri); }),
            watcher.onDidDelete(uri => this.remove(uri.toString())),
            vscode.workspace.onDidChangeWorkspaceFolders(() => { void this.discover(); }),
            this.controller.createRunProfile('Run FerroUI Scenario', vscode.TestRunProfileKind.Run,
                (request, token) => this.run(request, token, false), true),
            this.controller.createRunProfile('Review FerroUI Screenshot Baselines', vscode.TestRunProfileKind.Run,
                (request, token) => this.run(request, token, true), false),
        ];
        this.controller.resolveHandler = () => this.discover();
        this.controller.refreshHandler = () => this.discover();
    }

    private async discover(): Promise<void> {
        const previous = this.revision;
        const files = await vscode.workspace.findFiles(scenarioPattern, '**/{.git,node_modules,target}/**');
        if (this.disposed) return;
        const found = new Set(files.map(uri => uri.toString()));
        this.controller.items.forEach(item => {
            if (!found.has(item.id) && (this.revisions.get(item.id) ?? 0) <= previous) this.remove(item.id);
        });
        await Promise.all(files.map(uri => this.load(uri)));
    }

    private remove(id: string): void {
        this.revisions.delete(id);
        this.controller.items.delete(id);
    }

    private async load(uri: vscode.Uri): Promise<void> {
        const id = uri.toString();
        const revision = ++this.revision;
        this.revisions.set(id, revision);
        let label = path.basename(uri.fsPath, '.ferroui-scenario.json');
        let problem: string | undefined;
        try {
            const text = new TextDecoder().decode(await vscode.workspace.fs.readFile(uri));
            const definition = JSON.parse(text) as { name?: unknown };
            if (typeof definition.name === 'string' && definition.name.trim()) label = definition.name;
        } catch (error) {
            problem = errorMessage(error);
        }
        if (this.disposed || revision !== this.revisions.get(id)) return;
        const item = this.controller.items.get(id) ?? this.controller.createTestItem(id, label, uri);
        item.label = label;
        item.error = problem;
        this.controller.items.add(item);
    }

    private async run(request: vscode.TestRunRequest, token: vscode.CancellationToken, review: boolean): Promise<void> {
        if (!request.include && !this.controller.items.size) await this.discover();
        const excluded = new Set(request.exclude?.map(item => item.id));
        const included = request.include ? [...request.include] : [];
        if (!request.include) this.controller.items.forEach(item => included.push(item));
        const items = included.filter(item => !excluded.has(item.id));
        const run = this.controller.createTestRun(request);
        const cancellation = new AbortController();
        const subscription = token.onCancellationRequested(() => cancellation.abort());
        if (token.isCancellationRequested) cancellation.abort();
        items.forEach(item => run.enqueued(item));
        try {
            for (const item of items) {
                if (cancellation.signal.aborted) {
                    run.skipped(item);
                    continue;
                }
                run.started(item);
                const started = performance.now();
                try {
                    if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before running native scenarios.');
                    const report = review
                        ? await this.reviews.run(item.uri!, cancellation.signal, scenarioBaselineRef(item.uri!), true)
                        : await this.runner.run(item.uri!, cancellation.signal);
                    this.showReport(run, item, report, performance.now() - started);
                } catch (error) {
                    if (cancellation.signal.aborted) run.skipped(item);
                    else run.errored(item, new vscode.TestMessage(errorMessage(error)));
                }
            }
        } finally {
            subscription.dispose();
            run.end();
        }
    }

    private showReport(run: vscode.TestRun, item: vscode.TestItem, report: ScenarioReport, duration: number): void {
        for (const notice of report.notices) run.appendOutput(`${notice}\r\n`, undefined, item);
        for (const step of report.steps) {
            const location = item.uri && new vscode.Location(item.uri, new vscode.Position(0, 0));
            run.appendOutput(`Step ${step.step} · ${step.message}\r\n`, location, item);
        }
        if (report.passed) {
            run.passed(item, duration);
            return;
        }
        const failed = report.steps.find(step => !step.passed);
        const markdown = new vscode.MarkdownString();
        markdown.appendText(failed ? `Step ${failed.step} · ${failed.message}` : 'The native scenario failed.');
        for (const artifact of failed?.artifacts ?? []) {
            markdown.appendMarkdown(`\n\n[${path.basename(artifact).replace(/[\[\]]/g, '')}](${vscode.Uri.file(artifact).toString()})`);
        }
        run.failed(item, new vscode.TestMessage(markdown), duration);
    }

    dispose(): void {
        this.disposed = true;
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

import * as path from 'node:path';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import { ScenarioRunner, scenarioBaselineRef, scenarioPattern, type ScenarioReport, type VisualReview } from './scenarioRunner';
import { scenarioReviewHtml, webviewOptions } from './view';
import { isInside } from './workspacePaths';

interface ReviewPanel {
    panel: vscode.WebviewPanel;
    report: ScenarioReport;
    reference?: string;
    cancellation: AbortController;
    busy: boolean;
}

export class ScenarioReview implements vscode.Disposable {
    private readonly panels = new Map<string, ReviewPanel>();
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly context: vscode.ExtensionContext, private readonly runner: ScenarioRunner) {
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.scenario.review', (uri?: vscode.Uri) => this.choose(uri, false)),
            vscode.commands.registerCommand('ferroui.scenario.compare', (uri?: vscode.Uri) => this.choose(uri, true)),
        ];
    }

    async run(uri: vscode.Uri, signal: AbortSignal, reference?: string, open = false): Promise<ScenarioReport> {
        const report = await this.runner.run(uri, signal, { review: true, reference });
        signal.throwIfAborted();
        if (open) this.show(uri, report, reference);
        return report;
    }

    show(uri: vscode.Uri, report: ScenarioReport, reference?: string): void {
        const key = uri.toString();
        let review = this.panels.get(key);
        if (review) {
            review.report = report;
            review.reference = reference;
            review.panel.title = reviewTitle(report);
            review.panel.reveal();
            this.publish(review);
            return;
        }
        const root = this.context.extensionUri;
        const options = webviewOptions(root);
        const panel = vscode.window.createWebviewPanel('ferroui.baselineReview', reviewTitle(report), vscode.ViewColumn.Beside, {
            ...options,
            localResourceRoots: [...options.localResourceRoots!, vscode.Uri.file(path.dirname(report.artifactDirectory))],
        });
        const state: ReviewPanel = { panel, report, reference, cancellation: new AbortController(), busy: false };
        this.panels.set(key, state);
        panel.onDidDispose(() => {
            state.cancellation.abort();
            this.panels.delete(key);
        });
        panel.webview.onDidReceiveMessage(message => { void this.receive(uri, state, message); });
        panel.webview.html = scenarioReviewHtml(panel.webview, root);
    }

    private async choose(uri: vscode.Uri | undefined, compare: boolean): Promise<void> {
        try {
            const active = uri ?? vscode.window.activeTextEditor?.document.uri;
            const file = active?.fsPath.endsWith('.ferroui-scenario.json') ? active : await this.selectScenario();
            if (!file) return;
            const configured = scenarioBaselineRef(file);
            const reference = compare ? await vscode.window.showInputBox({
                title: 'Compare Scenario Baselines with Git',
                prompt: 'Choose a local branch, tag, or commit to find the merge base',
                value: configured ?? 'origin/main',
                validateInput: text => text.trim() ? undefined : 'Enter a Git reference.',
            }) : configured;
            if (compare && reference === undefined) return;
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification, title: 'Reviewing FerroUI baselines', cancellable: true,
            }, async (_, token) => {
                const cancellation = new AbortController();
                const subscription = token.onCancellationRequested(() => cancellation.abort());
                try {
                    if (token.isCancellationRequested) return;
                    await this.run(file, cancellation.signal, reference?.trim(), true);
                } catch (error) {
                    if (!cancellation.signal.aborted) throw error;
                } finally { subscription.dispose(); }
            });
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    private async selectScenario(): Promise<vscode.Uri | undefined> {
        const files = await vscode.workspace.findFiles(scenarioPattern, '**/{.git,node_modules,target}/**');
        if (!files.length) throw new Error('Open a workspace with a saved FerroUI scenario.');
        const choices = files.map(uri => ({
            uri, label: path.basename(uri.fsPath, '.ferroui-scenario.json'), description: vscode.workspace.asRelativePath(uri),
        })).sort((a, b) => a.description.localeCompare(b.description));
        const selected = await vscode.window.showQuickPick(choices, { title: 'Review FerroUI Baselines', matchOnDescription: true });
        return selected?.uri;
    }

    private async receive(uri: vscode.Uri, state: ReviewPanel, message: { type?: string; step?: unknown; approval?: unknown }): Promise<void> {
        if (message?.type === 'ready') {
            this.publish(state);
            return;
        }
        if (message?.type === 'source') {
            await vscode.commands.executeCommand('vscode.open', uri);
            return;
        }
        if (state.busy || state.cancellation.signal.aborted) return;
        const approval = state.report.visuals?.find(item => item.step === message?.step)?.approval;
        if (message?.type !== 'refresh' && !(message?.type === 'approve' && approval && approval === message.approval)) return;
        const displayed = state.report;
        state.busy = true;
        void state.panel.webview.postMessage({ type: 'status', busy: true, text: approval ? 'Verifying the reviewed capture…' : 'Running the scenario…' });
        try {
            const report = await this.runner.run(uri, state.cancellation.signal, {
                review: true, approval: message.type === 'approve' ? approval ?? undefined : undefined, reference: state.reference,
            });
            if (state.cancellation.signal.aborted || state.report !== displayed) return;
            state.report = report;
            this.publish(state);
        } catch (error) {
            if (!state.cancellation.signal.aborted) {
                void state.panel.webview.postMessage({ type: 'status', busy: false, error: true, text: errorMessage(error) });
            }
        } finally {
            state.busy = false;
        }
    }

    private publish(state: ReviewPanel): void {
        const { report, panel } = state;
        const image = (file: string | null) => file && isInside(report.artifactDirectory, file)
            ? panel.webview.asWebviewUri(vscode.Uri.file(file)).toString() : null;
        const capture = ({ actual, expected, difference, ...item }: VisualReview) => ({
            ...item,
            baseline: vscode.workspace.asRelativePath(vscode.Uri.file(item.baseline), false),
            actual: image(actual),
            expected: image(expected),
            difference: image(difference),
        });
        const issue = report.steps.find(step => !step.passed && !report.visuals?.some(item => item.step === step.step));
        void panel.webview.postMessage({
            type: 'review',
            captures: report.visuals?.map(capture) ?? [],
            reference: report.reference,
            approved: report.approved ? vscode.workspace.asRelativePath(vscode.Uri.file(report.approved), false) : undefined,
            error: issue ? `Step ${issue.step} · ${issue.message}` : undefined,
        });
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        for (const { panel } of this.panels.values()) panel.dispose();
        this.panels.clear();
    }
}

function reviewTitle(report: ScenarioReport): string {
    return `Baseline Review · ${report.name}`;
}

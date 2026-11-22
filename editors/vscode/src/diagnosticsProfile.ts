import { Buffer } from 'node:buffer';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import type { RuntimeTarget } from './runtimeConnections';
import type { ProfilePhase, ProfileReport, ProfileRow, RuntimeProfiles } from './runtimeProfiles';
import { choose } from './studioContext';

const phaseLabels = { measure: 'Measure', arrange: 'Arrange', render: 'RSX render', reconcile: 'Reconciliation' };
const runtimeLabels: Record<RuntimeTarget, string> = { managed: 'Managed app', attached: 'Attached app' };
const request = () => AbortSignal.timeout(15_000);

export type ProfileItem =
    | { type: 'profileInfo'; report: ProfileReport }
    | { type: 'profileGroup'; report: ProfileReport; kind: ProfileRow['kind'] }
    | { type: 'profileRow'; report: ProfileReport; row: ProfileRow }
    | { type: 'profilePhase'; report: ProfileReport; row: ProfileRow; phase: ProfilePhase };

export class DiagnosticsProfile implements vscode.Disposable {
    private readonly commands: vscode.Disposable[];

    constructor(private readonly profiles: RuntimeProfiles) {
        this.commands = [
            vscode.commands.registerCommand('ferroui.diagnostics.recordProfile', () => this.run(() => this.record())),
            vscode.commands.registerCommand('ferroui.diagnostics.stopProfile', () => this.run(async () => {
                const report = this.profiles.report;
                if (report?.recording) await this.profiles.stop(report, report.id, request());
            })),
            vscode.commands.registerCommand('ferroui.diagnostics.exportProfile', () => this.run(() => this.export())),
            vscode.commands.registerCommand('ferroui.diagnostics.profileSource', (item?: ProfileItem) => this.run(async () => {
                if (item?.type === 'profileRow') await this.profiles.sourceAt(item.report, item.report.id, item.row.row_id, request(), true);
            })),
        ];
    }

    get context(): string {
        const report = this.profiles.report;
        return report?.recording && this.profiles.isCurrent(report) ? 'profileRecording' : 'profileReady';
    }

    roots(): Array<ProfileItem | { type: 'notice'; label: string }> {
        const report = this.profiles.report;
        if (!report) return [{ type: 'notice', label: 'Record a UI Profile while using a running app' }];
        const result: ReturnType<DiagnosticsProfile['roots']> = [{ type: 'profileInfo', report }];
        if (this.profiles.error) result.push({ type: 'notice', label: this.profiles.error });
        if (report.omitted_calls) result.push({ type: 'notice',
            label: report.omitted_calls + ' calls omitted after reaching the capture limit' });
        if (!report.rows.length) return [...result, { type: 'notice', label: report.recording
            ? 'Waiting for native layout or component work' : 'No UI work was recorded' }];
        for (const kind of ['control', 'component'] as const) {
            if (report.rows.some(row => row.kind === kind)) result.push({ type: 'profileGroup', report, kind });
        }
        return result;
    }

    children(item: ProfileItem): ProfileItem[] {
        const { report } = item;
        if (item.type === 'profileGroup') return report.rows.filter(row => row.kind === item.kind)
            .map(row => ({ type: 'profileRow', report, row }));
        if (item.type === 'profileRow') return item.row.phases.map(phase => ({
            type: 'profilePhase', report, row: item.row, phase,
        }));
        return [];
    }

    item(item: ProfileItem): vscode.TreeItem {
        const { report } = item;
        const id = 'ui-profile:' + report.runtime + ':' + report.generation + ':' + report.id;
        if (item.type === 'profileInfo') {
            const state = !this.profiles.isCurrent(report) ? 'Application disconnected'
                : !report.window_open ? 'Window closed' : report.recording ? 'Recording' : 'Capture complete';
            const tree = new vscode.TreeItem(state);
            tree.id = id;
            tree.description = (report.elapsed_ms / 1000).toFixed(1) + ' s';
            const location = runtimeLabels[report.runtime] + ' · Window ' + report.window_id;
            tree.tooltip = report.omitted_calls ? location + '\n' + report.omitted_calls + ' calls omitted' : location;
            tree.iconPath = new vscode.ThemeIcon(report.recording ? 'record' : 'graph');
            tree.contextValue = 'profileReport';
            return tree;
        }
        if (item.type === 'profileGroup') {
            const tree = new vscode.TreeItem(item.kind === 'control' ? 'Native Layout' : 'RSX Components', vscode.TreeItemCollapsibleState.Collapsed);
            const count = report.rows.filter(row => row.kind === item.kind).length;
            tree.id = id + ':' + item.kind;
            tree.description = String(count);
            tree.iconPath = new vscode.ThemeIcon(item.kind === 'control' ? 'layout' : 'symbol-class');
            return tree;
        }
        const rowId = id + ':' + item.row.row_id;
        if (item.type === 'profilePhase') {
            const { phase } = item;
            const tree = new vscode.TreeItem(phaseLabels[phase.phase]);
            tree.id = rowId + ':' + phase.phase;
            tree.description = time(phase.self_ms) + ' self · ' + phase.calls + ' calls';
            tree.tooltip = time(phase.total_ms) + ' including nested work\n' + time(phase.max_self_ms) + ' longest self time';
            return tree;
        }
        const { row } = item;
        const label = row.name && row.kind === 'control' ? row.label + ' #' + row.name : row.label;
        const tree = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.Collapsed);
        tree.id = rowId;
        tree.description = time(row.self_ms) + ' self · ' + row.phases.reduce((sum, phase) => sum + phase.calls, 0) + ' calls';
        tree.tooltip = row.phases.map(phase => phaseLabels[phase.phase] + ' · ' + time(phase.self_ms) + ' self · ' + phase.calls + ' calls').join('\n');
        tree.accessibilityInformation = { label: label + '. ' + tree.description };
        tree.iconPath = new vscode.ThemeIcon(row.kind === 'control' ? 'symbol-field' : 'symbol-class');
        if (row.has_source && row.node_id !== null && this.profiles.isCurrent(report)) tree.contextValue = 'profileSource';
        return tree;
    }

    private async record(): Promise<void> {
        const targets = this.profiles.targets();
        if (!targets.length) throw new Error('Start or attach a FerroUI application with MCP.');
        const target = targets.length === 1 ? targets[0]
            : (await choose('Application to profile', targets.map(runtime => ({ label: runtimeLabels[runtime], runtime }))))?.runtime;
        if (!target) return;
        const session = await this.profiles.windows(request(), target);
        if (!session.windows.length) throw new Error('Open a window in the running application.');
        const window = session.windows.length === 1 ? session.windows[0]
            : (await choose('Window to profile', session.windows.map(window => ({
                label: window.text || window.name || window.type, description: 'Window ' + window.node_id, window,
            }))))?.window;
        if (!window) return;
        const capture = await choose('UI profile duration', [5, 10, 20, 30].map(seconds => ({
            label: seconds + ' seconds', seconds,
        })));
        if (capture) await this.profiles.start(session, window.node_id, capture.seconds * 1000, request());
    }

    private async export(): Promise<void> {
        const report = this.profiles.report;
        if (!report) return;
        const folder = vscode.workspace.workspaceFolders?.[0]?.uri;
        const uri = await vscode.window.showSaveDialog({
            title: 'Export UI Profile',
            defaultUri: folder && vscode.Uri.joinPath(folder, 'ferroui-ui-profile.json'),
            filters: { JSON: ['json'] },
        });
        if (uri) await vscode.workspace.fs.writeFile(uri, Buffer.from(JSON.stringify(report, null, 2) + '\n'));
    }

    private async run(action: () => Promise<void>): Promise<void> {
        try { await action(); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void { this.commands.forEach(command => command.dispose()); }
}

function time(ms: number): string {
    return ms > 0 && ms < .01 ? '<0.01 ms' : ms.toFixed(2) + ' ms';
}

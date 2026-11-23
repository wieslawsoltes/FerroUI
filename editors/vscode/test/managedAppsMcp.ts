import { strict as assert } from 'node:assert';
import { readFile, unlink, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { RuntimeClient } from '../src/runtimeClient';
import type { RuntimeNode } from '../src/runtimeProtocol';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { verifyRuntimeMcpAccess } from './runtimeMcpAccess';
import { withManagedAppFixture } from './managedAppFixture';
import { verifyHostedInputMcp } from './hostedInputMcp';
import { liveAppControl, liveAppView, verifyLiveAppPresentation } from './liveAppPresentationMcp';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface Run { session: string; runtime: RuntimeSummary['runtime']; pid: number | null; status: string; mcpAvailable: boolean }

export async function verifyManagedApps({ invoke, content }: McpFixture): Promise<void> {
    const executable = process.env.FERROUI_TEST_HOSTED_APP;
    if (!executable || process.platform === 'win32') return;
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    await withManagedAppFixture(workspace, async ({ root, files, labels, expected }) => {
        const runs: Run[] = [];
        const tools = async <T>(name: string, args: object = {}) => content<T>(await invoke(name, args));
        const list = () => tools<{ managed: Run[]; runtimes: RuntimeSummary[] }>('ferroui_app_sessions');
        const status = (run: Run) => tools<Run>('ferroui_reload_status', { app_session: run.session });
        const control = (action: string, run?: Run) => invoke('ferroui_reload_control', { action, app_session: run?.session });
        const endpointFor = async (label: string) => {
            const definitions = await vscode.commands.executeCommand<vscode.McpServerDefinition[]>('ferroui.test.mcpDefinitions');
            const definition = definitions.find(def => def instanceof vscode.McpHttpServerDefinition && def.label.includes(label)) as vscode.McpHttpServerDefinition | undefined;
            if (!definition) throw new Error(`No inspector for ${label}`);
            return { url: definition.uri.toString(), token: definition.headers.Authorization.replace(/^Bearer /, '') };
        };
        const clientFor = async (label: string) => new RuntimeClient(await endpointFor(label));
        const save = async (file: string, draft: string) => {
            const source = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
            const edit = new vscode.WorkspaceEdit();
            edit.replace(source.uri, new vscode.Range(new vscode.Position(0, 0), source.positionAt(source.getText().length)), draft);
            assert.ok(await vscode.workspace.applyEdit(edit) && await source.save());
        };
        const focus = async (run: Run, label: string) => {
            await tools('ferroui_runtime_open', { runtime: run.runtime });
            await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === `Live App · ${label}`)));
        };
        const clients: RuntimeClient[] = [];
        const attachments: RuntimeSummary[] = [];
        try {
            const targets = (await tools<{ targets: { label: string }[] }>('ferroui_reload_targets')).targets;
            assert.deepEqual(targets.map(target => target.label), labels);
            const rejection = path.join(root, 'account-desk', 'reject-build');
            await writeFile(rejection, 'reject the first left app build');
            await Promise.all(targets.map(async (target, index) => {
                runs[index] = await tools('ferroui_reload_control', { action: 'start', ...target });
            }));
            const [left, right] = runs;
            await until(async () => (await status(left)).status === 'Launch failed' && (await status(right)).mcpAvailable);
            const independent = await status(right);
            await unlink(rejection);
            const retry = await tools<Run>('ferroui_reload_control', { action: 'start', ...targets[0] });
            assert.equal(retry.session, left.session);
            await until(async () => (await list()).runtimes.filter(target => target.available).length === 2);
            const first = await status(left);
            const second = await status(right);
            assert.ok(first.pid && second.pid && first.pid !== second.pid);
            assert.equal(second.pid, independent.pid);
            assert.ok((await control('stop')).isError, 'Stopping one of several runtimes needs an exact app session');
            const reopened = await tools<Run>('ferroui_reload_control', { action: 'start', ...targets[0] });
            assert.deepEqual([reopened.session, reopened.pid], [left.session, first.pid]);
            for (let index = 0; index < labels.length; index++) clients.push(await clientFor(labels[index]));
            const [leftClient, rightClient] = clients;
            const headings = await Promise.all(clients.map(client => client.call<RuntimeNode[]>('find_elements', { name: 'Heading' })));
            assert.deepEqual(headings.map(nodes => nodes[0].text), expected);
            await verifyRuntimeMcpAccess({ invoke, content }, left.runtime);
            const sources = await Promise.all(runs.map((run, index) => tools<{ runtime: string; generation: number; source: { uri: string; version: number } }>(
                'ferroui_runtime_source', { runtime: run.runtime, node: headings[index][0].node_id })));
            assert.notEqual(sources[0].generation, sources[1].generation);
            assert.deepEqual(sources.map(source => source.runtime), runs.map(run => run.runtime));
            assert.deepEqual(sources.map(source => path.basename(vscode.Uri.parse(source.source.uri).fsPath)), ['Window.xaml', 'Window.xaml']);
            assert.ok(sources[0].source.uri.includes('account-desk') && sources[1].source.uri.includes('routing-queue'));
            const wrongApp = await invoke('ferroui_runtime_apply_source', { runtime: right.runtime, node: headings[1][0].node_id,
                expected_generation: sources[0].generation, expected_version: sources[1].source.version, property: 'Text', value: 'Wrong app' });
            assert.match(wrongApp.content[0]?.text ?? '', /current application generation/);
            const rejected = await invoke('ferroui_runtime_attach', { url: (await endpointFor(labels[0])).url, token: '0'.repeat(64) });
            assert.match(rejected.content[0]?.text ?? '', /authenticate/);
            for (const [index, label] of labels.entries()) {
                const endpoint = await endpointFor(label);
                const attached = await tools<RuntimeSummary>('ferroui_runtime_attach', { ...endpoint, label: `Inspector ${index + 1}` });
                assert.ok(!Object.hasOwn(attached, 'token'));
                attachments.push(attached);
            }
            assert.notEqual(attachments[0].generation, attachments[1].generation);
            const reattached = await tools<RuntimeSummary>('ferroui_runtime_attach', await endpointFor(labels[0]));
            assert.equal(reattached.runtime, attachments[0].runtime);
            assert.notEqual(reattached.generation, attachments[0].generation);
            await tools('ferroui_runtime_detach', { runtime: attachments[0].runtime });
            assert.deepEqual((await list()).runtimes.filter(target => target.kind === 'attached').map(target => target.runtime), [attachments[1].runtime]);
            await tools('ferroui_runtime_detach', { runtime: attachments[1].runtime });
            attachments.length = 0;
            await focus(left, labels[0]);
            await until(async () => (await liveAppView({ invoke, content }, left.runtime)()).frame !== null);
            await focus(right, labels[1]);
            await until(async () => (await liveAppView({ invoke, content }, right.runtime)()).frame !== null);
            const oldView = await verifyLiveAppPresentation({ invoke, content }, left.runtime, right.runtime, headings[0][0].node_id);
            content(await control('select', left));
            assert.equal(await vscode.commands.executeCommand('ferroui.hotReload.processId'), String(first.pid));
            const [search] = await rightClient.call<RuntimeNode[]>('find_elements', { name: 'Search' });
            await rightClient.call('type_text', { node_id: search.node_id, text: 'Retain this filter' });

            const draft = (await readFile(files[0], 'utf8')).replace(expected[0], 'Approved customer records');
            await writeFile(rejection, 'fail only the left app');
            await save(files[0], draft);
            await until(async () => (await status(left)).status.includes('Update failed'));
            assert.equal((await status(left)).pid, first.pid);
            assert.equal((await status(right)).pid, second.pid);
            assert.equal((await leftClient.call<RuntimeNode[]>('find_elements', { name: 'Heading' }))[0].text, expected[0]);
            await unlink(rejection);
            content(await control('rebuild', left));
            await until(async () => { const current = await status(left); return Boolean(current.mcpAvailable && current.pid !== first.pid); });
            const [rebuilt] = (await list()).runtimes.filter(target => target.runtime === left.runtime);
            assert.notEqual(rebuilt.generation, sources[0].generation);
            const retired = await invoke('ferroui_runtime_query', {
                runtime: left.runtime, expected_generation: sources[0].generation, tool: 'list_windows',
            });
            assert.match(retired.content[0]?.text ?? '', /application connection changed/);
            const retiredView = await invoke('ferroui_live_app_control', liveAppControl(oldView, { action: 'refresh' }));
            assert.match(retiredView.content[0]?.text ?? '', /Live App editor changed/);
            assert.equal((await status(right)).pid, second.pid);
            assert.ok((await rightClient.call<{ matched: boolean }>('wait_for', { name: 'Result', text: 'Retain this filter' })).matched);
            const update = await clientFor(labels[0]);
            clients.push(update);
            assert.equal((await update.call<RuntimeNode[]>('find_elements', { name: 'Heading' }))[0].text, 'Approved customer records');
            const leftHistory = await tools<{ records: { target: string; outcome: string }[] }>('ferroui_reload_history', { app_session: left.session });
            const rightHistory = await tools<{ records: unknown[] }>('ferroui_reload_history', { app_session: right.session });
            assert.ok(leftHistory.records.some(record => record.outcome === 'buildFailed' && record.target === labels[0]));
            assert.equal(rightHistory.records.length, 0);

            await save(path.join(root, 'shared', 'src', 'lib.rs'), 'pub const WORKSPACE: &str = "Northwind Studio";\n');
            await until(async () => {
                const active = (await list()).runtimes;
                const appA = active.find(item => item.runtime === left.runtime);
                const appB = active.find(item => item.runtime === right.runtime);
                return Boolean(appA?.available && appB?.available && appA.pid !== rebuilt.pid && appB.pid !== second.pid);
            });
            const rightAfterShared = await status(right);
            assert.notEqual(rightAfterShared.pid, second.pid);
            content(await control('stop', left));
            assert.deepEqual((await list()).managed.map(run => [run.session, run.pid]), [[right.session, rightAfterShared.pid]]);
            const remaining = await clientFor(labels[1]);
            clients.push(remaining);
            assert.ok(await remaining.supports('list_windows'));
            await verifyHostedInputMcp({ invoke, content }, files[0], targets[0]);
            console.log('Two native apps isolated their Live App tabs, inspectors, reload failures, and stops. Shared dependency saves rebuilt both scoped targets');
        } finally {
            clients.forEach(client => client.dispose());
            for (const attachment of attachments) await invoke('ferroui_runtime_detach', { runtime: attachment.runtime });
            for (const run of runs.filter(Boolean)) await control('stop', run);
            const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => labels.some(label => tab.label === `Live App · ${label}`));
            await vscode.window.tabGroups.close(tabs);
        }
    });
}

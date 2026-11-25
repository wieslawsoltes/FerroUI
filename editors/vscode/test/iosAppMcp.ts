import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { verifyIosAppDebug } from './iosAppDebugMcp';
import { openEditorMcp } from './mcpFixture';
import { until } from './wait';

const profile = 'contact-review';

export async function verifyIosAppMcp(device: string, kind: 'simulator' | 'ios-device'): Promise<void> {
    const tool = kind === 'simulator' ? 'ferroui_ios_simulator' : 'ferroui_ios_device';
    const command = kind === 'simulator' ? 'ferroui.iosSimulator.run' : 'ferroui.iosDevice.run';
    const iosTarget = (session: RuntimeSummary) => kind === 'simulator' ? session.simulator : session.iosDevice;
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const sourceUri = vscode.Uri.joinPath(folder, 'src/iOS/FerroUI.iOS/examples/simulator_inspector.rs');
    const document = await vscode.workspace.openTextDocument(sourceUri);
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('ferroui.showAppSessions');

    const mcp = await openEditorMcp(80_000);
    const { invoke, content, close } = mcp;
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = config.inspect<boolean>('allowWrites')?.globalValue;
    const request = { device, profile, folder: folder.toString() };
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    try {
        const targets = content<{ supported: boolean; devices: { id: string; booted?: boolean; status?: string }[];
            apps: { id: string; folder: string; installsBundle: boolean }[] }>(
            await invoke(`${tool}_targets`, { folder: request.folder }));
        assert.equal(targets.supported, true);
        assert.ok(targets.devices.some(target => target.id.toLowerCase() === device.toLowerCase()
            && (target.booted || target.status === 'ready')), 'The selected iOS test target is ready');
        assert.ok(targets.apps.some(app => app.id === profile && app.folder === request.folder && app.installsBundle));
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.match((await invoke(`${tool}_run`, request)).content[0]?.text ?? '', /allowWrites/);
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);

        const started = content<RuntimeSummary>(await invoke(`${tool}_run`, request));
        const { runtime, generation } = started;
        assert.equal(started.kind, kind);
        assert.ok(runtime.startsWith(`${kind}:`));
        assert.equal(iosTarget(started)?.device.toLowerCase(), device.toLowerCase());
        assert.equal(started.available, true);
        assert.ok(generation);
        assert.equal(content<RuntimeSummary>(await invoke(`${tool}_run`, request)).runtime, runtime);
        const debug = { runtime, expected_generation: generation };
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.match((await invoke(`${tool}_debug`, debug)).content[0]?.text ?? '', /allowWrites/);
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        assert.match((await invoke(`${tool}_debug`, { ...debug, expected_generation: generation + 1 })).content[0]?.text ?? '',
            /current accepted iOS (simulator|device) generation/);
        const query = async <T>(tool: string, args: object = {}) =>
            content<{ result: T }>(await invoke('ferroui_runtime_query', {
                runtime, expected_generation: generation, tool, arguments: args,
            })).result;
        const windows = await query<RuntimeNode[]>('list_windows');
        assert.equal(windows.length, 1);
        const nodes = await query<RuntimeNode[]>('get_tree', { node_id: windows[0].node_id, depth: 12 });
        assert.equal(nodes[0]?.node_id, windows[0].node_id);
        assert.ok(nodes[0]?.children?.length, 'The native window has a visual hierarchy');
        const screenshot = async () => {
            const response = await invoke('ferroui_runtime_query', { runtime, expected_generation: generation,
                tool: 'screenshot', arguments: { node_id: windows[0].node_id } });
            assert.equal(content<{ generation: number }>(response).generation, generation);
            const png = Buffer.from(response.content.find(item => item.type === 'image')?.data ?? '', 'base64');
            assert.equal(png.subarray(1, 4).toString(), 'PNG');
            assert.ok(png.readUInt32BE(16) > 200 && png.readUInt32BE(20) > 400);
            return png;
        };
        const before = await screenshot();
        const [button] = await query<RuntimeNode[]>('find_elements', { name: 'ReviewContact' });
        assert.equal(button?.name, 'ReviewContact');
        const anchor = content<{ source: { uri: string; language: string; position: { line: number }; canApply: boolean } }>(
            await invoke('ferroui_runtime_source', { runtime, node: button.node_id })).source;
        assert.equal(anchor?.uri, sourceUri.toString());
        assert.equal(anchor.language, 'rust');
        assert.ok(document.lineAt(anchor.position.line).text.includes('ReviewContact'));
        assert.equal(anchor.canApply, true);
        content(await invoke('ferroui_runtime_open', { runtime }));
        await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label.includes(started.label))));

        content(await invoke('ferroui_runtime_call', { runtime, expected_generation: generation,
            tool: 'click', arguments: { node_id: button.node_id } }));
        await until(async () => (await query<{ matched: boolean }>('wait_for', {
            name: 'ReviewedCount', text: '1 contacts reviewed', timeout_ms: 200,
        })).matched);
        assert.notDeepEqual(await screenshot(), before, 'The native frame reflects the interaction');
        if (process.env.FERROUI_TEST_IOS_DEBUG) await verifyIosAppDebug(mcp, started, document, button.node_id);
        const stale = await invoke('ferroui_runtime_query', { runtime, expected_generation: generation + 1, tool: 'list_windows' });
        assert.match(stale.content[0]?.text ?? '', /application connection changed/);
        assert.match((await invoke('ferroui_runtime_detach', { runtime })).content[0]?.text ?? '', /iOS (simulator|device)/i);
        if (kind === 'ios-device') {
            const { output } = content<{ output: string }>(await invoke('ferroui_ios_device_output', { runtime }));
            assert.match(output, /inspector ready/);
        }

        await vscode.commands.executeCommand('ferroui.appSessions.stop', runtime);
        assert.ok(!(await sessions()).some(session => session.runtime === runtime));
        await vscode.commands.executeCommand(command, request);
        const replacement = (await sessions()).find(session => iosTarget(session)?.device.toLowerCase() === device.toLowerCase());
        assert.ok(replacement, 'The VS Code run command publishes a live iOS session');
        assert.notEqual(replacement.runtime, runtime);
        assert.notEqual(replacement.generation, generation);
        await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label.includes(replacement.label))));
        content(await invoke(`${tool}_stop`, { runtime: replacement.runtime }));
        assert.ok(!(await sessions()).some(session => session.runtime === replacement.runtime));
        console.log(`iOS ${kind} discovery, native Live App frames and input, checked RSX source, exact sessions and stop passed`);
    } finally {
        try {
            await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
            for (const session of await sessions()) {
                const target = iosTarget(session);
                if (target?.device.toLowerCase() === device.toLowerCase() && target.profile === profile) {
                    await invoke(`${tool}_stop`, { runtime: session.runtime });
                }
            }
        } finally {
            await config.update('allowWrites', previous, vscode.ConfigurationTarget.Global);
            close();
        }
    }
}

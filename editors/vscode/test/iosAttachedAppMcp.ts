import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { openEditorMcp } from './mcpFixture';
import { until } from './wait';

export async function verifyIosAttachedAppMcp(device: string, port: number, token: string): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri.toString();
    const request = { device, profile: 'contact-review', folder, port, token };
    const configuration = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = configuration.inspect<boolean>('allowWrites')?.globalValue;
    const { invoke, content, close } = await openEditorMcp(40_000);
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    const owned = async () => (await sessions()).find(session => session.iosDevice?.attached
        && session.iosDevice.device.toLowerCase() === device.toLowerCase() && session.iosDevice.profile === request.profile);
    const readWindows = async (session: RuntimeSummary) => content<{ result: RuntimeNode[] }>(await invoke('ferroui_runtime_query', {
        runtime: session.runtime, expected_generation: session.generation, tool: 'list_windows',
    })).result;
    try {
        await configuration.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.match((await invoke('ferroui_ios_device_attach', request)).content[0]?.text ?? '', /allowWrites/);
        await configuration.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const invalid = { ...request, token: (token.startsWith('0') ? '1' : '0') + token.slice(1) };
        assert.ok((await invoke('ferroui_ios_device_attach', invalid)).isError);
        assert.equal(await owned(), undefined);
        const first = content<RuntimeSummary>(await invoke('ferroui_ios_device_attach', request));
        assert.ok(first.iosDevice?.attached && first.available && first.generation && first.pid);
        const definitions = await vscode.commands.executeCommand<vscode.McpServerDefinition[]>('ferroui.test.mcpDefinitions');
        assert.ok(!definitions.some(definition => definition instanceof vscode.McpHttpServerDefinition
            && definition.headers.Authorization === `Bearer ${token}`), 'The USB inspector is available through editor MCP');
        assert.equal(content<RuntimeSummary>(await invoke('ferroui_ios_device_attach', request)).runtime, first.runtime);
        const [window] = await readWindows(first);
        assert.ok(window?.bounds?.width);
        const frame = await invoke('ferroui_runtime_query', { runtime: first.runtime, expected_generation: first.generation,
            tool: 'screenshot', arguments: { node_id: window.node_id } });
        assert.ok(frame.content.some(image => image.type === 'image' && image.data?.startsWith('iVBORw0KGgo')));
        await vscode.commands.executeCommand('ferroui.appSessions.stop', first.runtime);
        assert.equal(await owned(), undefined);

        await vscode.commands.executeCommand('ferroui.iosDevice.attach', request);
        const replacement = await owned();
        assert.ok(replacement?.available && replacement.generation);
        assert.equal(replacement.pid, first.pid, 'Disconnect leaves the independently running app process alive');
        assert.notEqual(replacement.generation, first.generation);
        assert.notEqual(replacement.runtime, first.runtime);
        const stale = await invoke('ferroui_runtime_query', { runtime: replacement.runtime,
            expected_generation: first.generation, tool: 'list_windows' });
        assert.ok(stale.isError, 'An old agent request cannot use the replacement attachment');
        await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label.includes(replacement.label))));
        assert.ok((await readWindows(replacement)).length);
        content(await invoke('ferroui_ios_device_stop', { runtime: replacement.runtime }));
        assert.equal(await owned(), undefined);
        console.log('Physical iOS USB attach, native frame, workbench reattach to the same PID, and MCP disconnect passed');
    } finally {
        try {
            await configuration.update('allowWrites', true, vscode.ConfigurationTarget.Global);
            const session = await owned();
            if (session) await invoke('ferroui_ios_device_stop', { runtime: session.runtime });
        } finally {
            await configuration.update('allowWrites', previous, vscode.ConfigurationTarget.Global);
            close();
        }
    }
}

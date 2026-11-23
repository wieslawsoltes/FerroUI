import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeToolDefinition } from '../src/runtimeClient';
import type { RuntimeNode } from '../src/runtimeProtocol';
import type { AppRuntime } from '../src/runtimeTargets';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRuntimeMcpAccess({ invoke, content }: McpFixture, runtime: AppRuntime): Promise<void> {
    const catalog = async () => content<{ runtime: AppRuntime; generation: number; tools: RuntimeToolDefinition[] }>(
        await invoke('ferroui_runtime_tools', { runtime }));
    const { generation, tools } = await catalog();
    const query = (tool: string, arguments_: object = {}) =>
        invoke('ferroui_runtime_query', { runtime, expected_generation: generation, tool, arguments: arguments_ });
    const call = (tool: string, arguments_: object) =>
        invoke('ferroui_runtime_call', { runtime, expected_generation: generation, tool, arguments: arguments_ });
    assert.equal(tools.find(tool => tool.name === 'list_windows')?.annotations?.readOnlyHint, true);
    assert.equal(tools.find(tool => tool.name === 'type_text')?.annotations?.readOnlyHint, false);
    const retired = await invoke('ferroui_runtime_query', { runtime, expected_generation: generation + 1, tool: 'list_windows' });
    assert.match(retired.content[0]?.text ?? '', /application connection changed/);
    assert.match((await query('choose_a_missing_tool')).content[0]?.text ?? '', /Choose a tool/);
    assert.match((await query('type_text')).content[0]?.text ?? '', /ferroui_runtime_call/);

    const settings = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = settings.inspect<boolean>('allowWrites')?.globalValue;
    assert.equal(previous, true);
    const find = async (name: string) => content<{ result: RuntimeNode[] }>(await query('find_elements', { name })).result[0];
    const search = await find('Search');
    const input = { node_id: search.node_id, text: 'Checked editor bridge' };
    try {
        await settings.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.equal((await catalog()).generation, generation);
        const windows = content<{ runtime: AppRuntime; generation: number; result: RuntimeNode[] }>(await query('list_windows'));
        assert.equal(windows.runtime, runtime);
        assert.equal(windows.generation, generation);
        assert.ok(windows.result.length);
        const screenshot = await query('screenshot', { node_id: windows.result[0].node_id });
        assert.equal(content<{ generation: number }>(screenshot).generation, generation);
        const png = screenshot.content.find(item => item.type === 'image')?.data;
        assert.ok(png && Buffer.from(png, 'base64').subarray(1, 4).toString() === 'PNG');
        assert.match((await call('type_text', input)).content[0]?.text ?? '', /allowWrites/);
    } finally {
        await settings.update('allowWrites', previous, vscode.ConfigurationTarget.Global);
    }
    content(await call('type_text', input));
    await until(async () => content<{ result: { matched: boolean } }>(
        await query('wait_for', { name: 'Result', text: input.text })).result.matched);
}

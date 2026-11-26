import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeNode, RuntimeViewChange, RuntimeViewSnapshot } from '../src/runtimeProtocol';
import type { AppRuntime } from '../src/runtimeTargets';
import { runtimeNodePath } from '../src/runtimeTree';
import type { McpFixture } from './mcpFixture';
import { until } from './wait';

export function liveAppView({ invoke, content }: McpFixture, runtime: AppRuntime) {
    return async () => content<RuntimeViewSnapshot>(await invoke('ferroui_live_app_view', { runtime }));
}

export function liveAppControl(view: RuntimeViewSnapshot, change: RuntimeViewChange) {
    return { runtime: view.runtime, expected_generation: view.generation, expected_revision: view.revision, change };
}

export async function verifyLiveAppPresentation(mcp: McpFixture, left: AppRuntime, right: AppRuntime, heading: number) {
    const { invoke, content } = mcp;
    const read = liveAppView(mcp, left);
    const readOther = liveAppView(mcp, right);
    const original = await read();
    const other = await readOther();
    assert.ok(original.window !== null && original.frame !== null);
    assert.ok(original.windows.some(window => window.node_id === original.window));
    const control = (view: RuntimeViewSnapshot, change: RuntimeViewChange) => invoke('ferroui_live_app_control', liveAppControl(view, change));
    const change = async (view: RuntimeViewSnapshot, action: RuntimeViewChange) => content<RuntimeViewSnapshot>(await control(view, action));

    const filter = { action: 'filter', field: 'treeFilter', query: 'Heading' } as const;
    const configuration = vscode.workspace.getConfiguration('ferroui.mcp');
    const previousWrites = configuration.inspect<boolean>('allowWrites')?.globalValue;
    try {
        await configuration.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.match((await control(original, filter)).content[0]?.text ?? '', /allowWrites/);
        assert.equal((await read()).treeFilter, original.treeFilter);
    } finally { await configuration.update('allowWrites', previousWrites, vscode.ConfigurationTarget.Global); }

    const hierarchy = content<{ result: RuntimeNode[] }>(await invoke('ferroui_runtime_query', {
        runtime: left, expected_generation: original.generation, tool: 'get_tree',
        arguments: { node_id: original.window, tree: 'visual', depth: 10 },
    })).result;
    const branch = runtimeNodePath(hierarchy, heading)?.at(-2)?.node_id;
    assert.ok(branch !== undefined, 'The native heading has a branch in the accepted hierarchy');
    const initialView = await read();
    const fold = (view: RuntimeViewSnapshot) => ({ action: 'disclosure', node: branch, frame: view.frame!, expanded: false } as const);
    const closed = await change(initialView, fold(initialView));
    assert.deepEqual(closed.disclosure, { collapsed: [branch] });
    const filtered = await change(closed, filter);
    const stale = await control(original, { action: 'hierarchy', hierarchy: 'logical' });
    assert.match(stale.content[0]?.text ?? '', /Live App editor changed/);
    assert.equal(filtered.treeFilter, 'Heading');
    const foldedSearch = await change(filtered, fold(filtered));
    assert.deepEqual(foldedSearch.disclosure, { collapsed: [branch], search: { query: 'heading', collapsed: [branch] } });
    const currentHeading = await read();
    const selected = await change(currentHeading, { action: 'select', node: heading, frame: currentHeading.frame!, view: 'properties' });
    assert.deepEqual(selected.selection, { node: heading, view: 'properties' });
    assert.deepEqual(selected.disclosure, { collapsed: [branch], search: { query: 'heading', collapsed: [] } });
    const cleared = await change(selected, { ...filter, query: '' });
    assert.deepEqual(cleared.disclosure.collapsed, [branch]);
    const inspecting = await change(cleared, { action: 'filter', field: 'propertyFilter', query: 'Text' });
    assert.equal(inspecting.propertyFilter, 'Text');
    const interacting = await change(inspecting, { action: 'mode', mode: 'interact' });
    assert.equal(interacting.mode, 'interact');
    const unknown = await control(interacting, { action: 'window', window: Number.MAX_SAFE_INTEGER });
    assert.match(unknown.content[0]?.text ?? '', /current Live App window/);
    const same = await change(interacting, { action: 'window', window: original.window });
    assert.equal(same.revision, interacting.revision);

    const automation = await change(same, { action: 'hierarchy', hierarchy: 'automation' });
    assert.equal(automation.hierarchy, 'automation');
    assert.deepEqual(automation.disclosure, { collapsed: [] });
    assert.equal(automation.selection, null);
    await until(async () => (await read()).frame !== null);
    const beforeRefresh = await read();
    const refreshing = await change(beforeRefresh, { action: 'refresh' });
    assert.equal(refreshing.generation, original.generation);
    assert.equal(refreshing.window, original.window);
    assert.equal(refreshing.frame, null);
    await until(async () => (await read()).frame !== null);
    const current = await read();
    assert.ok(current.frame! > beforeRefresh.frame!);
    const expired = await control(current, { action: 'select', node: original.window, frame: beforeRefresh.frame!, view: 'properties' });
    assert.match(expired.content[0]?.text ?? '', /Live App frame changed/);
    const expiredDisclosure = await control(current, fold(beforeRefresh));
    assert.match(expiredDisclosure.content[0]?.text ?? '', /Live App frame changed/);
    const missingBranch = await control(current, { ...fold(current), node: Number.MAX_SAFE_INTEGER });
    assert.match(missingBranch.content[0]?.text ?? '', /Choose a branch/);
    const diagnostics = await change(current, { action: 'select', node: original.window, frame: current.frame!, view: 'diagnostics' });
    assert.deepEqual(diagnostics.selection, { node: original.window, view: 'diagnostics' });
    const unchanged = await readOther();
    assert.equal(unchanged.generation, other.generation);
    assert.equal(unchanged.revision, other.revision, 'Presentation commands stay pinned to their own editor');
    console.log('Live App MCP verified checked selection, branch disclosure, filters, hierarchy, mode and refresh in two native app editors');
    return diagnostics;
}

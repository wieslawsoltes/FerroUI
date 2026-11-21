import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { DesignFixtureCatalog } from '../src/designDataProtocol';
import type { McpFixture } from './mcp';

export async function verifyDesignData({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const original = document.getText();
    const revision = async () => content<{ revision: number }>(await invoke('designer_wait', { session })).revision;
    const catalog = async () => content<DesignFixtureCatalog>(await invoke('designer_studio_query', {
        session, tool: 'designer_design_fixtures', arguments: {},
    }));
    const edit = async (value: string) => invoke('designer_studio_edit', {
        session, expected_revision: await revision(), tool: 'designer_design_data_edit',
        arguments: { action: 'set_value', node: (await catalog()).node, property: 'Fixtures', value },
    });
    const present = async (action: object) => content(await invoke('designer_studio_present', {
        session, expected_revision: await revision(), tool: 'designer_design_data_edit', arguments: action,
    }));
    const manifest = { default: 'Content', fixtures: [
        { name: 'Content', data: { State: 'Ready', Items: [{ Name: 'Alpine' }], Count: 1 } },
        { name: 'Loading', data: { State: 'Loading', Items: [], Count: 0 } },
    ] };
    assert.ok((await edit(JSON.stringify({ ...manifest, default: 'Missing' }))).isError);
    assert.equal(document.getText(), original);
    content(await edit(JSON.stringify(manifest, null, 2)));
    await revision();
    assert.deepEqual((await catalog()).manifest, manifest);
    const authored = document.getText();
    await present({ action: 'fixture', name: 'Loading' });
    assert.equal((await catalog()).selected, 'Loading');
    content(await invoke('designer_view', { session, refresh: true }));
    await revision();
    assert.equal((await catalog()).selected, 'Loading');
    await present({ action: 'enabled', enabled: false });
    assert.equal((await catalog()).enabled, false);
    assert.equal(document.getText(), authored);
    await present({ action: 'fixture', name: null });
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    console.log('VS Code MCP validated and authored typed named fixtures, preserved the selection through a fresh render, and undid the source transaction');
}

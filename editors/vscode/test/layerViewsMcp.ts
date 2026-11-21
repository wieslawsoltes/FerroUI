import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyLayerViews({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const source = document.getText();
    const revision = async () => content<{ revision: number }>(await invoke('designer_wait', { session })).revision;
    const catalog = async () => content<{ layer_epoch: number; scope_epoch: number;
        layers: { node: number; name: string; locked: boolean; hidden: boolean; canvas_selectable: boolean }[] }>(
        await invoke('designer_studio_query', { session, tool: 'designer_layers', arguments: {} }));
    const layer = (model: Awaited<ReturnType<typeof catalog>>) => {
        const button = model.layers.find(node => node.name === 'saveButton');
        assert.ok(button);
        return button;
    };
    const present = async (args: object) => {
        const expected_revision = await revision();
        const model = await catalog();
        return content(await invoke('designer_studio_present', { session, expected_revision, tool: 'designer_layer', arguments: {
            ...args, expected_layer_epoch: model.layer_epoch, expected_scope_epoch: model.scope_epoch,
        } }));
    };
    const node = layer(await catalog()).node;
    const outline = async (filter?: string) => content<{ total: number; nodes: OutlineNode[] }>(
        await invoke('designer_outline', { session, filter }));
    const full = new Map((await outline()).nodes.map(row => [row.id, row]));
    const path: number[] = [];
    for (let row = full.get(node); row; row = row.parent === null ? undefined : full.get(row.parent)) path.unshift(row.id);
    const matches = await outline('  bUtToN SaVeBuTtOn  ');
    assert.ok(path.length > 1);
    assert.equal(matches.total, path.length);
    assert.deepEqual(matches.nodes.map(row => row.id), path);
    assert.deepEqual((await outline('No such layer')).nodes, []);
    await present({ action: 'set_locked', nodes: [node], locked: true });
    await present({ action: 'set_hidden', nodes: [node], hidden: true });
    assert.equal(document.getText(), source);
    content(await invoke('designer_source', { session, expected_version: document.version, source: source + '\n<!-- Layer refresh -->' }));
    await revision();
    const restored = layer(await catalog());
    assert.deepEqual([restored.locked, restored.hidden, restored.canvas_selectable], [true, true, false]);
    await present({ action: 'reset' });
    assert.equal(layer(await catalog()).canvas_selectable, true);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), source);
    console.log('VS Code MCP filtered the authored hierarchy with its ancestor path and preserved named layer flags across an edit');
}

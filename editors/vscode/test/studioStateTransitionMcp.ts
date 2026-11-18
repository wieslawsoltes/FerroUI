import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { StateTransitionPosition } from '../src/stateTransitionProtocol';
import type { McpFixture } from './mcp';

const scene = `<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Name="Owner">
  <Canvas.Resources><x:Double x:Key="MutedOpacity">0.25</x:Double></Canvas.Resources>
  <Button Name="Status" Opacity="1" Content="Customer status"/>
</Canvas>`;

export async function verifyStateTransition({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const original = document.getText();
    const wait = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    content(await invoke('designer_source', { session, expected_version: document.version, source: scene }));
    let frame = await wait();
    const linear = { kind: 'Linear' };
    const model = { owner: frame.nodes[0].id, name: 'Status', states: [
        { name: 'Normal', setters: [{ target: 'Status', property: 'Opacity', value: '1' }] },
        { name: 'Muted', setters: [{ target: 'Status', property: 'Opacity', value: '{StaticResource MutedOpacity}' }] },
    ], transitions: [
        { from: 'Normal', to: 'Muted', duration: .4, easing: linear },
        { from: null, to: 'Muted', duration: .8, easing: linear },
    ] };
    content(await invoke('designer_studio_edit', { session, expected_revision: frame.revision, tool: 'designer_state',
        arguments: { action: 'upsert', model } }));
    frame = await wait();
    const authored = document.getText();
    const opacity = async () => {
        const node = frame.nodes.find(node => node.name === 'Status')!.id;
        const { properties } = content<{ properties: { name: string; effective: string }[] }>(await invoke('designer_inspect', { session, node }));
        return Number(properties.find(property => property.name === 'Opacity')?.effective);
    };
    const request = (revision: number, args: object) => invoke('designer_transition_control', {
        session, expected_revision: revision, arguments: args,
    });
    const control = async (args: object) => {
        const position = content<StateTransitionPosition>(await request(frame.revision, args));
        frame = await wait();
        return position;
    };
    const retired = frame.revision;
    const opened = await control({ action: 'open', owner: frame.nodes[0].id, group: 'Status', from: 'Normal', to: 'Muted', time: .2 });
    assert.equal(opened.duration, .4);
    assert.equal(opened.revision, frame.revision);
    assert.equal(await opacity(), .625);
    assert.ok((await request(retired, { action: 'seek', time: .1 })).isError);
    assert.equal((await control({ action: 'seek', time: .4 })).time, .4);
    assert.equal(await opacity(), .25);
    await control({ action: 'seek', time: .1 });
    assert.equal(await opacity(), .8125);
    content(await request(frame.revision, { action: 'stop' }));
    frame = await wait();
    assert.equal(await opacity(), 1);
    assert.equal(document.getText(), authored);
    for (const expected of [scene, original]) {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        frame = await wait();
        assert.equal(document.getText(), expected);
    }
    console.log('VS Code and MCP previewed a directional state transition with a live resource, sought backwards, rejected a stale frame and restored the base state');
}

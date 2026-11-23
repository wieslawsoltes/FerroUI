import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { AnimationGraphMode } from '../src/animationGraph';
import type { McpFixture } from './mcp';

interface GraphDisplay {
    mode: AnimationGraphMode;
    revision: number;
    available_modes: AnimationGraphMode[];
    generation?: number;
}

export async function verifyTimelineGraphMcp({ invoke, content }: McpFixture, session: number,
    document: vscode.TextDocument, generation?: number): Promise<void> {
    const prefix = generation === undefined ? 'designer_timeline' : 'designer_hosted_timeline';
    const read = async () => content<GraphDisplay>(await invoke(`${prefix}_view`, { session }));
    const first = await read();
    const version = document.version;
    const source = document.getText();
    assert.deepEqual(first.available_modes, ['keys', 'value', 'velocity']);
    assert.equal(first.generation, generation);
    const select = (graph_mode: AnimationGraphMode, expected_revision = first.revision, expected_generation = generation) =>
        invoke(`${prefix}_set_graph`, { session, expected_revision, graph_mode,
            ...(expected_generation === undefined ? {} : { expected_generation }) });
    assert.match((await select('value', first.revision + 1)).content[0]?.text ?? '', /current .*revision/);
    if (generation !== undefined) {
        assert.match((await select('velocity', first.revision, generation + 1)).content[0]?.text ?? '', /current .*generation/);
    }
    assert.equal((await read()).mode, first.mode);
    for (const mode of ['velocity', 'value', 'keys', first.mode] as const) {
        const applied = content<GraphDisplay>(await select(mode));
        assert.equal(applied.mode, mode);
        assert.equal(applied.revision, first.revision);
        assert.equal((await read()).mode, mode);
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
}

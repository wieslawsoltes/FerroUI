import { strict as assert } from 'node:assert';
import { symlink, unlink, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';

interface BufferState { uri: string; revision: string; source: string; dirty: boolean }

export async function verifyWorkspaceMcp({ invoke, content }: McpFixture): Promise<void> {
    const cargo = 'McpProject/Cargo.toml';
    const main = 'McpProject/src/main.rs';
    const original = 'fn main() { println!("🧭"); }\n';
    const manifest = '[package]\nname = "mcp-project"\nversion = "0.1.0"\nedition = "2021"\n';
    const first = content<BufferState>(await invoke('ferroui_workspace_create', { path: cargo, source: manifest }));
    const second = content<BufferState>(await invoke('ferroui_workspace_create', { path: main, source: original, open: true }));
    assert.ok((await invoke('ferroui_workspace_create', { path: main, source: 'overwrite' })).isError);
    const { files } = content<{ files: { path: string }[] }>(await invoke('ferroui_workspace_files', { glob: 'McpProject/**/*' }));
    assert.ok(files.some(file => file.path.replaceAll('\\', '/') === main));
    const start = original.indexOf('🧭');
    const range = { start: { line: 0, character: start }, end: { line: 0, character: start + 2 } };
    const read = async (file: string) => content<BufferState>(await invoke('ferroui_workspace_read', { path: file }));
    const selected = content<BufferState>(await invoke('ferroui_workspace_read', { path: main, range }));
    assert.equal(selected.source, '🧭');
    const filesToEdit = [{ path: main, expected_revision: second.revision, edits: [{ range, newText: 'FerroUI' }] },
        { path: cargo, expected_revision: first.revision, source: manifest + '\n[features]\nhot-reload = []\n' }];
    assert.ok((await invoke('ferroui_workspace_edit', { files: [filesToEdit[0], { ...filesToEdit[1], expected_revision: 'stale' }] })).isError);
    assert.equal((await read(main)).source, original, 'A stale buffer does not modify the other file');
    content(await invoke('ferroui_workspace_edit', { files: filesToEdit }));
    const updated = await read(main);
    assert.equal(updated.source, original.replace('🧭', 'FerroUI'));
    assert.equal(updated.dirty, true);
    assert.ok((await invoke('ferroui_workspace_file', { path: main, action: 'save', expected_revision: second.revision })).isError);
    for (const file of [main, cargo]) {
        const buffer = await read(file);
        const saved = content<BufferState>(await invoke('ferroui_workspace_file', { path: file, action: 'save', expected_revision: buffer.revision }));
        assert.equal(saved.dirty, false);
    }
    const { targets } = content<{ targets: { binary: string; package: string; features: string[] }[] }>(
        await invoke('ferroui_reload_targets', { manifest_path: cargo }));
    assert.equal(targets.length, 1);
    assert.equal(targets[0].package, 'mcp-project');
    assert.deepEqual(targets[0].features, ['hot-reload']);
    assert.ok((await invoke('ferroui_reload_control', { action: 'start', manifest_path: cargo, binary: 'missing' })).isError);

    const draft = content<BufferState>(await invoke('ferroui_workspace_create', { path: 'Delete me.txt', source: 'temporary' }));
    const renamed = content<BufferState>(await invoke('ferroui_workspace_file', { path: 'Delete me.txt', action: 'rename',
        expected_revision: draft.revision, new_path: 'Renamed.txt' }));
    assert.ok(renamed.uri.endsWith('/Renamed.txt'));
    assert.ok((await read('Renamed.txt')).source === 'temporary');
    content(await invoke('ferroui_workspace_file', { path: 'Renamed.txt', action: 'delete', expected_revision: renamed.revision }));
    assert.ok((await invoke('ferroui_workspace_read', { path: 'Renamed.txt' })).isError);

    if (process.platform !== 'win32') {
        const root = vscode.workspace.workspaceFolders![0].uri.fsPath;
        const sibling = path.dirname(root);
        await writeFile(path.join(sibling, 'outside.txt'), 'outside the workspace');
        const alias = path.join(root, 'external');
        await symlink(sibling, alias);
        try {
            assert.ok((await invoke('ferroui_workspace_read', { path: 'external/outside.txt' })).isError);
            assert.ok((await invoke('ferroui_workspace_create', { path: 'external/new.txt', source: 'outside' })).isError);
        } finally {
            await unlink(alias);
        }
    }
    console.log('VS Code MCP workspace creation, UTF-16 ranges, atomic edits, save, rename, delete and Cargo discovery passed');
}

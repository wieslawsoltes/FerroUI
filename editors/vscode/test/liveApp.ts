import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import * as vscode from 'vscode';
import { revision } from '../src/reloadProcess';
import { readRuntimeDiagnostics } from '../src/runtimeDiagnostics';
import type { RuntimeNode, RuntimeSource } from '../src/runtimeProtocol';
import { sourcePosition } from '../src/runtimeSource';
import { withNativeApp } from './nativeAppFixture';

export async function verifyLiveApp(): Promise<void> {
    await withNativeApp(['--demo'], async client => {
        assert.ok(await client.supports('get_element_at'));
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'NewTask' })).matched);
        const [window] = await client.call<RuntimeNode[]>('list_windows');
        const [entry] = await client.call<RuntimeNode[]>('find_elements', { name: 'NewTask' });
        const [add] = await client.call<RuntimeNode[]>('find_elements', { name: 'AddTask' });
        const png = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.ok(entry && add && png.length);
        const source = await client.call<RuntimeSource>('get_source', { node_id: entry.node_id });
        assert.equal(source?.language, 'rust');
        assert.equal(source.columnEncoding, 'unicodeScalar');
        assert.ok(source.invocation && source.invocation.line < source.line);
        const rust = await readFile(new URL(source.uri), 'utf8');
        assert.equal(source.revision, revision(Buffer.from(rust)));
        const line = rust.split(/\r?\n/)[source.line - 1];
        assert.match(Array.from(line).slice(source.column - 1).join(''), /^TextBox\s*\{\s*name: "NewTask"/);
        const unicode = await vscode.workspace.openTextDocument({ language: 'rust', content: 'rsx! { "👋" TextBox {} }\r\n' });
        const position = sourcePosition(unicode, { ...source, line: 1, column: 12 });
        assert.equal(position?.character, unicode.lineAt(0).text.indexOf('TextBox'));
        assert.equal(sourcePosition(unicode, { ...source, line: 1, column: 999 }), undefined);
        await client.call('type_text', { node_id: entry.node_id, text: 'QA task from VS Code' });
        await client.call('click', { node_id: add.node_id });
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { text: 'QA task from VS Code' })).matched);
        const updated = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.notEqual(updated, png);
        const diagnostics = await readRuntimeDiagnostics(client);
        const stats = diagnostics.frames.find(frame => frame.node_id === window.node_id);
        assert.ok(stats && stats.layout_pass_count > 0 && stats.layout_timing_available, JSON.stringify(diagnostics.frames));
        assert.ok(Array.isArray(diagnostics.logs.entries));
        console.log('Live App found the authored RSX control, entered text, clicked a bound action and captured the updated native window');
    });
}

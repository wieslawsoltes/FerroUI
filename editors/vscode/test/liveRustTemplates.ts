import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { revision } from '../src/reloadProcess';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { readRuntimeTemplates } from '../src/runtimeTemplates';
import { withNativeApp } from './nativeAppFixture';

export async function verifyLiveRustTemplates(): Promise<void> {
    await withNativeApp(['--tree-templates-live'], async client => {
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'Accounts' })).matched);
        const [grid] = await client.call<RuntimeNode[]>('find_elements', { name: 'Accounts' });
        const templates = (await readRuntimeTemplates(client, grid.node_id)).filter(entry => entry.column);
        assert.equal(templates.length, 4);
        for (const template of templates) {
            assert.ok(template.source?.language === 'rust');
            const uri = vscode.Uri.parse(template.source.uri);
            assert.ok(uri.path.endsWith('/app_preview/tree_templates.rs'));
            const bytes = await vscode.workspace.fs.readFile(uri);
            assert.equal(template.source.revision, revision(Buffer.from(bytes)));
            assert.equal(template.column?.expander_inner, template.column?.header === 'Company');
        }
        const companyEditor = templates.find(entry =>
            entry.member === 'CellEditingTemplate' && entry.column?.header === 'Company');
        assert.ok(companyEditor?.source);
        const file = await vscode.workspace.openTextDocument(vscode.Uri.parse(companyEditor.source.uri));
        assert.match(file.lineAt(companyEditor.source.line - 1).text, /cell_editing_template/);
        assert.equal(new Set(templates.map(template => template.id)).size, 4);
        console.log('Live App verified the native TreeDataGrid templates and the compiled Rust editing constructor');
    });
}

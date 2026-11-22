import { strict as assert } from 'node:assert';
import { readFile, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import * as vscode from 'vscode';
import type { HostedFrame } from '../src/hostedProtocol';
import type { HostedSourceLocation } from '../src/hostedSources';
import type { HostedTemplateCatalog } from '../src/hostedTemplates';
import type { NativePreview } from '../src/nativePreview';

export async function verifyHostedRustTemplates(preview: NativePreview): Promise<void> {
    const extension = vscode.extensions.getExtension('FerroUI.ferroui')!;
    const root = await realpath(join(extension.extensionPath, '../../samples/TodoWorkbench'));
    const manifest = join(root, 'Cargo.toml');
    const viewPath = join(root, 'app_preview/template_review.rs');
    const rowsPath = join(root, 'app_preview/template_rows.rs');
    const uri = pathToFileURL(viewPath).toString();
    const rowsUri = pathToFileURL(rowsPath).toString();
    const [source, rows] = await Promise.all([readFile(viewPath, 'utf8'), readFile(rowsPath, 'utf8')]);
    const caret = { line: source.split('\n').findIndex(line => line.includes('StackPanel { width:')), character: 10 };
    const project = { workspaceRoot: root, metadata: { manifest, directories: [], package: 'todo-workbench' } };
    const render = async (revision: number, sourceText = source, moduleDraft?: string) => {
        const document = await preview.inProject({ uri, source: sourceText, language: 'rust', caret }, project);
        const result = await preview.renderHostedWorkspace({
            revision, document, assembly: 'todo-workbench', projectPath: '/app_preview/template_review.rs', fixture: 'Review queue',
        }, {
            root,
            rustProject: { manifest, targets: [join(root, 'main.rs')], features: ['app-preview', 'hot-reload', 'mcp'] },
            localSources: moduleDraft === undefined ? [] : [{ uri: rowsUri, source: moduleDraft }],
        });
        assert.equal(result.error, null);
        assert.ok(result.frame);
        assert.equal(result.frame.fixture, 'Review queue');
        assert.ok(result.dependencies?.includes(rowsUri), 'Follow the selected artifact’s app-preview module');
        return result.frame;
    };
    const catalog = (frame: HostedFrame) => preview.hostedTemplates<HostedTemplateCatalog>(frame, { action: 'catalog' });
    const first = await render(3200);
    assert.equal(first.templatesAvailable, true);
    const entries = (await catalog(first)).templates;
    const queue = entries.find(entry => entry.label === 'Queue' && entry.member === 'ItemTemplate');
    assert.ok(queue?.assignment, 'The realized list identifies the shared Rust template definition');
    assert.equal(queue.definition.uri, rowsUri);
    assert.equal(queue.assignment.uri, uri);
    assert.equal(queue.canCopy, false);
    const locate = (frame: HostedFrame, target: (typeof queue)['definition'], text: string) =>
        preview.hostedStyles<HostedSourceLocation>(frame, { action: 'locate', address: target.address, range: target.range, source: text });
    const definition = await locate(first, queue.definition, rows);
    assert.equal(lineText(rows, definition), '::ferroui_view::rsx');
    assert.equal(lineText(source, await locate(first, queue.assignment, source)), 'queue_row()');
    await assert.rejects(locate(first, queue.definition, rows + '\n// unsaved'), /source changed/);
    const priority = entries.filter(entry => entry.label === 'Priority');
    assert.deepEqual(priority.map(entry => entry.member).sort(), ['ItemTemplate', 'SelectionBoxItemTemplate']);
    assert.equal(priority[0].owner, priority[1].owner);
    const cell = entries.find(entry => entry.member === 'CellTemplate');
    assert.ok(cell?.assignment);
    assert.equal(cell.typeName, 'TableViewColumn');
    assert.equal(lineText(source, await locate(first, cell.definition, source)), 'rsx');
    assert.equal(cell.definition.uri, uri);

    const staleModule = await render(3201, source, rows.replace('#EEF3FB', '#DCEAFF'));
    const remaining = (await catalog(staleModule)).templates;
    assert.ok(!remaining.some(entry => entry.label === 'Queue'), 'A dirty shared module cannot borrow the compiled template stamp');
    assert.ok(remaining.some(entry => entry.member === 'CellTemplate'));
    await assert.rejects(catalog(first), /current app-hosted preview frame/);
    const staleView = await render(3202, source.replace('width: 520.0', 'width: 522.0'));
    assert.deepEqual((await catalog(staleView)).templates, [], 'The app still uses its compiled Rust fixture');
    assert.equal((await catalog(await render(3203))).templates.length, entries.length);
    console.log('App Preview verified shared and inline Rust item and cell template sources, and rejected stale compiled sources and frames');
}

function lineText(source: string, location: HostedSourceLocation): string {
    const { start, end } = location.range;
    assert.equal(start.line, end.line);
    return source.split('\n')[start.line].slice(start.character, end.character);
}

import { strict as assert } from 'node:assert';
import { readFile, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import * as vscode from 'vscode';
import type { HostedFrame } from '../src/hostedProtocol';
import type { HostedSourceLocation } from '../src/hostedSources';
import type { HostedTemplateCatalog } from '../src/hostedTemplates';
import type { NativePreview } from '../src/nativePreview';

export async function verifyHostedTreeTemplates(preview: NativePreview): Promise<void> {
    const extension = vscode.extensions.getExtension('FerroUI.ferroui')!;
    const root = await realpath(join(extension.extensionPath, '../../samples/TodoWorkbench'));
    const manifest = join(root, 'Cargo.toml');
    const project = { workspaceRoot: root, metadata: { manifest, directories: [], package: 'todo-workbench' } };
    const rustProject = { manifest, targets: [join(root, 'main.rs')], features: ['app-preview', 'hot-reload', 'mcp'] };
    const render = async (file: string, language: 'rust' | 'xaml', revision: number) => {
        const path = join(root, file);
        const uri = pathToFileURL(path).toString();
        const source = await readFile(path, 'utf8');
        const line = language === 'rust' ? source.split('\n').findIndex(text => text.includes('StackPanel { width:')) : 0;
        const document = await preview.inProject({ uri, source, language, caret: { line, character: 10 } }, project);
        const result = await preview.renderHostedWorkspace({
            revision, document, assembly: 'todo-workbench', projectPath: `/${file}`, fixture: 'Account hierarchy',
        }, { root, rustProject, localSources: [] });
        assert.equal(result.error, null);
        assert.ok(result.frame?.templatesAvailable);
        assert.equal(result.frame.fixture, 'Account hierarchy');
        return { source, uri, frame: result.frame };
    };
    type Entry = HostedTemplateCatalog['templates'][number];
    const gridSlots = async (frame: HostedFrame) => {
        const { templates } = await preview.hostedTemplates<HostedTemplateCatalog>(frame, { action: 'catalog' });
        const slots = templates.filter(entry => entry.typeName === 'TreeDataGridTemplateColumn');
        assert.equal(slots.length, 4, JSON.stringify(templates));
        return slots;
    };
    const pair = (slots: Entry[], label: string) => {
        const display = slots.find(entry => entry.label === label && entry.member === 'CellTemplate');
        const editing = slots.find(entry => entry.label === label && entry.member === 'CellEditingTemplate');
        assert.ok(display && editing, JSON.stringify(slots));
        assert.equal(display.owner, editing.owner);
        assert.equal(display.canCopy, false);
        assert.equal(editing.canCopy, false);
        return { display, editing };
    };
    const locate = (frame: HostedFrame, target: Entry['definition'], source: string) =>
        preview.hostedStyles<HostedSourceLocation>(frame, { action: 'locate', address: target.address, range: target.range, source });

    const rust = await render('app_preview/tree_templates.rs', 'rust', 3300);
    const rustSlots = await gridSlots(rust.frame);
    for (const label of ['Accounts / Company', 'Accounts / Review']) {
        const { display, editing } = pair(rustSlots, label);
        for (const slot of [display, editing]) {
            assert.equal(slot.definition.uri, rust.uri);
            const definition = await locate(rust.frame, slot.definition, rust.source);
            const { start, end } = definition.range;
            assert.equal(rust.source.split('\n')[start.line].slice(start.character, end.character), 'rsx');
            assert.ok(slot.assignment);
            assert.equal(slot.assignment.uri, rust.uri);
            const assignment = await locate(rust.frame, slot.assignment, rust.source);
            const body = rust.source.split('\n').slice(assignment.range.start.line, assignment.range.end.line + 1).join('\n');
            assert.match(body, slot.member === 'CellTemplate' ? /TextBlock/ : /TextBox/);
        }
    }
    const xaml = await render('HostedTreeTemplates.xaml', 'xaml', 3301);
    const xamlSlots = await gridSlots(xaml.frame);
    const resources = join(root, 'TreeTemplateResources.xaml');
    const dictionary = await readFile(resources, 'utf8');
    const company = pair(xamlSlots, 'Accounts / Company');
    for (const [slot, key] of [[company.display, 'CompanyDisplay'], [company.editing, 'CompanyEditor']] as const) {
        assert.equal(slot.definition.uri, pathToFileURL(resources).toString());
        const source = await locate(xaml.frame, slot.definition, dictionary);
        assert.match(dictionary.split('\n')[source.range.start.line], new RegExp(`x:Key="${key}"`));
    }
    await assert.rejects(locate(xaml.frame, company.editing.definition, dictionary + '\n<!-- draft -->'), /XAML buffer changed/);
    const review = pair(xamlSlots, 'Accounts / Review');
    for (const slot of [review.display, review.editing]) {
        assert.equal(slot.definition.uri, xaml.uri);
        const { range } = await locate(xaml.frame, slot.definition, xaml.source);
        assert.match(xaml.source.split('\n')[range.start.line - 1], new RegExp(slot.member!));
    }
    await assert.rejects(gridSlots(rust.frame), /current app-hosted preview frame/);
    console.log('App Preview opened native TreeDataGrid display and editing templates in inline Rust, XAML, and an included dictionary');
}

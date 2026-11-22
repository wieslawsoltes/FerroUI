import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { runEditorHistory } from '../src/editorHistory';
import type { HostedFrame } from '../src/hostedProtocol';
import { HostedSources } from '../src/hostedSources';
import { HostedTemplates } from '../src/hostedTemplates';
import type { NativePreview } from '../src/nativePreview';

export async function verifyHostedTemplateEditor(preview: NativePreview, frame: HostedFrame, viewUri: vscode.Uri,
    themeUri: string, view: string, theme: string): Promise<string> {
    assert.equal(frame.templatesAvailable, true);
    const buffer = await vscode.workspace.openTextDocument(viewUri);
    const editor = await vscode.window.showTextDocument(buffer, { preview: false });
    const signal = AbortSignal.timeout(15_000);
    const sources = new HostedSources(editor, async (_, signal, run) => {
        const version = buffer.version;
        return run(preview, () => {
            signal.throwIfAborted();
            assert.equal(buffer.version, version);
        });
    });
    const templates = new HostedTemplates(sources);
    const catalog = await templates.catalog(frame, signal);
    const selected = catalog.templates.find(template => template.label === 'ReviewAction');
    assert.ok(selected?.canCopy, 'Match the effective native template for ReviewAction');
    const { address, uri, range } = selected.definition;
    assert.equal(uri, themeUri);
    assert.equal(address, 'ferres://todo-workbench/Exported/Dictionaries/Outer.xaml');
    assert.equal(theme.split('\n')[range.start.line].slice(range.start.character, range.end.character), 'ControlTemplate');
    const row = catalog.templates.find(template => template.label === 'ReviewQueue' && template.member === 'ItemTemplate');
    assert.ok(row, 'The native items control retains the included data template identity');
    assert.equal(row.definition.uri, themeUri);
    assert.equal(row.canCopy, false);
    await templates.locate(frame, row.owner, signal, { member: row.member });
    const rowEditor = vscode.window.activeTextEditor;
    assert.equal(rowEditor?.document.getText(rowEditor.selection), 'DataTemplate');
    assert.deepEqual(await templates.locate(frame, selected.owner, signal), { uri, range });
    const sourceEditor = vscode.window.activeTextEditor;
    assert.ok(sourceEditor);
    assert.equal(sourceEditor.document.getText(sourceEditor.selection), 'ControlTemplate');
    const source = sourceEditor.document;

    const edit = new vscode.WorkspaceEdit();
    edit.insert(source.uri, source.positionAt(source.getText().length), '\n<!-- Changed include -->');
    assert.ok(await vscode.workspace.applyEdit(edit));
    await assert.rejects(templates.copy(frame, selected.owner, false, signal), /included XAML buffer changed/);
    await runEditorHistory(source, 'undo', () => signal.throwIfAborted(), signal);
    assert.equal(source.getText(), theme);

    const receipt = await templates.copy(frame, selected.owner, true, signal);
    assert.ok(receipt.applied);
    const result = buffer.getText();
    assert.match(result, /<Button\.Template>\s*<(?:\w+:)?ControlTemplate/);
    assert.ok(result.includes('Theme="{StaticResource HostedActionTheme}"'));
    await runEditorHistory(buffer, 'undo', () => signal.throwIfAborted(), signal);
    assert.equal(buffer.getText(), view);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.input instanceof vscode.TabInputText && [source.uri.toString(), buffer.uri.toString()].includes(tab.input.uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    return result;
}

import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { runEditorHistory } from '../src/editorHistory';
import { NativePreview } from '../src/nativePreview';
import { applyTextEdits, sourceOf } from '../src/textDocument';
import { verifyMcp } from './mcp';
import { verifyMultiSelection, verifyStudioPresentations, verifyWorkbenchFeatures } from './workbenchFeatures';
import { verifyCargoTarget } from './cargoTarget';
import { verifyDrag, verifyRsxImageDrag } from './drag';
import { verifyColumnsDrag } from './studioColumnsDrag';
import { verifyXamlRefactoring } from './xamlLanguage';
import { verifyRuntimeClient } from './runtimeClient';
import { verifyHostedPreview } from './hostedPreview';
import { verifyLiveApp } from './liveApp';
import { verifyLiveXaml } from './liveXaml';
import { verifyProcessShutdown, verifyReloadHistory } from './reloadProcess';
import { verifyTimelineViewport } from './timelineViewport';
import { until } from './wait';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { RuntimeSourceEditor } from '../src/runtimeSource';

const editorPosition = (line = 0, character = 0) => new vscode.Position(line, character);
const viewport = { width: 640, height: 420, dark: false };

export async function run(): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const extension = vscode.extensions.getExtension<{ designer: DesignerWorkbench; runtimeSource: RuntimeSourceEditor }>('FerroUI.ferroui')!;
    assert.ok(extension, 'FerroUI extension is installed in the Extension Development Host');
    const { designer, runtimeSource } = await extension.activate();
    verifyTimelineViewport();
    await verifyProcessShutdown();
    verifyReloadHistory();
    await verifyRuntimeClient();
    await vscode.commands.executeCommand('ferroui.openLiveApp');
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === 'FerroUI Live App')));
    const live = vscode.window.tabGroups.all.flatMap(group => group.tabs).find(tab => tab.label === 'FerroUI Live App');
    if (live) await vscode.window.tabGroups.close(live);
    const xaml = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, 'SettingsView.xaml'));
    assert.equal(xaml.languageId, 'ferroui-xaml');
    await vscode.window.showTextDocument(xaml);
    await vscode.commands.executeCommand('ferroui.openDesigner');
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === 'Design · SettingsView.xaml')));
    assert.ok(vscode.window.visibleTextEditors.some(visible => visible.document === xaml));
    await until(() => designer.designs().some(panel => panel.document === xaml && panel.canvasReady));

    await until(async () => {
        const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>('vscode.executeDocumentSymbolProvider', xaml.uri);
        return symbols?.some(symbol => symbol.name.includes('UserControl')) ?? false;
    });
    const line = xaml.getText().split('\n').findIndex(text => text.includes('Content="Save changes"'));
    const column = xaml.lineAt(line).text.indexOf('Content');
    await until(async () => {
        const hovers = await vscode.commands.executeCommand<vscode.Hover[]>('vscode.executeHoverProvider', xaml.uri, editorPosition(line, column + 2));
        return Boolean(hovers?.length);
    });
    const completionLine = xaml.getText().split('\n').findIndex(text => text.includes('<Button'));
    const completionColumn = xaml.lineAt(completionLine).text.indexOf('<Button') + 3;
    await until(async () => {
        const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
            'vscode.executeCompletionItemProvider', xaml.uri, editorPosition(completionLine, completionColumn),
        );
        return Boolean(completions?.items.some(item => item.label === 'Button'
            || (typeof item.label !== 'string' && item.label.label === 'Button')));
    });
    const invalid = new vscode.WorkspaceEdit();
    invalid.insert(xaml.uri, editorPosition(), '<broken');
    assert.ok(await vscode.workspace.applyEdit(invalid));
    await until(() => vscode.languages.getDiagnostics(xaml.uri).some(diagnostic => diagnostic.severity === vscode.DiagnosticSeverity.Error));
    const repair = new vscode.WorkspaceEdit();
    repair.delete(xaml.uri, new vscode.Range(editorPosition(), editorPosition(0, '<broken'.length)));
    assert.ok(await vscode.workspace.applyEdit(repair));
    await until(() => vscode.languages.getDiagnostics(xaml.uri).length === 0);
    console.log('VS Code XAML language association, symbols, hover, completion and live diagnostics passed');
    await verifyXamlRefactoring(workspace);
    const panel = designer.designs().find(panel => panel.document === xaml);
    assert.ok(panel);
    await verifyDrag(panel);
    await verifyColumnsDrag(designer, workspace);

    const undoXaml = async (expected: string) => {
        const version = xaml.version;
        await runEditorHistory(xaml, 'undo', () => assert.equal(xaml.version, version), AbortSignal.timeout(5000));
        await until(() => xaml.version !== version);
        assert.equal(xaml.getText(), expected);
    };

    const channel = vscode.window.createOutputChannel('FerroUI test');
    const preview = await NativePreview.start(extension, channel);
    try {
        assert.ok(preview.assets.length >= 20 && preview.assets.every(asset => asset.rust));
        const xamlInput = sourceOf(xaml, 'xaml', editorPosition());
        const frame = await preview.render(xamlInput, viewport);
        assert.ok(frame.image, frame.renderError ?? 'Expected a PNG from the headless renderer');
        assert.equal(Buffer.from(frame.image.png, 'base64').subarray(1, 4).toString(), 'PNG');
        const button = frame.nodes.find(node => node.name === 'saveButton');
        assert.ok(button?.bounds?.width, 'Native button bounds map to the outlined source element');
        const version = xaml.version;
        const { edits } = await preview.edit(xamlInput, { type: 'setProperty', node: button.id, name: 'Content', value: 'Ship it 🧪' });
        await applyTextEdits(xaml, version, edits);
        assert.ok(xaml.getText().includes('Content="Ship it 🧪"'));
        assert.ok(xaml.isDirty, 'The designer edits the unsaved buffer');
        await assert.rejects(applyTextEdits(xaml, version, edits), /source changed/i);
        await undoXaml(xamlInput.source);
        const parent = frame.nodes.find(node => node.label === 'StackPanel');
        assert.ok(parent?.canInsert);
        const placement = { origin: { type: 'asset', control: 'TextBlock' }, target: { type: 'outline', node: parent.id, position: 'inside' } } as const;
        await assert.rejects(preview.placement(1, placement), /rendered layout changed/i);
        const { edits: insertion, selection } = await preview.edit(xamlInput, { type: 'place', frame: 0, placement });
        await applyTextEdits(xaml, xaml.version, insertion);
        assert.ok(xaml.getText().includes('Text="Text"'));
        assert.ok(selection && xaml.lineAt(selection.start.line).text.includes('<TextBlock'));
        await undoXaml(xamlInput.source);

        const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, 'views.rs'));
        assert.equal(rust.languageId, 'rust');
        await vscode.commands.executeCommand('ferroui.openDesigner', rust.uri);
        const rsxDesigner = designer.designs().find(panel => panel.document === rust);
        assert.ok(rsxDesigner);
        await verifyRsxImageDrag(rsxDesigner);

        // The MCP audit opens other editors and views. Complete the source-only undo checks first.
        await vscode.commands.executeCommand('ferroui.openDesigner', xaml.uri);
        await verifyMcp(xaml);
        const draft = frame.counterpart.source?.replace('Save changes', 'Apply settings');
        assert.ok(frame.counterpart.editable && draft);
        const { edits: rsxEdits } = await preview.edit(xamlInput, { type: 'applyCounterpart', source: draft });
        await applyTextEdits(xaml, xaml.version, rsxEdits);
        assert.ok(xaml.getText().includes(`x:Name="saveButton" Content="Apply settings"`));

        const rustInput = sourceOf(rust, 'rust', editorPosition());
        const rsxField = rust.positionAt(rust.getText().indexOf('padding: 36.0') + 3);
        const rsxCompletion = await vscode.commands.executeCommand<vscode.CompletionList>('vscode.executeCompletionItemProvider', rust.uri, rsxField);
        assert.ok(rsxCompletion?.items.some(item => item.label === 'padding' && item.detail?.includes('Border.Padding')));
        const outsideRsx = await vscode.commands.executeCommand<vscode.CompletionList>('vscode.executeCompletionItemProvider', rust.uri, editorPosition());
        assert.ok(!outsideRsx?.items.some(item => item.detail?.includes('Border.Padding')));
        const rustFrame = await preview.render(rustInput, viewport);
        assert.equal(rustFrame.blocks.length, 2);
        assert.ok(rustFrame.image, rustFrame.renderError ?? 'Expected a rendered RSX tree');
        assert.ok(rustFrame.counterpart.editable && rustFrame.counterpart.source);
        const xamlDraft = rustFrame.counterpart.source.replace('Workspace settings', 'Project settings');
        const { edits: rustEdits } = await preview.edit(rustInput, { type: 'applyCounterpart', source: xamlDraft });
        await applyTextEdits(rust, rust.version, rustEdits);
        assert.equal(rust.getText(), rustInput.source.replace('Workspace settings', 'Project settings'));
        const rustPanel = rustFrame.nodes.find(node => node.label === 'StackPanel');
        assert.ok(rustPanel?.canInsert);
        const latestRust = sourceOf(rust, 'rust', editorPosition());
        await preview.render(latestRust, viewport, 2);
        const { edits: rustInsert, selection: insertedRust } = await preview.edit(latestRust, { type: 'place', frame: 2, placement: {
            origin: { type: 'asset', control: 'TextBlock' }, target: { type: 'outline', node: rustPanel.id, position: 'inside' },
        } });
        await applyTextEdits(rust, rust.version, rustInsert);
        assert.ok(rust.getText().includes('text: "Text"'));
        assert.ok(insertedRust && rust.lineAt(insertedRust.start.line).text.includes('TextBlock'));
        assert.ok(rust.getText().endsWith(latestRust.source.slice(latestRust.source.indexOf('pub fn counter'))));
        const partial = await preview.render(sourceOf(rust, 'rust', rust.positionAt(rust.getText().lastIndexOf('"Increment"'))), viewport);
        assert.ok(partial.image && !partial.counterpart.editable);
        assert.ok(partial.notices.some(notice => notice.includes('omitted Rust items')));
        await verifyWorkbenchFeatures(preview, xamlInput.uri, rustInput.uri);
        await verifyMultiSelection(preview, xamlInput.uri, rustInput.uri);
        await verifyStudioPresentations(preview, xamlInput.uri);
        await verifyHostedPreview(preview, xamlInput.uri);
        await verifyLiveApp();
        await verifyLiveXaml(workspace, runtimeSource);
        const commands = await vscode.commands.getCommands(true);
        assert.ok(commands.includes('ferroui.outline.focus'), 'Outline is a contributed VS Code view');
        assert.ok(commands.includes('ferroui.properties.focus'), 'Properties is a separate contributed VS Code view');
        assert.ok(commands.includes('ferroui.toolbox.focus'), 'Toolbox is a separate contributed VS Code view');
        for (const panel of ['events', 'resources', 'assets']) assert.ok(commands.includes(`ferroui.${panel}.focus`), `${panel} is a contributed VS Code view`);
        await vscode.commands.executeCommand('ferroui.showOutline');
        await vscode.commands.executeCommand('ferroui.showProperties');
        await vscode.commands.executeCommand('ferroui.showToolbox');
        await vscode.commands.executeCommand('ferroui.showStudio');
        await verifyCargoTarget(workspace, designer);
        console.log('VS Code docked views, native XAML and RSX rendering, both projection directions, guarded edits and undo passed');
    } finally {
        preview.dispose();
        channel.dispose();
    }
}

import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import { until } from './wait';

export async function verifyXamlRefactoring(workspace: vscode.Uri): Promise<void> {
    const uri = vscode.Uri.joinPath(workspace, 'Namescope.xaml');
    const source = '<StackPanel xmlns="https://github.com/ferroui">\n'
        + '<Button Name="Action" Content="Action"/>\n'
        + '<TextBlock Text="{Binding #Action.Width}"/>\n'
        + '</StackPanel>\n';
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    const declaration = document.positionAt(source.indexOf('Action') + 1);
    const reference = document.positionAt(source.indexOf('#Action') + 2);

    const definitions = await vscode.commands.executeCommand<(vscode.Location | vscode.LocationLink)[]>(
        'vscode.executeDefinitionProvider', uri, reference);
    assert.ok(definitions.some(definition => {
        const range = 'range' in definition ? definition.range : definition.targetSelectionRange;
        const target = 'uri' in definition ? definition.uri : definition.targetUri;
        return target.toString() === uri.toString() && range?.contains(declaration);
    }));
    const references = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeReferenceProvider', uri, declaration);
    assert.ok(references.some(location => location.uri.toString() === uri.toString() && location.range.contains(reference)));
    const wordEnd = document.positionAt(source.indexOf('#Action.Width') + '#Action.Width'.length);
    const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>('vscode.executeInlayHintProvider', uri, new vscode.Range(0, 0, 3, 0));
    assert.ok(hints.some(hint => hint.position.isEqual(wordEnd) && hint.kind === vscode.InlayHintKind.Type));
    const rename = await vscode.commands.executeCommand<vscode.WorkspaceEdit>('vscode.executeDocumentRenameProvider', uri, declaration, 'Submit');
    assert.deepEqual(rename.get(uri).map(edit => document.getText(edit.range)), ['Action', 'Action']);
    assert.ok(await vscode.workspace.applyEdit(rename));
    assert.equal(document.getText(), source.replace('Name="Action"', 'Name="Submit"').replace('#Action', '#Submit'));

    const formatting = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        'vscode.executeFormatDocumentProvider', uri, { tabSize: 2, insertSpaces: true });
    assert.ok(formatting.length);
    const formatted = new vscode.WorkspaceEdit();
    formatted.set(uri, formatting);
    assert.ok(await vscode.workspace.applyEdit(formatted));
    assert.equal(document.lineAt(1).text, '  <Button Name="Submit" Content="Action"/>');
    const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>('vscode.provideDocumentSemanticTokens', uri);
    assert.ok(tokens.data.length > 0);
    await verifyResourceEdits(workspace);
    console.log('VS Code XAML definitions, references, type hints, namescope rename, formatting and semantic tokens passed');
}

async function verifyResourceEdits(workspace: vscode.Uri): Promise<void> {
    const dictionary = vscode.Uri.joinPath(workspace, 'SafeResources.xaml');
    const uri = vscode.Uri.joinPath(workspace, 'ResourceConsumers.xaml');
    const source = '<Grid xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">\r\n'
        + '<Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries>'
        + '<ResourceInclude Source="SafeResources.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources>\r\n'
        + '<Border><Border.Resources><Color x:Key="Shadow"/></Border.Resources><!-- 😀 -->'
        + '<Button Background="{StaticResource Primary}" Tag="{DynamicResource \'shade\'}"/></Border>\r\n'
        + '<TextBox Name="Input"/><TextBlock Text="{Binding #Input.teXT}"/></Grid>\r\n';
    await vscode.workspace.fs.writeFile(dictionary, Buffer.from('<ResourceDictionary xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">'
        + '<Color x:Key="Primary"/><Color x:Key="Shade"/></ResourceDictionary>'));
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    const target = document.positionAt(source.indexOf('Primary'));
    const indexedDictionary = vscode.Uri.file(await realpath(dictionary.fsPath));
    await until(async () => {
        const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeDefinitionProvider', uri, target);
        return definitions.some(definition => definition.uri.toString() === indexedDictionary.toString());
    });
    for (const [offset, original, replacement] of [
        [source.indexOf("'shade'") + 1, 'shade', 'Shade'],
        [source.indexOf('teXT'), 'teXT', 'Text'],
    ] as const) {
        const problem = document.positionAt(offset);
        const range = new vscode.Range(problem, problem.translate(0, original.length));
        const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
            'vscode.executeCodeActionProvider', uri, range, vscode.CodeActionKind.QuickFix.value);
        const fix = actions.find(action => action.title === `Use ${replacement}`);
        assert.ok(fix?.edit, `The LSP suggests ${replacement} for ${original}`);
        assert.equal(fix.edit.get(uri)[0].newText, replacement);
    }
    await assert.rejects(async () => vscode.commands.executeCommand(
        'vscode.executeDocumentRenameProvider', uri, target, 'Shadow'), /would change the resolution of Primary/);
    const checked = await vscode.commands.executeCommand<vscode.WorkspaceEdit>(
        'vscode.executeDocumentRenameProvider', uri, target, 'Available');
    assert.equal(checked.size, 2, 'Rename includes the unopened dictionary and the active view');
    assert.ok(await vscode.workspace.applyEdit(checked));
    assert.equal(document.getText(), source.replace('Primary', 'Available'));
    assert.ok((await vscode.workspace.openTextDocument(indexedDictionary)).getText().includes('x:Key="Available"'));
    console.log('VS Code XAML provided typed and scoped quick fixes and guarded cross-file resource rename');
}

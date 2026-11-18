import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';

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
    console.log('VS Code XAML definitions, references, namescope rename, formatting and semantic tokens passed');
}

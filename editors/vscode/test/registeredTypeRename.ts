import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RegisteredRename } from '../src/registeredRename';
import { until } from './wait';

export async function verifyRegisteredTypeRename(xaml: vscode.TextDocument, rustUri: vscode.Uri, registryUri: vscode.Uri) {
    const extension = vscode.extensions.getExtension<{ typeRename: RegisteredRename }>('FerroUI.ferroui')!;
    const { typeRename } = await extension.activate();
    const xamlConsumer = vscode.Uri.joinPath(xaml.uri, '..', 'RegisteredTypeConsumer.xaml');
    const styled = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(xaml.uri, '..', 'RegisteredPropertyConsumer.xaml'));
    const originalStyle = styled.getText();
    const registry = await vscode.workspace.openTextDocument(registryUri);
    const originalRegistry = registry.getText();
    const codeConsumer = vscode.Uri.joinPath(rustUri, '..', 'registered_type_consumer.rs');
    const markup = '<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:ui="using:Audit.Controls" Content="{x:Type ui:AuditCard}"/>';
    const code = 'use crate::AuditCard;\nfn inspect(_: &AuditCard) {}\n';
    await vscode.workspace.fs.writeFile(xamlConsumer, Buffer.from(markup));
    await vscode.workspace.fs.writeFile(codeConsumer, Buffer.from(code));

    const originalXaml = xaml.getText();
    const rust = await vscode.workspace.openTextDocument(rustUri);
    const originalRust = rust.getText();
    const symbol = 'AuditCard';
    const replacement = 'OverviewCard';
    const provider = vscode.languages.registerRenameProvider({ language: 'rust', pattern: '**/Sample App/src/main.rs' }, {
        provideRenameEdits(document, position, newName) {
            assert.equal(document.uri.toString(), rustUri.toString());
            assert.equal(document.offsetAt(position), originalRust.indexOf('struct AuditCard') + 'struct '.length);
            assert.equal(newName, replacement);
            const edit = new vscode.WorkspaceEdit();
            for (const occurrence of originalRust.matchAll(/\bAuditCard\b/g)) {
                edit.replace(rustUri, new vscode.Range(rust.positionAt(occurrence.index), rust.positionAt(occurrence.index + symbol.length)), newName);
            }
            const lines = code.split('\n');
            lines.forEach((line, row) => {
                const column = line.indexOf(symbol);
                if (column >= 0) edit.replace(codeConsumer, new vscode.Range(row, column, row, column + symbol.length), newName);
            });
            return edit;
        },
    });
    try {
        const caret = xaml.positionAt(originalXaml.indexOf(symbol) + 2);
        const position = { line: caret.line, character: caret.character };
        await vscode.window.showTextDocument(xaml, { viewColumn: vscode.ViewColumn.One });
        await typeRename.openReview(xaml, { version: xaml.version, position }, replacement);
        await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Rename AuditCard → OverviewCard') === true);
        await vscode.commands.executeCommand('ferroui.typeRenameReview.apply');
        await until(() => xaml.getText().includes(replacement));
        const editedXaml = await vscode.workspace.openTextDocument(xamlConsumer);
        const editedRust = await vscode.workspace.openTextDocument(codeConsumer);
        assert.equal(editedXaml.getText(), markup.replaceAll(symbol, replacement));
        assert.equal(editedRust.getText(), code.replaceAll(symbol, replacement));
        assert.equal(rust.getText(), originalRust.replaceAll(symbol, replacement));
        assert.equal(styled.getText(), originalStyle.replaceAll(symbol, replacement));
        const renamedMapping = JSON.stringify({ path: 'crate::OverviewCard', properties: { Caption: 'heading' } });
        assert.equal(registry.getText(), originalRegistry
            .replace('Audit.Controls.AuditCard', 'Audit.Controls.OverviewCard')
            .replace(', MAPPING)', `, r#"${renamedMapping}"#)`));
        await vscode.window.showTextDocument(xaml);
        await vscode.commands.executeCommand('undo');
        await until(() => xaml.getText() === originalXaml && editedXaml.getText() === markup
            && editedRust.getText() === code && rust.getText() === originalRust && styled.getText() === originalStyle
            && registry.getText() === originalRegistry);
        console.log('VS Code reviewed an RSX mapping key and builder path with the Rust rename and undid all six files');
    } finally { provider.dispose(); }
}

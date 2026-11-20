import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RegisteredPropertyRename } from '../src/registeredPropertyRename';
import { until } from './wait';

export async function verifyRegisteredPropertyRename(xaml: vscode.TextDocument, rustUri: vscode.Uri) {
    const extension = vscode.extensions.getExtension<{ propertyRename: RegisteredPropertyRename }>('FerroUI.ferroui')!;
    const { propertyRename } = await extension.activate();
    const original = xaml.getText();
    const sibling = vscode.Uri.joinPath(xaml.uri, '..', 'RegisteredPropertyConsumer.xaml');
    const consumer = original.replace('Caption="Review"', 'Caption="🧭 More details"');
    await vscode.workspace.fs.writeFile(sibling, Buffer.from(consumer));
    const rust = await vscode.workspace.openTextDocument(rustUri);
    const rustOriginal = rust.getText();
    const rustDraft = new vscode.WorkspaceEdit();
    rustDraft.insert(rust.uri, new vscode.Position(0, 0), '// Unsaved source 🪴\n');
    assert.ok(await vscode.workspace.applyEdit(rustDraft));
    const draft = rust.getText();
    const caret = xaml.positionAt(original.indexOf('Caption') + 1);
    const position = { line: caret.line, character: caret.character };
    const request = {
        path: vscode.workspace.asRelativePath(xaml.uri, false),
        folder: vscode.workspace.getWorkspaceFolder(xaml.uri)?.uri.toString(),
        position, new_name: 'Heading',
    };
    const signal = AbortSignal.timeout(30_000);
    const plan = await propertyRename.agent(request, signal);
    assert.equal(plan.documents.length, 3);
    assert.deepEqual(plan.unmapped, []);
    assert.equal(plan.documents.find(document => document.edit.uri === rust.uri.toString())?.before, draft);
    const nextConsumer = consumer.replace('More details', 'Full details');
    await vscode.workspace.fs.writeFile(sibling, Buffer.from(nextConsumer));
    await assert.rejects(propertyRename.agent({ ...request, review: plan.review }, signal), /changed/);
    assert.equal(xaml.getText(), original);
    assert.equal(rust.getText(), draft);

    await vscode.window.showTextDocument(xaml, { viewColumn: vscode.ViewColumn.One });
    await propertyRename.openReview(xaml, { version: xaml.version, position }, request.new_name);
    await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Rename Caption → Heading') === true);
    await vscode.commands.executeCommand('ferroui.propertyRenameReview.apply');
    await until(() => xaml.getText().includes('Heading='));
    const opened = await vscode.workspace.openTextDocument(sibling);
    assert.equal(opened.getText(), nextConsumer.replace('Caption=', 'Heading='));
    assert.equal(rust.getText(), draft.replace('Caption: String', 'Heading: String'));
    await vscode.window.showTextDocument(xaml);
    await vscode.commands.executeCommand('undo');
    await until(() => xaml.getText() === original && opened.getText() === nextConsumer && rust.getText() === draft);
    await vscode.window.showTextDocument(rust);
    await vscode.commands.executeCommand('undo');
    await until(() => rust.getText() === rustOriginal);
    console.log('VS Code reviewed a registered property change, rejected a stale plan, and undid the grouped Rust and XAML edits');
}

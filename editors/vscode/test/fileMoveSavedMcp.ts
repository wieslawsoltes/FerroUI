import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifySavedFileMove({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Saved reference move');
    const file = (path: string) => vscode.Uri.joinPath(root, path);
    const beforePath = 'Customer & notes.xaml';
    const afterPath = 'Archive/Customer % review.xaml';
    const write = (path: string, source: string) => vscode.workspace.fs.writeFile(file(path), Buffer.from(source));
    const old = file(beforePath).toString();
    const board = {
        version: 3, active: 0, stages: [{
            name: 'Customer review', source_uri: old, target: { kind: 'document' }, layout: 'fixed',
            environment: { width: 480, height: 300, theme: 'light', flow_direction: 'inherit' },
            pinned: true, auto_update: true, mode: 'select', design_time_data: true, states: [],
        }], prototype: { flows: [{ name: 'Review', start: 0 }], links: [] },
    };
    const boardFile = 'Review.ferrostages';
    const scenarioFile = 'Review.ferroui-scenario.json';
    const scenario = {
        version: 1, name: 'Customer 🪴', document: `./${beforePath}`,
        prototype: { stageSet: boardFile, flow: 'Review', sources: { [old]: { path: beforePath } } },
        steps: [{ action: 'expectStage', name: 'Customer review' }],
    };
    const json = (data: object) => JSON.stringify(data, null, 2).replaceAll('\n', '\r\n') + '\r\n';
    await vscode.workspace.fs.createDirectory(file('Archive'));
    await Promise.all([
        write(beforePath, '<Border xmlns="https://github.com/ferroui"><TextBlock Text="Customer review"/></Border>'),
        write(boardFile, '\uFEFF' + json(board)), write(scenarioFile, json(scenario)),
    ]);
    const scenarioDoc = await vscode.workspace.openTextDocument(file(scenarioFile));
    const editor = await vscode.window.showTextDocument(scenarioDoc);
    const rename = new vscode.Range(scenarioDoc.positionAt(scenarioDoc.getText().indexOf('Customer 🪴')),
        scenarioDoc.positionAt(scenarioDoc.getText().indexOf('Customer 🪴') + 'Customer 🪴'.length));
    const relative = (uri: vscode.Uri) => vscode.workspace.asRelativePath(uri, false).replaceAll('\\', '/');
    const args = { path: relative(file(beforePath)), destination: relative(file(afterPath)) };
    const plan = async () => content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', args));
    const apply = (review: string) => invoke('ferroui_workspace_move_xaml', { ...args, review });
    const first = await plan();
    assert.equal(first.documents.length, 3);
    await editor.edit(edit => edit.replace(rename, 'Unsaved customer review 🪴'));
    assert.ok((await apply(first.review)).isError);
    const updated = await plan();
    const draft = scenarioDoc.getText();
    const boardEdit = updated.documents.find(entry => entry.edit.uri.endsWith(boardFile))!;
    const caseEdit = updated.documents.find(entry => entry.edit.uri.endsWith(scenarioFile))!;
    assert.equal(caseEdit.edit.version, scenarioDoc.version);
    assert.equal(caseEdit.before, draft);
    assert.equal(caseEdit.edit.edits.length, 3);
    content<FileMovePlan>(await apply(updated.review));
    const stageDoc = await vscode.workspace.openTextDocument(file(boardFile));
    assert.equal(JSON.parse(stageDoc.getText()).stages[0].source_uri, updated.to);
    const run = JSON.parse(scenarioDoc.getText());
    assert.equal(run.document, `./${afterPath}`);
    assert.deepEqual(run.prototype.sources, { [updated.to]: { path: afterPath } });
    assert.equal(run.name, 'Unsaved customer review 🪴');
    assert.ok(await stageDoc.save());
    assert.ok(await scenarioDoc.save());
    const result = content<{ passed: boolean }>(await invoke('ferroui_scenario_run', { path: relative(file(scenarioFile)) }));
    assert.equal(result.passed, true);
    const stat = (uri: vscode.Uri) => vscode.workspace.fs.stat(uri).then(() => true, () => false);
    // Saving a background file can release its editor model. Read its current
    // buffer after each history step, including when Undo reopens the model.
    const stageText = async () => (await vscode.workspace.openTextDocument(file(boardFile))).getText();
    await vscode.window.showTextDocument(scenarioDoc);
    await vscode.commands.executeCommand('undo');
    await until(async () => scenarioDoc.getText() === draft && await stat(file(beforePath)) && await stageText() === json(board));
    await vscode.commands.executeCommand('redo');
    await until(async () => scenarioDoc.getText() === caseEdit.after && await stat(file(afterPath))
        && await stageText() === boardEdit.after.replace(/^\uFEFF/, ''));
    console.log('Reviewed XAML move preserved a saved stage and runnable scenario, rejected a stale JSON draft, and grouped Undo and Redo');
}

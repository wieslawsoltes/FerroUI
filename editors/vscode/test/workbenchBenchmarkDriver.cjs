const { strict: assert } = require('node:assert');
const { writeFile } = require('node:fs/promises');
const vscode = require('vscode');

const warmups = 4;
const samplesPerRun = 9;

exports.run = async () => {
    const extension = vscode.extensions.getExtension('FerroUI.ferroui');
    assert.ok(extension, 'Load FerroUI from the extracted VSIX extension directory');
    const { designer } = await extension.activate();
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, 'SettingsView.xaml');
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document, { preview: false });
    await vscode.commands.executeCommand('ferroui.openDesigner', uri);
    const panel = designer.designs().find(design => design.document === document);
    assert.ok(panel, 'Open the real designer in the workbench');
    let snapshot = await acceptedAfter(panel, undefined);
    const viewport = { width: snapshot.image.width, height: snapshot.image.height };
    await panel.controlCanvas({ zoom: 'fit' }, AbortSignal.timeout(15_000));

    let current = 'Northwind';
    const original = document.getText();
    const start = original.indexOf(current);
    assert.ok(start >= 0);
    const range = new vscode.Range(document.positionAt(start), document.positionAt(start + current.length));
    const samples = [];
    for (let sample = 0; sample < warmups + samplesPerRun; sample++) {
        const next = current === 'Northwind' ? 'Southwind' : 'Northwind';
        const token = panel.currentToken;
        const edit = new vscode.WorkspaceEdit();
        edit.replace(uri, range, next);
        const began = performance.now();
        assert.ok(await vscode.workspace.applyEdit(edit));
        const accepted = await acceptedAfter(panel, token);
        assert.notEqual(accepted.image.png, snapshot.image.png, 'The changed text reached a new native image');
        // The webview answers the canvas action after it presents the pending decoded image.
        await panel.controlCanvas({ zoom: 'fit' }, AbortSignal.timeout(15_000));
        const elapsed = performance.now() - began;
        if (sample >= warmups) samples.push(elapsed);
        current = next;
        snapshot = accepted;
    }
    const revert = new vscode.WorkspaceEdit();
    revert.replace(uri, range, 'Northwind');
    assert.ok(await vscode.workspace.applyEdit(revert));
    assert.equal(document.getText(), original);
    assert.ok(await document.save());
    await writeFile(process.env.FERROUI_BENCHMARK_RESULT, JSON.stringify({
        vscode: vscode.version, extension: extension.packageJSON.version, viewport, warmups, samples,
    }));
};

function acceptedAfter(panel, token) {
    const signal = AbortSignal.timeout(15_000);
    return new Promise((resolve, reject) => {
        function finish(done) {
            change.dispose();
            signal.removeEventListener('abort', abort);
            done();
        }
        function abort() { finish(() => reject(new Error('Timed out waiting for the next accepted preview'))); }
        function read() {
            const snapshot = panel.snapshot;
            if (panel.currentToken !== token && snapshot) {
                finish(() => {
                    if (snapshot.image && !snapshot.renderError) resolve(snapshot);
                    else reject(new Error(snapshot.renderError ?? 'The preview returned no image'));
                });
            }
        }
        const change = panel.onDidChange(read);
        signal.addEventListener('abort', abort, { once: true });
        read();
    });
}

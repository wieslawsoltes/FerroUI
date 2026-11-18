import { strict as assert } from 'node:assert';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { selectCargoTarget } from '../src/cargoTarget';
import { buildCargo } from '../src/reloadProcess';

export async function verifyCargoTarget(workspace: vscode.Uri): Promise<void> {
    const project = vscode.Uri.joinPath(workspace, 'Apps', 'Sample App');
    const write = async (relative: string, content: string) => {
        const uri = vscode.Uri.joinPath(project, relative);
        await vscode.workspace.fs.createDirectory(vscode.Uri.file(path.dirname(uri.fsPath)));
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
        return uri;
    };
    await write('Cargo.toml', '[package]\nname = "audit-app"\nversion = "0.1.0"\nedition = "2024"\n[features]\nhot-reload = []\n');
    await write('.cargo/config.toml', '[build]\ntarget-dir = "audit-output"\n');
    const source = await write('src/main.rs', 'fn main() {}\n');
    await vscode.window.showTextDocument(source);
    const target = await selectCargoTarget();
    assert.ok(target);
    assert.equal(target.packageName, 'audit-app');
    assert.deepEqual(target.features, ['hot-reload']);
    assert.equal(target.cargoDirectory, project.fsPath);
    assert.equal(path.basename(target.targetDirectory), 'audit-output');
    const output = vscode.window.createOutputChannel('Cargo target test');
    try {
        const { artifact } = buildCargo(target, output);
        assert.equal(path.basename(path.dirname(path.dirname(await artifact))), 'audit-output');
    } finally {
        output.dispose();
    }
    console.log('VS Code discovered and built the nested Cargo project using its Cargo configuration');
}

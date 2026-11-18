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
    const metadata = JSON.stringify({
        format: 2, name: 'Audit Application', crate_name: 'audit_app',
        types: [{ name: 'AuditCard', namespace: 'Audit.Controls', kind: 'class', rust_path: 'audit_app::AuditCard',
            properties: [{ name: 'Caption', value_type: 'String', setter: {} }] }],
    });
    await write('build.rs', `fn main() {
        let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("audit.xamlmeta");
        std::fs::write(path, r#"${metadata}"#).unwrap();
    }\n`);
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
    const markup = '<local:AuditCard xmlns:local="using:Audit.Controls" Caption="Review"/>';
    const uri = await write('Card.xaml', markup);
    const editor = await vscode.window.showTextDocument(uri, { viewColumn: vscode.ViewColumn.One });
    const xaml = editor.document;
    assert.equal(xaml.languageId, 'ferroui-xaml');
    await vscode.commands.executeCommand('ferroui.buildProjectSchema');
    const caption = xaml.positionAt(markup.indexOf('Caption') + 1);
    let hovers: vscode.Hover[] | undefined;
    for (const deadline = Date.now() + 10_000; Date.now() < deadline;) {
        hovers = await vscode.commands.executeCommand<vscode.Hover[]>('vscode.executeHoverProvider', xaml.uri, caption);
        if (hovers?.some(hover => hover.contents.some(content =>
            (typeof content === 'string' ? content : content.value).includes('Caption: String')))) {
            console.log('VS Code used the nested Cargo configuration and published its generated XAML schema');
            return;
        }
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    const diagnostics = vscode.languages.getDiagnostics(xaml.uri).map(item => item.message);
    assert.fail(`The built project schema did not provide the type of AuditCard.Caption in ${xaml.uri}. ` +
        `Hovers: ${JSON.stringify(hovers)}. Diagnostics: ${diagnostics.join(', ')}`);
}

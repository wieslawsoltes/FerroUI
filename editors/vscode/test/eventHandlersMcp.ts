import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { HandlerPlan, HandlerTarget } from '../src/eventHandlerProtocol';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyEventHandlers({ invoke, content }: McpFixture): Promise<void> {
    const folder = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Event audit');
    const rustUri = vscode.Uri.joinPath(folder, 'src', 'settings.rs');
    const xamlUri = vscode.Uri.joinPath(folder, 'Settings.xaml');
    const xamlSource = '<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Audit.Settings">\r\n'
        + '  <StackPanel>\r\n    <!-- 🦀 --> <Button x:Name="Apply" Content="Apply settings" />\r\n'
        + '    <Button x:Name="Reset" Click="OnReset" Content="Reset settings" />\r\n  </StackPanel>\r\n</UserControl>\r\n';
    const rustSource = '// 🦀 Registration and comments stay in place.\nstruct Settings;\nferro_class!(Settings: UserControl);\n'
        + 'ferro_class_info!(Settings { markup: { namespace: "Audit" } });\n';
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(folder, 'src'));
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(folder, '.cargo'));
    const registry = {
        format: 2, name: 'Audit', crate_name: 'event_audit',
        xmlns_definitions: [['https://github.com/ferroui', 'FerroUI.Controls']],
        types: [
            { name: 'Settings', namespace: 'Audit', explicit_namespace: 'Audit', kind: 'class',
                object_model: true, class_markup: true, module: 'event_audit::settings', rust_path: '::event_audit::settings::Settings' },
            { name: 'Interactive', namespace: 'FerroUI.Interactivity', kind: 'class', rust_path: '::ferroui_base::interactivity::Interactive' },
            { name: 'Button', namespace: 'FerroUI.Controls', kind: 'class', rust_path: '::ferroui_controls::Button',
                base: '::ferroui_base::interactivity::Interactive', fields: [{ name: 'ClickEvent', is_static: true,
                    return_type: '::ferroui_base::interactivity::RoutedEvent<::ferroui_base::interactivity::RoutedEventArgs>' }] },
        ],
    };
    const write = (file: string, source: string) => vscode.workspace.fs.writeFile(vscode.Uri.joinPath(folder, file), Buffer.from(source));
    await Promise.all([
        write('Cargo.toml', '[package]\nname = "event-audit"\nversion = "0.1.0"\nedition = "2024"\n'),
        write('.cargo/config.toml', '[build]\ntarget-dir = "cargo-output"\n'),
        write('src/main.rs', 'macro_rules! ferro_class { ($($tokens:tt)*) => {} }\n'
            + 'macro_rules! ferro_class_info { ($($tokens:tt)*) => {} }\nmod settings;\nfn main() {}\n'),
        write('src/settings.rs', rustSource),
        write('Settings.xaml', xamlSource),
        write('build.rs', `fn main() { let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("event_audit.xamlmeta");
            std::fs::write(path, r#"${JSON.stringify(registry)}"#).unwrap(); }`),
    ]);
    const doc = await vscode.workspace.openTextDocument(xamlUri);
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('ferroui.buildProjectSchema');
    const at = doc.positionAt(doc.getText().indexOf('OnReset') + 2);
    await until(async () => {
        const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>('vscode.executeCodeActionProvider', xamlUri,
            new vscode.Range(at, at), vscode.CodeActionKind.QuickFix.value);
        return !!actions?.some(action => action.command?.command === 'ferroui.createEventHandler');
    });
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: 'Event audit/Settings.xaml' }));
    const frame = async () => content<{ revision: number }>(await invoke('designer_wait', { session })).revision;
    const revision = await frame();
    const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    const apply = nodes.find(node => node.name === 'Apply');
    assert.ok(apply);
    const input = { session, expected_revision: revision, node: apply.id, event: 'Click' };
    const read = async (handler?: string) => content<HandlerTarget & { rustVersion: number; plan?: HandlerPlan }>(
        await invoke('designer_event_handler', { ...input, handler }));
    const target = await read();
    assert.equal(target.suggestedName, 'OnApplyClick');
    assert.match(target.parameterTypes[1], /IRoutedEventArgs/);
    const rust = await vscode.workspace.openTextDocument(rustUri);
    const change = new vscode.WorkspaceEdit();
    change.insert(rustUri, new vscode.Position(0, 0), '// An unsaved Rust edit\n');
    assert.ok(await vscode.workspace.applyEdit(change));
    const before = rust.getText();
    const original = doc.getText();
    const scaffold = (version: number, override = {}) => invoke('designer_scaffold_handler', {
        ...input, handler: 'OnApplyClick', expected_rust_version: version, ...override,
    });
    assert.match((await scaffold(target.rustVersion)).content[0]?.text ?? '', /Rust buffer changed/);
    assert.equal(doc.getText(), original);
    assert.equal(rust.getText(), before);
    const preview = await read('OnApplyClick');
    assert.equal(preview.plan?.edit.document_changes.length, 2);
    assert.equal(rust.getText(), before);
    assert.ok((await scaffold(preview.rustVersion, { expected_revision: revision + 100 })).isError);
    const plan = content<HandlerPlan>(await scaffold(preview.rustVersion));
    assert.match(doc.getText(), /Click="OnApplyClick"/);
    assert.match(rust.getText(), /fn OnApplyClick\(/);
    assert.ok(rust.getText().startsWith('// An unsaved Rust edit\n'));
    assert.ok(doc.isDirty && rust.isDirty);
    assert.equal(Buffer.from(await vscode.workspace.fs.readFile(xamlUri)).toString(), xamlSource);
    assert.equal(Buffer.from(await vscode.workspace.fs.readFile(rustUri)).toString(), rustSource);
    assert.equal(rust.getText(asRange(plan.declaration.range)), 'OnApplyClick');
    const definitionAt = doc.positionAt(doc.getText().indexOf('OnApplyClick') + 2);
    await until(async () => {
        const definitions = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', xamlUri, definitionAt);
        return definitions.some(location => location.uri.fsPath === rustUri.fsPath && location.range.isEqual(asRange(plan.declaration.range)));
    });
    const editedXaml = doc.getText();
    const editedRust = rust.getText();
    content(await invoke('designer_history', { session, expected_version: doc.version, action: 'undo' }));
    assert.equal(doc.getText(), original);
    assert.equal(rust.getText(), before);
    content(await invoke('designer_history', { session, expected_version: doc.version, action: 'redo' }));
    assert.equal(doc.getText(), editedXaml);
    assert.equal(rust.getText(), editedRust);
    content(await invoke('designer_history', { session, expected_version: doc.version, action: 'undo' }));
    assert.equal(doc.getText(), original);
    assert.equal(rust.getText(), before);
    console.log('VS Code offered the XAML handler Quick Fix, previewed and applied guarded paired edits through MCP, and revealed the unsaved Rust declaration');
}

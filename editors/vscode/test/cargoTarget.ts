import { strict as assert } from 'node:assert';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { describeCargoTarget, discoverCargoTargets, selectCargoTarget } from '../src/cargoTarget';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { buildCargo } from '../src/reloadProcess';
import { verifyRegisteredPropertyRename } from './registeredPropertyRename';
import { verifyRegisteredTypeRename } from './registeredTypeRename';
import { until } from './wait';

export async function verifyCargoTarget(workspace: vscode.Uri, designer: DesignerWorkbench): Promise<void> {
    const project = vscode.Uri.joinPath(workspace, 'Apps', 'Sample App');
    const write = async (relative: string, content: string) => {
        const uri = vscode.Uri.joinPath(project, relative);
        await vscode.workspace.fs.createDirectory(vscode.Uri.file(path.dirname(uri.fsPath)));
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
        return uri;
    };
    await write('Cargo.toml', '[package]\nname = "audit-app"\nversion = "0.1.0"\nedition = "2024"\n[features]\nhot-reload = []\n[dependencies]\naudit-widgets = { path = "widgets" }\n');
    await write('.cargo/config.toml', '[build]\ntarget-dir = "audit-output"\n');
    const mappingKey = 'FerroUI.Designer.Rsx.v1:Audit.Controls.AuditCard';
    const mapping = JSON.stringify({ path: 'crate::AuditCard', properties: { Caption: 'heading' } });
    const registry = await write('src/registry.rs', [
        'pub struct MarkupAssembly { pub metadata: &\'static [(&\'static str, &\'static str)] }',
        'pub static DESIGN: MarkupAssembly = MarkupAssembly {',
        '    metadata: &[',
        `        ("${mappingKey}", r#"${mapping}"#),`,
        '    ],',
        '};',
        '',
    ].join('\n'));
    const metadata = JSON.stringify({
        format: 2, name: 'Audit Application', crate_name: 'audit_app',
        metadata: [[mappingKey, mapping]],
        types: [{ name: 'AuditCard', namespace: 'Audit.Controls', kind: 'class', rust_path: '::audit_app::AuditCard',
            object_model: true, base: '::audit_widgets::Meter', default_constructor: {},
            properties: [{ name: 'Caption', value_type: 'String', setter: {} }] }],
    });
    const appSite = { rust_type: '::audit_app::AuditCard', type: 'AuditCard', file: 'src/main.rs', owner: 'AuditCard' };
    await write('build.rs', exportMetadata(metadata, {
        manifest: vscode.Uri.joinPath(project, 'Cargo.toml').fsPath, assembly: 'Audit Application', crate_name: 'audit_app',
        assembly_source: { file: 'src/registry.rs', item: 'DESIGN' },
        definitions: [
            { ...appSite, kind: 'class', type_declaration: { file: 'src/main.rs', name: 'AuditCard' } },
            { ...appSite, kind: 'class_info', member: 'Caption' },
        ],
    }));
    await write('widgets/Cargo.toml', '[package]\nname = "audit-widgets"\nversion = "0.1.0"\nedition = "2024"\n');
    const dependencySource = await write('widgets/src/lib.rs', 'macro_rules! ferro_class { ($($t:tt)*) => {} }\npub struct Meter;\nferro_class!(Meter: Control);\n');
    await write('widgets/build.rs', exportMetadata(JSON.stringify({
        format: 2, name: 'Audit Widgets', crate_name: 'audit_widgets', types: [{
            name: 'Meter', namespace: 'Audit.Widgets', kind: 'class', rust_path: '::audit_widgets::Meter',
            object_model: true, base: '::ferroui_controls::Control', default_constructor: {},
        }],
    }), {
        manifest: vscode.Uri.joinPath(project, 'widgets/Cargo.toml').fsPath, assembly: 'Audit Widgets', crate_name: 'audit_widgets',
        definitions: [{ rust_type: '::audit_widgets::Meter', type: 'Meter', file: 'src/lib.rs',
            kind: 'class', owner: 'Meter' }],
    }));
    const source = await write('src/main.rs', [
        'pub mod registry;',
        'macro_rules! ferro_class { ($($t:tt)*) => {} }',
        'macro_rules! ferro_class_info { ($($t:tt)*) => {} }',
        'pub struct AuditCard;',
        'ferro_class!(AuditCard: Meter);',
        'ferro_class_info!(AuditCard { markup: { properties: [Caption: String { get: caption }] } });',
        'fn main() {}',
        '',
    ].join('\n'));
    await vscode.window.showTextDocument(source);
    const target = await selectCargoTarget();
    assert.ok(target);
    assert.equal(target.packageName, 'audit-app');
    assert.deepEqual(target.features, ['hot-reload']);
    assert.equal(target.cargoDirectory, project.fsPath);
    assert.equal(path.basename(target.targetDirectory), 'audit-output');
    const selection = { ...describeCargoTarget(target), features: target.features };
    assert.equal((await discoverCargoTargets(selection))[0]?.packageId, target.packageId);
    assert.deepEqual(await discoverCargoTargets({ ...selection, features: [] }), []);
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
    let ready = false;
    for (const deadline = Date.now() + 10_000; Date.now() < deadline;) {
        hovers = await vscode.commands.executeCommand<vscode.Hover[]>('vscode.executeHoverProvider', xaml.uri, caption);
        if (hovers?.some(hover => hover.contents.some(content =>
            (typeof content === 'string' ? content : content.value).includes('Caption: String')))) {
            ready = true;
            break;
        }
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    const diagnostics = vscode.languages.getDiagnostics(xaml.uri).map(item => item.message);
    assert.ok(ready, `The built project schema did not provide the type of AuditCard.Caption in ${xaml.uri}. ` +
        `Hovers: ${JSON.stringify(hovers)}. Diagnostics: ${diagnostics.join(', ')}`);
    await verifyDefinitions(xaml, source, dependencySource);
    await verifyRegisteredPropertyRename(xaml, source, registry);
    await verifyRegisteredTypeRename(xaml, source, registry);

    const view = await write('View.xaml', '<StackPanel xmlns="https://github.com/ferroui" />');
    const panel = await designer.openDocument(view);
    const snapshot = await panel.waitForSnapshot(AbortSignal.timeout(15_000));
    const registered = snapshot.projectAssets ?? [];
    assert.ok(registered.some(asset => asset.title === 'AuditCard' && asset.category === 'Project controls'));
    const dependency = registered.find(asset => asset.title === 'Meter');
    assert.equal(dependency?.category, 'Dependency controls');
    const token = panel.currentToken;
    assert.ok(token !== undefined && dependency && snapshot.nodes[0].canInsert);
    await panel.place({ session: panel.session, token }, {
        origin: { type: 'asset', control: dependency.id },
        target: { type: 'outline', node: snapshot.nodes[0].id, position: 'inside' },
    });
    assert.match(panel.document.getText(), /<ui:Meter xmlns:ui="clr-namespace:Audit.Widgets;assembly=Audit Widgets"\s*\/>/);

    const rustDraft = 'fn view() { let _ = rsx! { crate::AuditCard { heading: "🦀 First" } }; }\n';
    const rust = await write('src/view.rs', rustDraft);
    const rustPanel = await designer.openDocument(rust);
    const rustSnapshot = await rustPanel.waitForSnapshot(AbortSignal.timeout(15_000));
    assert.ok(rustSnapshot.counterpart.source?.includes('Caption="🦀 First"'));
    assert.ok(!rustSnapshot.image, 'registered source metadata alone does not load custom controls into the preview host');
    const position = rustPanel.document.positionAt(rustDraft.indexOf('heading') + 1);
    const completion = await vscode.commands.executeCommand<vscode.CompletionList>('vscode.executeCompletionItemProvider', rust, position);
    assert.ok(completion?.items.some(item => item.label === 'heading' && item.detail?.includes('AuditCard.Caption')));
    const current = rustPanel.currentToken;
    const selected = rustPanel.selectedNode;
    assert.ok(current !== undefined && selected);
    await rustPanel.editNode({ session: rustPanel.session, token: current, node: selected.id }, {
        type: 'setProperty', node: selected.id, name: 'Caption', value: 'Updated',
    });
    assert.equal(rustPanel.document.getText(), rustDraft.replace('🦀 First', 'Updated'));
    console.log('VS Code used the nested Cargo registry for XAML, dependency Toolbox placement, and mapped RSX completion and literal editing');
}

async function verifyDefinitions(xaml: vscode.TextDocument, source: vscode.Uri, dependency: vscode.Uri) {
    const rust = await vscode.workspace.openTextDocument(source);
    const at = xaml.positionAt(xaml.getText().indexOf('Caption') + 1);
    const lookup = async (document: vscode.TextDocument, position: vscode.Position) =>
        (await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', document.uri, position))?.[0];
    const saved = await lookup(xaml, at);
    assert.equal(saved?.uri.toString(), source.toString());
    assert.equal(rust.getText(saved?.range), 'Caption');
    const draft = new vscode.WorkspaceEdit();
    draft.insert(source, new vscode.Position(0, 0), '// Unsaved navigation draft 🪴\n\n');
    assert.ok(await vscode.workspace.applyEdit(draft));
    const live = await lookup(xaml, at);
    assert.equal(live?.uri.toString(), source.toString());
    assert.equal(live?.range.start.line, saved!.range.start.line + 2);
    assert.equal(rust.getText(live?.range), 'Caption');
    await vscode.window.showTextDocument(rust);
    await vscode.commands.executeCommand('undo');
    await until(() => !rust.isDirty);

    const markup = '<d:Meter xmlns:d="using:Audit.Widgets"/>';
    const uri = vscode.Uri.joinPath(xaml.uri, '..', 'Meter.xaml');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(markup));
    const view = await vscode.workspace.openTextDocument(uri);
    const location = await lookup(view, view.positionAt(markup.indexOf('Meter') + 1));
    assert.equal(location?.uri.toString(), dependency.toString());
    assert.equal((await vscode.workspace.openTextDocument(dependency)).getText(location?.range), 'Meter');
    console.log('VS Code resolved live Rust declarations and dependency controls from the Cargo source index');
}

function exportMetadata(metadata: string, sourceIndex: object): string {
    return `fn main() {
        let directory = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(directory.join("audit.xamlmeta"), r#"${metadata}"#).unwrap();
        std::fs::write(directory.join("xaml-rust-sources.json"), r#"${JSON.stringify({version: 1, ...sourceIndex})}"#).unwrap();
    }\n`;
}

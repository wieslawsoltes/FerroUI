import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CreatedProject, ProjectTemplateCatalog, ProjectTemplatePlan, ProjectToolchainReport } from '../src/projectSetupProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface Inventory { files: { path: string; kind: string; invocations?: number }[]; total: number; truncated: boolean }

export async function verifyProjectMcp({ invoke, content }: McpFixture): Promise<void> {
    const list = async (kind?: string) => content<Inventory>(await invoke('ferroui_project_files', kind ? { kind } : {}));
    const initial = await list();
    assert.ok(initial.files.some(file => file.path === 'SettingsView.xaml' && file.kind === 'xaml'));
    assert.ok(initial.files.some(file => file.path === 'views.rs' && file.kind === 'rsx' && file.invocations === 2));
    assert.ok(initial.files.some(file => file.path === 'McpProject/Cargo.toml' && file.kind === 'cargo'));
    assert.ok(initial.files.some(file => file.path === 'Assets/Preview Image.png' && file.kind === 'asset'));
    assert.ok(initial.files.some(file => file.path === 'customer-search.ferroui-scenario.json' && file.kind === 'scenario'));

    const request = { kind: 'component', language: 'rsx', path: 'McpProject/src/revenue_card' };
    const { uri } = content<{ uri: string }>(await invoke('ferroui_project_scaffold', request));
    const [rustPath, xamlPath] = ['McpProject/src/revenue_card.rs', 'Views/OrdersView.xaml'];
    const rust = await vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
    assert.match(rust.getText(), /#\[component\(rerender\)\]\s+pub\(crate\) fn RevenueCard\(\) -> Element/);
    assert.ok((await invoke('ferroui_project_scaffold', request)).isError);
    content(await invoke('ferroui_project_scaffold', { kind: 'view', language: 'xaml', path: xamlPath }));
    content(await invoke('ferroui_workspace_create', { path: 'not_a_view.rs', source: 'const SAMPLE: &str = "rsx! { Button {} }";\n' }));
    content(await invoke('ferroui_workspace_create', { path: 'Review.ferrostages', source: '{"stages":[]}\n' }));

    await until(async () => (await list('rsx')).files.some(file => file.path === rustPath));
    assert.ok(!(await list('rsx')).files.some(file => file.path === 'not_a_view.rs'));
    await until(async () => (await list('xaml')).files.some(file => file.path === xamlPath));
    await until(async () => (await list('stages')).files.some(file => file.path === 'Review.ferrostages'));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: rustPath }));
    const image = await invoke('designer_preview_image', { session });
    assert.equal(image.isError, false, image.content[0]?.text);
    await verifyTemplates({ invoke, content });
    console.log('The Project model found Cargo manifests, native RSX invocations, XAML, assets and saved stage sets, and rendered an MCP-created component');
}

async function verifyTemplates({ invoke, content }: McpFixture): Promise<void> {
    const catalog = content<ProjectTemplateCatalog>(await invoke('ferroui_project_templates', {}));
    assert.deepEqual(catalog.templates.map(item => item.id), ['desktopRsx', 'desktopXaml']);
    const folder = vscode.workspace.workspaceFolders![0].uri;
    for (const { id: template, entry } of catalog.templates) {
        const name = template === 'desktopRsx' ? 'starter-rsx' : 'starter-xaml';
        const target = vscode.Uri.joinPath(folder, name);
        const args = { template, name, folder: folder.toString() };
        const plan = content<ProjectTemplatePlan>(await invoke('ferroui_project_plan_new', args));
        assert.equal(plan.entry, entry);
        assert.ok(plan.files.some(file => file.path === 'Cargo.toml' && file.source.includes('rust-version')));
        await assert.rejects(async () => vscode.workspace.fs.stat(target));
        assert.ok((await invoke('ferroui_project_create', { ...args, review: '0000000000000000' })).isError);
        const result = content<CreatedProject>(await invoke('ferroui_project_create', { ...args, review: plan.review }));
        assert.equal(vscode.Uri.file(result.entry).path, vscode.Uri.joinPath(vscode.Uri.file(plan.root), ...entry.split('/')).path);
        for (const file of plan.files) {
            const actual = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(target, ...file.path.split('/')));
            assert.equal(new TextDecoder().decode(actual), file.source);
        }
        const { session } = content<{ session: number }>(await invoke('designer_open', { path: name + '/' + entry }));
        const preview = await invoke('designer_preview_image', { session });
        assert.equal(preview.isError, false, preview.content[0]?.text);
        assert.ok((await invoke('ferroui_project_create', { ...args, review: plan.review })).isError);
    }
    assert.ok((await invoke('ferroui_project_plan_new', { folder: folder.toString(), template: 'desktopRsx',
        name: 'bad-checkout', checkout: '..' })).isError);
    const health = content<ProjectToolchainReport>(await invoke('ferroui_project_check_toolchain', { folder: folder.toString() }));
    assert.equal(health.minimumRust, catalog.rustVersion);
    assert.ok(health.cargo.version ?? health.cargo.error);
    if (health.rustc.version) assert.ok(health.host);
}

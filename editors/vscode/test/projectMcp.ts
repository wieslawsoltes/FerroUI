import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
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
    console.log('The Project model found Cargo manifests, native RSX invocations, XAML, assets and saved stage sets, and rendered an MCP-created component');
}

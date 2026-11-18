import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TokenPlan } from '../src/designTokenProtocol';
import type { PropertyField } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyDesignTokens({ invoke, content }: McpFixture): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder, 'customer-design.tokens.json'));
    const source = document.getText();
    const plan = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'import', source, options: {
        mappings: { 'brand.card': { key: 'CustomerCardBrush' } }, pixelsPerRem: 16,
    } }));
    assert.equal(plan.mappings.length, 5);
    assert.equal(plan.issues.length, 1);
    assert.equal(plan.issues[0].path, 'controls.title');
    assert.equal(plan.mappings.find(row => row.key === 'CustomerCardBrush')?.value, '#2D6A51');
    assert.equal(plan.mappings.find(row => row.key === 'layout.padding')?.value, '24');
    assert.equal(document.getText(), source);
    const dictionary = 'CustomerTokens.xaml';
    const create = { path: dictionary, source: plan.source };
    const created = content<{ revision: string }>(await invoke('ferroui_workspace_create', create));
    assert.ok((await invoke('ferroui_workspace_create', create)).isError);
    const view = 'CustomerTokenView.xaml';
    content(await invoke('ferroui_workspace_create', { path: view, source: `<Border xmlns="https://github.com/ferroui"
        Name="Card" Background="{StaticResource CustomerCardBrush}" Width="{StaticResource layout.cardWidth}">
        <Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source="${dictionary}"/>
        </ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources>
        <TextBlock Text="Customer review"/>
    </Border>` }));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: view }));
    const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(await invoke('designer_outline', { session }));
    const card = nodes.find(node => node.name === 'Card');
    assert.ok(card);
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(properties.find(field => field.name === 'Background')?.effective, '#FF2D6A51');
    assert.equal(properties.find(field => field.name === 'Width')?.effective, '320');
    const draft = plan.source.replace('x:Key="CustomerCardBrush" Color="#2D6A51"', 'x:Key="CustomerCardBrush" Color="#1B2C47"');
    assert.notEqual(draft, plan.source);
    content(await invoke('ferroui_workspace_edit', { files: [{ path: dictionary, expected_revision: created.revision, source: draft }] }));
    const updated = content<{ revision: number }>(await invoke('designer_wait', { session }));
    assert.ok(updated.revision > revision);
    const live = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(live.properties.find(field => field.name === 'Background')?.effective, '#FF1B2C47');
    const saved = vscode.Uri.joinPath(folder, dictionary);
    assert.equal(Buffer.from(await vscode.workspace.fs.readFile(saved)).toString(), plan.source);
    const authored = content<{ revision: string }>(await invoke('ferroui_workspace_read', { path: dictionary }));
    content(await invoke('ferroui_workspace_file', { path: dictionary, action: 'undo', expected_revision: authored.revision }));
    content(await invoke('designer_wait', { session }));
    const restored = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(restored.properties.find(field => field.name === 'Background')?.effective, '#FF2D6A51');
    const exported = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'export', source: plan.source, options: {
        mappings: { CustomerCardBrush: { token: 'brand.card' }, 'layout.cardWidth': { kind: 'dimension' } },
    } }));
    assert.equal(exported.issues.length, 0);
    const exportedJson = JSON.parse(exported.source);
    assert.equal(exportedJson.brand.card.$extensions['com.github.ferroui'].key, 'CustomerCardBrush');
    assert.deepEqual(exportedJson.layout.cardWidth.$value, { value: 320, unit: 'px' });
    assert.deepEqual(exportedJson.motion.fade.$value, { value: 250, unit: 'ms' });
    const roundtrip = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'import', source: exported.source }));
    assert.equal(roundtrip.issues.length, 0);
    assert.equal(roundtrip.mappings.find(row => row.key === 'CustomerCardBrush')?.value, '#2D6A51');
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === view || tab.label === `Design · ${view}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code planned a DTCG handoff, rendered merged XAML resources with native values and exported the explicit reverse mapping');
}

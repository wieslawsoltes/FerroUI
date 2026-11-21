import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';

export async function verifyGridMcp({ invoke, content }: McpFixture): Promise<void> {
    for (const path of ['GridWorkbench.xaml', 'grid_workbench.rs']) {
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
        const original = document.getText();
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(
            await invoke('designer_outline', { session }));
        const target = nodes.find(node => node.name === 'Cell');
        assert.ok(target);
        const model = content<{ panel: number; axes: { axis: string; tracks: { length: string }[] }[] }>(
            await invoke('designer_studio_query', { session, tool: 'designer_grid_tracks', arguments: { node: target.id } }));
        assert.equal(model.panel, nodes.find(node => node.name === 'CustomerGrid')?.id);
        assert.deepEqual(model.axes.find(axis => axis.axis === 'rows')?.tracks.map(track => track.length), ['Auto', '*']);
        const edit = { session, expected_revision: revision, tool: 'designer_grid_track_edit',
            arguments: { panel: model.panel, action: 'insert', axis: 'rows', index: 0, length: '24' } };
        content(await invoke('designer_studio_edit', edit));
        if (path.endsWith('.xaml')) {
            assert.match(document.getText(), /<RowDefinition Height="24"/);
            assert.match(document.getText(), /<RowDefinition Height="Auto" MinHeight="10" \/>/);
            assert.match(document.getText(), /Grid.Row="2" Grid.Column="1"/);
        } else {
            assert.equal(document.getText(), original.replace('"Auto,*"', '"24,Auto,*"').replace('Grid.row: 1', 'Grid.row: 2'));
        }
        assert.ok((await invoke('designer_studio_edit', edit)).isError);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        const opened = vscode.window.tabGroups.all.flatMap(group => group.tabs)
            .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
        await vscode.window.tabGroups.close(opened);
    }
    console.log('VS Code reused Studio Grid track plans in XAML and RSX and kept changed cells in one undo step');
}

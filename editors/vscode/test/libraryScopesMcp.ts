import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { scopedLibraryItems, type LibraryScopes } from '../src/libraryScopes';
import type { OutlineNode } from '../src/protocol';
import { StudioContext } from '../src/studioContext';
import { readLibrary } from '../src/studioLibrary';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyLibraryScopes({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ScopedObjects.xaml';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const source = `<StackPanel Name="Orders" xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:i="using:FerroUI.Xaml.Interactivity" xmlns:b="using:FerroUI.Xaml.Interactions.Core">
    <StackPanel.Resources>
      <SolidColorBrush x:Key="Accent" Color="#405AB0"/>
      <DataTemplate x:Key="OrderRow">
        <Border Name="Row">
          <Border.Resources><SolidColorBrush x:Key="Accent" Color="#D34F45"/></Border.Resources>
          <Button Name="Pin" Content="Pin">
            <i:Interaction.Behaviors><b:EventTriggerBehavior EventName="Click"/></i:Interaction.Behaviors>
          </Button>
        </Border>
      </DataTemplate>
    </StackPanel.Resources>
    <Button Name="Approve" Content="Approve 🧭"><i:Interaction.Behaviors><b:EventTriggerBehavior EventName="Click"/></i:Interaction.Behaviors></Button>
</StackPanel>
`.replaceAll('\n', '\r\n');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    const tool = <T>(name: string, args: object) => invoke(name, args).then(content<T>);
    const { session } = await tool<{ session: number }>('designer_open', { path });
    const query = () => tool<LibraryScopes>('designer_studio_query', { session,
        tool: 'designer_library_scopes', arguments: {} });
    const frame = async () => {
        const ready = await tool<{ status: string; renderError?: string }>('designer_wait', { session });
        assert.equal(ready.status, 'Ready', ready.renderError);
        return tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
    };

    const initial = await frame();
    const report = await query();
    const approve = report.scopes.find(scope => scope.path === 'Orders › Approve');
    assert.ok(approve);
    assert.equal(approve.owner, initial.nodes.find(node => node.name === 'Approve')?.id);
    assert.equal(document.getText(asRange(approve.objects[0].range)), 'b:EventTriggerBehavior');
    const accent = report.scopes.flatMap(scope => scope.objects).filter(object => object.title === 'Accent');
    assert.equal(accent.length, 2);
    assert.notEqual(accent[0].definition, accent[1].definition);
    const definition = report.scopes.flatMap(scope => scope.objects).find(object => object.title === 'OrderRow')?.definition;
    assert.ok(definition);

    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    assert.ok(panel.supportsLibraryScopes);
    const context = new StudioContext(panel, initial.revision, () => !panel.isDisposed);
    const rows = await scopedLibraryItems(context, (await readLibrary(context)).components, async () => undefined);
    const group = rows.find(item => item.label === approve.path);
    const action = (await group?.children?.())?.find(item => item.label === 'Click')?.run;
    assert.ok(action);
    await action();
    const editor = vscode.window.activeTextEditor;
    assert.equal(editor?.document.uri.toString(), document.uri.toString());
    assert.ok(editor?.selection.isEqual(asRange(approve.objects[0].range)));

    await tool('designer_studio_present', { session, expected_revision: (await frame()).revision,
        tool: 'designer_definition', arguments: { action: 'enter', node: definition } });
    await frame();
    const isolated = await query();
    assert.ok(isolated.scopeEpoch > report.scopeEpoch);
    assert.deepEqual(isolated.scopes.map(scope => scope.path), ['Orders › OrderRow › Row', 'Orders › OrderRow › Row › Pin']);
    await assert.rejects(action, /source or selection changed/i);
    await tool('designer_studio_present', { session, expected_revision: (await frame()).revision,
        tool: 'designer_definition', arguments: { action: 'exit' } });
    await frame();
    assert.ok((await query()).scopes.some(scope => scope.path === 'Orders › Approve'));
    assert.equal(document.getText(), source);
    console.log('Library scopes kept duplicate XAML keys independent, revealed the exact authored behavior, fenced stale actions and followed definition isolation');
}

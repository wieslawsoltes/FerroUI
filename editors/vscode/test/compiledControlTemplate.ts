import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { runEditorHistory } from '../src/editorHistory';
import { until } from './wait';

const namespace = 'xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"';
export const compiledTemplatePath = 'Themes/Customer source.xaml';
export const compiledTemplateKey = 'Packaged/CustomerTheme.xaml';
export const compiledTemplateAddress = `ferres://audit_app/${compiledTemplateKey}`;
export const compiledTemplateSource = `<ResourceDictionary ${namespace}>
  <ControlTheme x:Key="CustomerButton" TargetType="Button">
    <Setter Property="Template">
      <ControlTemplate TargetType="Button">
        <Border Background="#FF256F96" Padding="12,4">
          <ContentPresenter Content="{TemplateBinding Content}" Foreground="White" />
        </Border>
      </ControlTemplate>
    </Setter>
  </ControlTheme>
</ResourceDictionary>`;

interface TemplateReport {
    model: string;
    slots: { canCopyInline: boolean; copyReason?: string; includedDefinition?: { uri: string } }[];
}

export async function verifyCompiledControlTemplate(designer: DesignerWorkbench, theme: vscode.Uri,
    write: (path: string, contents: string) => Promise<vscode.Uri>): Promise<void> {
    const original = `<StackPanel ${namespace}>
  <StackPanel.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <ResourceInclude Source="${compiledTemplateAddress}" />
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </StackPanel.Resources>
  <Button Name="CustomerAction" Theme="{StaticResource CustomerButton}" Content="Save customer" />
</StackPanel>`;
    const source = await write('CustomerView.xaml', original);
    const panel = await designer.openDocument(source);
    const frame = () => panel.waitForSnapshot(AbortSignal.timeout(15_000));
    const before = await frame();
    assert.ok(before.image, before.renderError ?? 'Render the file-backed template');
    const node = before.nodes.find(node => node.name === 'CustomerAction')?.id;
    const token = panel.currentToken;
    assert.ok(node && token !== undefined);
    const report = await panel.studioQuery('designer_item_templates', { node, property: 'Template' }, [node],
        AbortSignal.timeout(15_000)) as TemplateReport;
    const slot = report.slots[0];
    assert.ok(slot.includedDefinition, 'Resolve the native template to the authored compiler input');
    assert.equal(vscode.Uri.parse(slot.includedDefinition.uri).fsPath, await realpath(theme.fsPath));
    assert.equal(slot.canCopyInline, true, slot.copyReason);

    await panel.studioEdit(token, 'designer_item_template', {
        node, property: 'Template', action: 'copy_inline', expected_model: report.model,
    }, [node], AbortSignal.timeout(15_000));
    assert.match(panel.document.getText(), /<Button\.Template>\s*<(?:\w+:)?ControlTemplate/);
    assert.ok(panel.document.getText().includes(compiledTemplateAddress));
    assert.equal((await frame()).image?.png, before.image.png);
    const version = panel.document.version;
    await runEditorHistory(panel.document, 'undo', () => assert.equal(panel.document.version, version), AbortSignal.timeout(5_000));
    await until(() => panel.document.getText() === original);
    console.log('VS Code copied the inherited template from a file-backed compiled alias and preserved the native frame and Undo');
}

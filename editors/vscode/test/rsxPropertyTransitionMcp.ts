import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';
import type { CanvasSnapshot } from '../src/protocol';

export async function verifyRustPropertyTransitionPreview({ invoke, content }: McpFixture): Promise<void> {
    const name = 'RsxPropertyTransitionsWorkbench.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const doc = await vscode.workspace.openTextDocument(uri);
    const source = doc.getText();
    const card = doc.positionAt(source.indexOf('name: "AccountCard"'));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: name, line: card.line, character: card.character }));
    const ready = content<{ status: string }>(await invoke('designer_wait', { session }));
    assert.equal(ready.status, 'Ready');
    const rendered = content<{ source: string; counterpart: CanvasSnapshot['counterpart'] }>(await invoke('designer_document', {
        session, include_source: true, include_counterpart: true,
    }));
    assert.match(rendered.counterpart.source ?? '', /DoubleTransition Property="{x:Static Visual.OpacityProperty}"/);
    assert.match(rendered.counterpart.source ?? '', /ThicknessTransition Property="{x:Static Layoutable.MarginProperty}"[^>]*Easing="CubicEaseOut"/);
    assert.equal(rendered.counterpart.editable, false);
    assert.equal(rendered.source, source);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === name || tab.label === `Design · ${name}`);
    await vscode.window.tabGroups.close(tabs);
}

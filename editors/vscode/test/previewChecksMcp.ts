import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { PreviewFinding } from '../src/protocol';
import type { StageChecks, StageModel } from '../src/stagesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyPreviewChecks({ invoke, content }: McpFixture): Promise<void> {
    const path = 'PreviewChecks.xaml';
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const args = { session, tool: 'designer_preview_checks', arguments: {} };
    const read = async () => content<{ findings: PreviewFinding[] }>(
        await invoke('designer_studio_query', args));
    const { findings } = await read();
    const codes = findings.map(finding => [finding.name, finding.code]);
    for (const expected of [['UnlabeledInput', 'missingAccessibleName'], ['FadedNote', 'lowTextContrast'], ['CutOffAction', 'clippedLayout']]) {
        assert.ok(codes.some(([name, code]) => name === expected[0] && code === expected[1]), JSON.stringify(findings));
    }
    assert.ok(!findings.some(finding => ['LabeledInput', 'ReadableNote', 'ScrollableAction'].includes(finding.name ?? '')), JSON.stringify(findings));
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    content(await invoke('designer_source', { session, expected_version: document.version,
        source: original.replace('Name="UnlabeledInput"', 'Name="UnlabeledInput" AutomationProperties.Name="Primary contact"') }));
    assert.ok(!(await read()).findings.some(finding => finding.name === 'UnlabeledInput'));
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    const board = content<StageModel>(await invoke('designer_stages_open', { session }));
    const stage = board.stages[0];
    assert.ok(stage);
    const target = { session, stage: stage.id, expected_epoch: stage.epoch };
    const stageReport = content<StageChecks>(await invoke('designer_stage_checks', target));
    const clipped = stageReport.findings.find(finding => finding.code === 'clippedLayout' && finding.name === 'CutOffAction');
    assert.ok(clipped?.range);
    assert.equal(document.getText(asRange(clipped.range)), 'Button');
    content(await invoke('designer_stage_change', { session, expected_epoch: board.epoch, arguments: {
        action: 'update', id: stage.id, configuration: { ...stage.configuration,
            environment: { ...stage.configuration.environment, width: stage.configuration.environment.width + 24 } },
    } }));
    assert.ok((await invoke('designer_stage_checks', target)).isError);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    await verifyLayoutChecks({ invoke, content });
}

async function verifyLayoutChecks({ invoke, content }: McpFixture): Promise<void> {
    const open = async (path: string, line = 0) => {
        const folder = vscode.workspace.workspaceFolders![0].uri;
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder, path));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path, line, character: 10 }));
        const read = async () => content<{ findings: PreviewFinding[] }>(await invoke('designer_studio_query', {
            session, tool: 'designer_preview_checks', arguments: {},
        })).findings;
        return { document, session, read };
    };
    const text = (source: vscode.TextDocument, finding?: PreviewFinding) => {
        assert.ok(finding?.range, JSON.stringify(finding));
        return source.getText(asRange(finding.range));
    };
    const xaml = await open('LayoutDiagnostics.xaml');
    const findings = await xaml.read();
    const card = findings.find(finding => finding.name === 'OverhangingCard' && finding.code === 'layoutOverflow');
    assert.equal(text(xaml.document, card), 'Border');
    assert.deepEqual(card?.overflow, { boundary: 'the Canvas bounds', left: 0, top: 0, right: 40, bottom: 0 });
    const properties = findings.filter(finding => finding.code === 'ineffectiveLayoutValue');
    assert.equal(text(xaml.document, properties.find(finding => finding.name === 'CapacityCard' && finding.property === 'Canvas.Right')), 'Canvas.Right="32"');
    assert.match(properties.find(finding => finding.name === 'CapacityCard' && finding.property === 'Width')?.message ?? '', /210 px to 188 px/);
    assert.equal(text(xaml.document, properties.find(finding => finding.property === 'Grid.Row')), 'Grid.Row="7"');
    assert.equal(text(xaml.document, properties.find(finding => finding.property === 'Grid.ColumnSpan')), 'Grid.ColumnSpan="4"');
    const comparison = content<StageModel>(await invoke('designer_stages_open', { session: xaml.session }));
    const compact = comparison.stages.find(stage => stage.configuration.environment.width < 640);
    assert.ok(compact);
    const compactReport = content<StageChecks>(await invoke('designer_stage_checks', {
        session: xaml.session, stage: compact.id, expected_epoch: compact.epoch,
    }));
    assert.ok(compactReport.findings.some(finding => finding.code === 'clippedLayout'
        && finding.overflow?.boundary === 'the preview viewport'), JSON.stringify(compactReport));
    const original = xaml.document.getText();
    const clipped = original.replace('Width="480" Height="135"', 'Width="480" Height="135" ClipToBounds="True"');
    content(await invoke('designer_source', { session: xaml.session, expected_version: xaml.document.version, source: clipped }));
    const refreshed = await xaml.read();
    assert.ok(!refreshed.some(finding => finding.name === 'OverhangingCard' && finding.code === 'layoutOverflow'));
    assert.equal(refreshed.find(finding => finding.name === 'OverhangingCard' && finding.code === 'clippedLayout')?.overflow?.right, 40);
    content(await invoke('designer_history', { session: xaml.session, expected_version: xaml.document.version, action: 'undo' }));
    assert.equal(xaml.document.getText(), original);

    const rust = [
        'fn heading() -> Element { rsx! { TextBlock { text: "🌿 Release review" } } }',
        'fn queue() -> Element { rsx! { Canvas { width: 360.0, height: 140.0,',
        '    Border { name: "Queue", width: 44.0, height: 40.0, Canvas.left: 338.0, Canvas.right: 14.0 }',
        '    TextBlock { text: status() }',
        '} } }',
        '',
    ].join('\r\n');
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'LayoutReview.rs');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(rust));
    const mapped = await open('LayoutReview.rs', 1);
    const mappedFindings = await mapped.read();
    assert.equal(text(mapped.document, mappedFindings.find(finding => finding.property === 'Canvas.Right')), '14.0');
    assert.equal(mappedFindings.find(finding => finding.code === 'layoutOverflow' && finding.name === 'Queue')?.overflow?.right, 22);
    const stages = content<StageModel>(await invoke('designer_stages_open', { session: mapped.session }));
    const stage = stages.stages[0];
    const report = content<StageChecks>(await invoke('designer_stage_checks', { session: mapped.session, stage: stage.id, expected_epoch: stage.epoch }));
    assert.equal(text(mapped.document, report.findings.find(finding => finding.property === 'Canvas.Right')), '14.0');
    assert.equal(mapped.document.getText(), rust);
    console.log('VS Code and MCP measured clipping and overflow, reported ineffective native layout values, and linked XAML and mapped RSX findings to source');
}

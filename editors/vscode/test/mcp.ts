import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';
import { verifyCanvasInteraction, verifyCanvasScroll } from './canvasInteraction';
import { verifyWorkspaceMcp } from './workspaceMcp';
import { verifyTimelineMcp } from './studioTimelineMcp';

interface ToolResponse {
    content: { type: string; text?: string; data?: string }[];
    isError: boolean;
}

export interface McpFixture {
    invoke(name: string, args?: object): Promise<ToolResponse>;
    content<T>(result: ToolResponse): T;
}

export async function verifyMcp(document: vscode.TextDocument): Promise<void> {
    const definitions = await vscode.commands.executeCommand<vscode.McpServerDefinition[]>('ferroui.test.mcpDefinitions');
    const server = definitions.find(definition => definition instanceof vscode.McpStdioServerDefinition) as vscode.McpStdioServerDefinition | undefined;
    assert.ok(server, 'VS Code receives the bundled FerroUI MCP definition');
    const child = spawn(server.command, server.args, { stdio: 'pipe', windowsHide: true,
        env: { ...process.env, ...Object.fromEntries(Object.entries(server.env).filter(([,value]) => value !== null).map(([key, value]) => [key, String(value)])) } });
    let output = '';
    child.stderr.on('data', chunk => { output += chunk.toString(); });
    let id = 0;
    const readers = new Map<number, (value: unknown) => void>();
    const lines = createInterface({ input: child.stdout });
    lines.on('line', line => {
        const message = JSON.parse(line);
        readers.get(message.id)?.(message);
    });
    child.on('error', error => { output = error.message; });
    const request = <T>(method: string, params = {}) => new Promise<T>((resolve, reject) => {
        const key = ++id;
        const timer = setTimeout(() => {
            readers.delete(key);
            reject(new Error(`MCP ${method} timed out. ${output}`));
        }, 15_000);
        readers.set(key, raw => {
            clearTimeout(timer);
            readers.delete(key);
            const message = raw as { error?: { message: string }; result?: T };
            if (message.error) reject(new Error(message.error.message));
            else resolve(message.result!);
        });
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: key, method, params }) + '\n');
    });
    const invoke = (name: string, args = {}) => request<ToolResponse>('tools/call', { name, arguments: args });
    const content = <T>(result: ToolResponse): T => {
        assert.equal(result.isError, false, result.content[0]?.text);
        return JSON.parse(result.content.find(item => item.type === 'text')!.text!);
    };
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const original = document.getText();
    try {
        const hello = await request<{ protocolVersion: string }>('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'vscode-test', version: '1' } });
        assert.equal(hello.protocolVersion, '2025-11-25');
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
        const { tools } = await request<{ tools: { name: string }[] }>('tools/list');
        assert.ok(tools.some(tool => tool.name === 'designer_place'));
        const { designs } = content<{ designs: { uri: string; session: number }[] }>(await invoke('ferroui_designs'));
        const session = designs.find(design => design.uri === document.uri.toString())!.session;
        const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(await invoke('designer_outline', { session }));
        const button = nodes.find(node => node.name === 'saveButton');
        assert.ok(button);
        const studio = content<{ tools: { name: string }[]; edits: { name: string }[] }>(await invoke('designer_studio_tools', { session }));
        assert.ok(studio.tools.some(tool => tool.name === 'designer_ast'));
        assert.ok(studio.tools.some(tool => tool.name === 'designer_events'));
        assert.ok(studio.edits.some(tool => tool.name === 'designer_program'));
        content(await invoke('designer_studio_query', { session, tool: 'designer_ast', arguments: { depth: 2 } }));
        const combined = content<{ nodes: { visual: boolean; logical: boolean }[] }>(
            await invoke('designer_studio_query', { session, tool: 'designer_tree', arguments: { tree: 'combined' } }));
        assert.ok(combined.nodes.some(row => row.visual && row.logical));
        const inspector = content<{ properties: { name: string; group: string }[]; events: { name: string }[] }>(
            await invoke('designer_inspect', { session, node: button.id }));
        assert.ok(inspector.properties.some(field => field.name === 'Width' && field.group === 'Layout'));
        assert.ok(inspector.events.some(field => field.name === 'Click'));
        const assets = content<{ assets: { id: string }[] }>(await invoke('designer_assets', { session }));
        assert.ok(assets.assets.some(asset => asset.id === 'Assets/Preview Image.png'));
        const toolbox = content<{ assets: { id: string; category: string }[] }>(await invoke('designer_toolbox', { session }));
        const reviewAction = toolbox.assets.find(asset => asset.id === 'project-file:Review Action');
        assert.equal(reviewAction?.category, 'Project snippets');
        const image = await invoke('designer_preview_image', { session });
        assert.equal(image.isError, false);
        assert.equal(Buffer.from(image.content.find(item => item.type === 'image')!.data!, 'base64').subarray(1, 4).toString(), 'PNG');
        const edit = { session, expected_revision: revision, label: 'Test agent edit', edits: [{ SetAttribute: { node: button.id, name: 'Content', value: 'MCP edited this buffer' } }] };
        const denied = await invoke('designer_apply', edit);
        assert.ok(denied.isError && denied.content[0]?.text?.includes('allowWrites'));
        const designData = { session, expected_revision: revision, tool: 'designer_design_data_edit',
            arguments: { action: 'set_value', node: button.id, property: 'Content', value: 'Studio sample' } };
        assert.ok((await invoke('designer_studio_edit', designData)).isError);
        assert.ok((await invoke('ferroui_workspace_create', { path: 'Denied.rs', source: 'fn main() {}' })).isError);
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const stages = content<{ epoch: number; stages: Array<{ id: number; epoch: number }> }>(await invoke('designer_stages_open', { session }));
        assert.equal(stages.stages.length, 2);
        const first = stages.stages[0];
        const stageImage = await invoke('designer_stage_image', { session, stage: first.id, expected_epoch: first.epoch });
        assert.equal(Buffer.from(stageImage.content.find(item => item.type === 'image')!.data!, 'base64').subarray(1, 4).toString(), 'PNG');
        const changedStages = content<{ stages: unknown[] }>(await invoke('designer_stage_change', { session, expected_epoch: stages.epoch,
            arguments: { action: 'add', name: 'Agent comparison', width: 420, height: 350, dark: true } }));
        assert.equal(changedStages.stages.length, 3);
        assert.ok((await invoke('designer_stage_change', { session, expected_epoch: stages.epoch, arguments: { action: 'remove', id: first.id } })).isError);
        const board = vscode.window.tabGroups.all.flatMap(group => group.tabs).find(tab => tab.label === 'Stages · SettingsView.xaml');
        if (board) await vscode.window.tabGroups.close(board);
        content(await invoke('designer_studio_edit', designData));
        assert.match(document.getText(), /d:Content="Studio sample"/);
        assert.ok((await invoke('designer_studio_edit', designData)).isError);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        const snippetFrame = content<{ revision: number }>(await invoke('designer_wait', { session }));
        const snippet = { session, expected_revision: snippetFrame.revision, asset: reviewAction!.id, anchor: button.id, position: 'after' };
        content(await invoke('designer_placement_hint', snippet));
        content(await invoke('designer_place', snippet));
        assert.match(document.getText(), /Content="Review selected item"/);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        const layoutFrame = content<{ revision: number; nodes: { label: string; id: number }[] }>(
            await invoke('designer_outline', { session }));
        const borders = layoutFrame.nodes.filter(node => node.label === 'Border');
        const selection = [button.id, borders[1].id];
        const arguments_ = { operation: 'reset_alignment' };
        const layout = content<{ model: string; edit_count: number }>(await invoke('designer_studio_query', {
            session, tool: 'designer_selection_layout', arguments: arguments_, selection,
        }));
        assert.ok(layout.edit_count >= 2);
        content(await invoke('designer_studio_edit', { session, expected_revision: layoutFrame.revision,
            tool: 'designer_selection_layout_edit', arguments: { ...arguments_, expected_model: layout.model }, selection }));
        assert.ok(!document.getText().includes('HorizontalAlignment='));
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        edit.expected_revision = content<{ revision: number }>(await invoke('designer_wait', { session })).revision;
        content(await invoke('designer_apply', edit));
        assert.ok(document.isDirty && document.getText().includes('Content="MCP edited this buffer"'));
        assert.equal((await invoke('designer_apply', edit)).isError, true);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        const latest = content<{ revision: number }>(await invoke('designer_inspect', { session, node: button.id }));
        content(await invoke('designer_set_event', { session, node: button.id, expected_revision: latest.revision,
            event: 'Click', action: 'set', handler: 'OnSave' }));
        assert.match(document.getText(), /Click="OnSave"/);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        await verifyTimelineMcp({ invoke, content }, session, document);
        const canvas = content<{ tab: string; scale: number; panTool: boolean }>(await invoke('designer_canvas', { session, tab: 'preview', scale: 2, pan_tool: true }));
        assert.deepEqual([canvas.tab, canvas.scale, canvas.panTool], ['preview', 2, true]);
        assert.equal(content<{ tab: string }>(await invoke('designer_canvas', { session, tab: 'counterpart', pan_tool: false })).tab, 'counterpart');
        await verifyCanvasInteraction({ invoke, content }, session, document);
        const language = content<{ result: { items: { name: string; range: { start: { line: number } } }[] } }>(
            await invoke('ferroui_workspace_language', { path: 'SettingsView.xaml', action: 'symbols' }));
        const saveLine = document.positionAt(original.indexOf('x:Name="saveButton"')).line;
        assert.ok(language.result.items.some(item => item.name === 'saveButton' && item.range.start.line === saveLine),
            JSON.stringify(language.result));
        const rust = content<{ session: number }>(await invoke('designer_open', { path: 'views.rs' }));
        const frame = content<{ revision: number; blocks: { active: boolean }[] }>(await invoke('designer_wait', { session: rust.session }));
        assert.equal(frame.blocks.length, 2);
        const rustSource = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'views.rs'));
        await verifyCanvasInteraction({ invoke, content }, rust.session, rustSource);
        const interactRevision = content<{ revision: number }>(await invoke('designer_wait', { session: rust.session })).revision;
        const selected = content<{ blocks: { active: boolean }[] }>(await invoke('designer_block', { session: rust.session,
            expected_revision: interactRevision, index: 1 }));
        assert.deepEqual(selected.blocks.map(block => block.active), [false, true]);
        await verifyCanvasScroll({ invoke, content });
        await verifyWorkspaceMcp({ invoke, content });
        console.log('VS Code MCP discovery, project Toolbox placement, shared Studio queries, PNG content, write permissions, stale revisions and editor undo passed');
    } finally {
        await config.update('allowWrites', undefined, vscode.ConfigurationTarget.Global);
        lines.close();
        child.stdin.end();
        child.kill();
    }
}

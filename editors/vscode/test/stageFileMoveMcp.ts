import { strict as assert } from 'node:assert';
import { pathToFileURL } from 'node:url';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { StageModel, StageInfo } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyStageFileMove({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Open stage move');
    const before = vscode.Uri.joinPath(root, 'Customer accounts.xaml');
    const after = vscode.Uri.joinPath(root, 'Archive', 'Accounts & notes.xaml');
    const index = vscode.Uri.joinPath(root, 'Company.xaml');
    const relative = (uri: vscode.Uri) => vscode.workspace.asRelativePath(uri, false).replaceAll('\\', '/');
    const sourceUri = (uri: vscode.Uri) => pathToFileURL(uri.fsPath).toString();
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(root, 'Archive'));
    await Promise.all([before, index].map((uri, i) => vscode.workspace.fs.writeFile(uri, Buffer.from(
        `<Border xmlns="https://github.com/ferroui" Background="#124567"><TextBlock Text="Customer view ${i}"/></Border>`,
    ))));
    const open = async (uri: vscode.Uri) => content<{ session: number }>(await invoke('designer_open', { path: relative(uri) })).session;
    const primary = await open(before);
    const comparison = await open(index);
    const board = async (session: number, open = false) => content<StageModel>(
        await invoke(open ? 'designer_stages_open' : 'designer_stages', { session }),
    );
    const edit = async (session: number, model: StageModel, args: object) => content<StageModel>(
        await invoke('designer_stage_change', { session, expected_epoch: model.epoch, arguments: args }),
    );
    const image = (session: number, stage: StageInfo) => invoke('designer_stage_image', {
        session, stage: stage.id, expected_epoch: stage.epoch,
    });
    const png = async (session: number, stage: StageInfo) => {
        const reply = await image(session, stage);
        const data = reply.content.find(item => item.type === 'image')?.data;
        assert.ok(data, JSON.stringify(reply.content[0]));
        return data;
    };
    const primaryBoard = await board(primary, true);
    const [desktop, compact] = primaryBoard.stages;
    const paused = await edit(primary, primaryBoard, {
        action: 'update', id: compact.id, configuration: { ...compact.configuration, auto_update: false },
    });
    const withAccounts = await edit(comparison, await board(comparison, true), {
        action: 'add', sourcePath: relative(before), name: 'Accounts', width: 400, height: 300, dark: false,
    });
    const linked = await edit(comparison, withAccounts, {
        action: 'setPrototype', graph: { flows: [{ name: 'Review', start: 2 }], links: [] },
    });
    const retained = await png(primary, paused.stages[1]);
    const linkedStage = linked.stages[2];
    const args = { path: relative(before), destination: relative(after) };
    const review = content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', args));
    content<FileMovePlan>(await invoke('ferroui_workspace_move_xaml', { ...args, review: review.review }));
    const assertBinding = async (uri: vscode.Uri) => {
        // Both panels follow the rename before the existing design session can read its board.
        await until(() => {
            const title = uri.path.split('/').at(-1);
            const labels = vscode.window.tabGroups.all.flatMap(group => group.tabs.map(tab => tab.label));
            return labels.includes(`Design · ${title}`) && labels.includes(`Stages · ${title}`);
        });
        const [first, second] = await Promise.all([board(primary), board(comparison)]);
        assert.deepEqual(first.stages.map(stage => stage.id), [desktop.id, compact.id]);
        assert.deepEqual(first.stages.map(stage => stage.configuration.source_uri), [sourceUri(uri), sourceUri(uri)]);
        assert.equal(first.stages[1].status.state, 'paused');
        assert.equal(await png(primary, first.stages[1]), retained);
        assert.deepEqual(second.stages.map(stage => stage.id), linked.stages.map(stage => stage.id));
        assert.deepEqual(second.manifest.prototype, linked.manifest.prototype);
        assert.equal(second.stages[0].configuration.source_uri, sourceUri(index));
        assert.equal(second.stages[2].configuration.source_uri, sourceUri(uri));
        assert.notEqual(second.stages[2].epoch, linkedStage.epoch);
    };
    await assertBinding(after);
    assert.ok((await image(primary, desktop)).isError, 'A moved stage retires its old frame epoch');
    assert.ok((await image(comparison, linkedStage)).isError, 'A moved secondary source retires its old frame epoch');
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(after));
    await vscode.commands.executeCommand('undo');
    await assertBinding(before);
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(before));
    await vscode.commands.executeCommand('redo');
    await assertBinding(after);
    console.log('Open Stages boards followed primary and secondary XAML sources through Apply, Undo, and Redo');
}

import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { fitStageCamera, type StageCamera, type StageInfo, type StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcpFixture';

export async function verifyStageCamera({ invoke, content }: McpFixture): Promise<void> {
    for (const [path, source] of [
        ['StageCamera.xaml', '<Button xmlns="https://github.com/ferroui" Content="Camera"/>'],
        ['stage_camera.rs', 'fn view() { rsx! { Button { "Camera" } }; }'],
    ] as const) {
        content(await invoke('ferroui_workspace_create', { path, source }));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
        const version = document.version;
        const change = (board: StageModel, args: object) => invoke('designer_stage_change', {
            session, expected_epoch: board.epoch, arguments: args,
        });
        const cameraChange = ({ id, epoch }: StageInfo, camera: StageCamera) => ({ action: 'camera', target: { id, epoch }, camera });
        const capture = async ({ id, epoch }: StageInfo) => {
            const reply = await invoke('designer_stage_image', { session, stage: id, expected_epoch: epoch });
            const png = reply.content.find(item => item.type === 'image')?.data;
            assert.ok(png, reply.content[0]?.text);
            return png;
        };
        const initial = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.deepEqual(initial.cameraZooms, [25, 50, 75, 100, 150, 200]);
        const [first, peer] = initial.stages;
        const pixels = await capture(first);
        const camera = { zoom_percent: 150, x: 84, y: 72 };
        const configured = content<StageModel>(await change(initial, cameraChange(first, camera)));
        assert.deepEqual(configured.stages[0].camera, camera);
        assert.deepEqual(configured.stages[1].camera, fitStageCamera);
        assert.deepEqual(configured.manifest, initial.manifest);
        assert.equal(configured.epoch, initial.epoch);
        assert.equal(await capture(configured.stages[0]), pixels);
        assert.ok((await change(configured, cameraChange({ ...first, epoch: first.epoch + 1 }, camera))).isError);
        assert.ok((await change(configured, cameraChange(first, { ...camera, zoom_percent: 37 }))).isError);
        assert.ok((await change(configured, cameraChange(first, { ...fitStageCamera, x: 1 }))).isError);
        const moved = content<StageModel>(await change(configured, { action: 'move', target: { id: peer.id, epoch: peer.epoch }, index: 0 }));
        assert.deepEqual(moved.stages.map(stage => stage.camera), [fitStageCamera, camera]);
        assert.ok((await change(configured, cameraChange(first, fitStageCamera))).isError);
        const tabs = () => vscode.window.tabGroups.all.flatMap(group => group.tabs);
        await vscode.window.tabGroups.close(tabs().filter(tab => tab.label === `Stages · ${path}`));
        const reopened = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.deepEqual(reopened.manifest, moved.manifest);
        assert.deepEqual(reopened.stages.map(stage => stage.camera), [fitStageCamera, camera]);
        const saved = reopened.manifest;
        assert.ok((await change(reopened, { action: 'restore', manifest: { ...saved, version: 999 } })).isError);
        assert.ok((await change(reopened, { action: 'restore', manifest: { ...saved, stages: [null] } })).isError);
        const outside = { ...saved, stages: [{ ...saved.stages[0], source_uri: 'untitled:Outside.xaml' }] };
        assert.ok((await change(reopened, { action: 'restore', manifest: outside })).isError);
        const stillOpen = content<StageModel>(await invoke('designer_stages', { session }));
        assert.equal(stillOpen.epoch, reopened.epoch);
        assert.deepEqual(stillOpen.stages.map(stage => stage.camera), [fitStageCamera, camera]);
        const restored = content<StageModel>(await change(reopened, { action: 'restore', manifest: saved }));
        assert.deepEqual(restored.manifest, saved);
        assert.deepEqual(restored.stages.map(stage => stage.camera), [fitStageCamera, fitStageCamera]);
        assert.notEqual(restored.stages[0].id, reopened.stages[0].id);
        assert.ok((await change(reopened, { action: 'restore', manifest: saved })).isError);
        assert.equal(document.version, version);
        assert.equal(document.getText(), source);
        await vscode.window.tabGroups.close(tabs().filter(tab => [path, `Design · ${path}`, `Stages · ${path}`].includes(tab.label)));
    }
    console.log('VS Code MCP verified independent stage cameras, guarded manifest restore, native pixels, and restored card order for XAML and RSX');
}

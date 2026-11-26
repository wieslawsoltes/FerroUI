import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CanvasGuidesState, SnapLine, SnapPreferences } from '../src/canvasGuidesProtocol';
import type { Bounds, DropHint, OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

type Guides = CanvasGuidesState & { revision: number };
type Resize = { dx: number; dy: number; snapLines: SnapLine[] };
const xaml = `<Canvas xmlns="https://github.com/ferroui">
  <Border Name="Moving" Width="70" Height="38" Canvas.Left="31.5" Canvas.Top="64" Background="#3F91B5"/>
  <Border Name="Second" Width="54" Height="38" Canvas.Left="125.5" Canvas.Top="64" Background="#22796C"/>
  <Border Name="Peer" Width="94" Height="80" Canvas.Left="140.5" Canvas.Top="180" Background="#29775D"/>
  <StackPanel Name="Flow" Canvas.Left="128" Canvas.Top="280" Width="160" Height="140" Background="White">
    <Button Content="Existing" Height="40"/>
  </StackPanel>
</Canvas>`;
const rust = ['use ferroui_view::prelude::*;', 'fn review() -> Element {', '  rsx! { Canvas {',
    '    // 🪴 keep this comment', '    Border { name: "Moving", width: 70.0, height: 38.0, Canvas.left: 31.5, Canvas.top: 64.0 }',
    '  } }', '}', 'fn sibling() -> Element { rsx! { TextBlock { text: "Keep this invocation" } } }', ''].join('\r\n');

export async function verifyCanvasSnapping({ invoke, content }: McpFixture): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const open = async (path: string, source: string) => {
        const uri = vscode.Uri.joinPath(folder, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        const { session } = content<{ session: number }>(await invoke('designer_open', { path, line: 0 }));
        const read = async () => content<Guides>(await invoke('designer_canvas_guides', { session }));
        const edit = async (model: Guides, edit: object) => content<Guides>(await invoke('designer_canvas_guides_edit', {
            session, expected_revision: model.revision, expected_epoch: model.epoch, edit,
        }));
        const undo = async () => {
            content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
            assert.equal(document.getText(), source);
        };
        const control = async (name: string) => {
            await read();
            return content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session })).nodes.find(node => node.name === name)!;
        };
        const configure = async (options: Partial<SnapPreferences> = {}) => edit(await read(), { action: 'configure_snap', options: {
            grid_visible: false, grid_spacing: 16, snap_to_grid: false, snap_to_guides: true, snap_to_objects: false, snap_tolerance: 6, ...options,
        } });
        return { session, document, read, edit, undo, control, node: async (name: string) => (await control(name)).id, configure };
    };
    const opts = (model: Guides, zoom = 1, bypass = false) => ({
        session: model.frame.session, expected_revision: model.revision, snap: { epoch: model.epoch, zoom, bypass },
    });
    const bounds = (value: DropHint) => value.bounds as Bounds;
    const hint = async (args: object) => content<DropHint>(await invoke('designer_placement_hint', args));
    const place = async (args: object) => content(await invoke('designer_place', args));
    const guideEdit = { action: 'apply', rulers: false, guides: false,
        lines: [{ axis: 'Vertical', position: 105.5 }, { axis: 'Horizontal', position: 100 }] };
    const view = await open('canvas-snapping.xaml', xaml);
    const baseline = await view.read();
    assert.equal(baseline.settings.snap_to_guides, false);
    const hiddenGuides = await view.edit(baseline, guideEdit);
    const configured = await view.configure();
    const moving = await view.node('Moving');
    const move = { node: moving, x: 101, y: 97 };
    const base = await hint({ ...move, ...opts(configured) });
    assert.deepEqual([bounds(base).x, bounds(base).y], [105.5, 100]);
    assert.deepEqual(base.snapLines, [{ axis: 'Horizontal', position: 105.5 }, { axis: 'Vertical', position: 100 }]);
    for (const options of [opts(configured, 4), opts(configured, 1, true)]) {
        const raw = await hint({ ...move, ...options });
        assert.deepEqual([bounds(raw).x, bounds(raw).y, raw.snapLines?.length], [101, 97, 0]);
    }
    assert.ok((await invoke('designer_placement_hint', { ...move, ...opts(hiddenGuides) })).isError);
    await place({ ...move, ...opts(configured) });
    assert.match(view.document.getText(), /Name="Moving"[^>]*Canvas.Left="105.5" Canvas.Top="100"/);
    await view.undo();

    const group = { selection: [await view.node('Moving'), await view.node('Second')], x: 101, y: 97 };
    const groupArgs = { ...group, ...opts(await view.read()) };
    assert.equal(bounds(await hint(groupArgs)).x, 105.5);
    await place(groupArgs);
    assert.match(view.document.getText(), /Name="Second"[^>]*Canvas.Left="199.5" Canvas.Top="100"/);
    await view.undo();

    const resize = { ...opts(await view.read()), node: await view.node('Moving'), handle: 3, dx: 3, dy: 6 };
    const preview = content<Resize>(await invoke('designer_resize_hint', resize));
    assert.deepEqual([preview.dx, preview.dy, preview.snapLines], [4, 6, [{ axis: 'Horizontal', position: 105.5 }]]);
    content(await invoke('designer_resize', resize));
    assert.match(view.document.getText(), /Name="Moving" Width="74" Height="38"/);
    await view.undo();

    const peers = await view.configure({ snap_to_guides: false, snap_to_objects: true });
    const peerX = (await view.control('Peer')).bounds!.x;
    const peer = await hint({ ...opts(peers), node: await view.node('Moving'), x: peerX - .25, y: 110 });
    assert.equal(bounds(peer).x, peerX);
    const grid = await view.configure({ snap_to_guides: false, snap_to_grid: true, grid_visible: true });
    const toolbox = content<{ assets: { id: string; title: string }[] }>(await invoke('designer_toolbox', { session: view.session }));
    const asset = toolbox.assets.find(item => item.title === 'Button')!.id;
    const insert = { ...opts(grid), asset, x: 155, y: 125 };
    const gridHint = await hint(insert);
    assert.deepEqual([bounds(gridHint).x, bounds(gridHint).y], [160, 128]);
    await place(insert);
    assert.match(view.document.getText(), /Canvas.Left="160" Canvas.Top="128"/);
    await view.undo();
    const flow = { ...opts(await view.read()), asset, x: 151, y: 330 };
    const flowHint = await hint(flow);
    assert.deepEqual(flowHint.snapLines, []);
    assert.deepEqual(flowHint.bounds, (await hint({ ...flow, snap: undefined })).bounds);

    const rsx = await open('canvas-snapping.rs', rust);
    await rsx.edit(await rsx.read(), guideEdit);
    await rsx.configure();
    const rustMove = { ...opts(await rsx.read()), ...move, node: await rsx.node('Moving') };
    assert.equal(bounds(await hint(rustMove)).x, 105.5);
    await place(rustMove);
    const authored = rsx.document.getText();
    const snappedProperties = /Canvas\.left: 105\.5, Canvas\.top: 100(?:\.0)?(?=\s*})/;
    assert.match(authored, snappedProperties);
    assert.equal(authored.replace(snappedProperties, 'Canvas.left: 31.5, Canvas.top: 64.0'), rust);
    await rsx.undo();
    console.log('VS Code preview and MCP snapped Canvas insert, group moves and resize, preserved layout slots and Rust source, and validated zoom, bypass, stale settings and Undo');
}

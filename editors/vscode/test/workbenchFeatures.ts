import { strict as assert } from 'node:assert';
import type { NativePreview } from '../src/nativePreview';
import type { Source, TextEdit } from '../src/protocol';

const viewport = { width: 640, height: 420, dark: false };
const caret = { line: 0, character: 0 };

export async function verifyWorkbenchFeatures(preview: NativePreview, xamlUri: string, rustUri: string): Promise<void> {
    let xaml: Source = { language: 'xaml', uri: xamlUri, caret, source: `
<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <UserControl.Resources><SolidColorBrush x:Key="AccentBrush" Color="#655DD7"/></UserControl.Resources>
    <Border Padding="48"><StackPanel Name="Body"><Button Name="Save" Click="OnSave" Content="Save" Background="#FF655DD7"/></StackPanel></Border>
</UserControl>` };
    const frame = await preview.render(xaml, viewport, 10);
    assert.ok(frame.image, frame.renderError ?? 'XAML events preserve a frame');
    assert.ok(frame.notices.some(notice => notice.includes('event handlers')));
    const button = frame.nodes.find(node => node.name === 'Save')!;
    const details = await preview.inspect(xaml, 10, { type: 'source', node: button.id });
    assert.equal(details.events.find(event => event.name === 'Click')?.handler, 'OnSave');
    assert.ok(details.properties.some(field => field.name === 'Width' && field.group === 'Layout' && field.editable));
    assert.ok(details.properties.some(field => field.name === 'Background' && field.kind === 'color' && field.effective === '#FF655DD7'));
    assert.ok(!details.properties.some(field => field.name === 'Click'));
    const runtime = await preview.studioQuery(10, 'designer_tree', { tree: 'combined' }) as {
        nodes: { inspector_id: number; visual: boolean; logical: boolean; source_id: number | null }[];
    };
    assert.ok(runtime.nodes.some(row => row.source_id === button.id && row.visual && row.logical));
    const generated = runtime.nodes.find(row => row.source_id === null && row.visual)!;
    assert.ok(generated);
    const inspected = await preview.inspect(xaml, 10, { type: 'runtime', id: generated.inspector_id });
    assert.ok(inspected.properties.length && inspected.properties.every(field => !field.editable));
    await assert.rejects(preview.inspect(xaml, 9, { type: 'runtime', id: generated.inspector_id }), /current preview/);

    const resources = await preview.resources(xaml, 10, button.id);
    const accent = resources.find(resource => resource.name === 'AccentBrush')!;
    assert.ok(accent.visible && accent.properties.includes('Background'));
    const applied = await preview.edit(xaml, { type: 'applyResource', frame: 10, node: button.id,
        resource: accent.id, property: 'Background', dynamic: false });
    assert.match(apply(xaml.source, applied.edits), /Background="\{StaticResource 'AccentBrush'\}"/);
    const ownHandler = await preview.edit(xaml, { type: 'setEvent', node: button.id, name: 'Click', value: 'OnApply' });
    assert.match(apply(xaml.source, ownHandler.edits), /Click="OnApply"/);
    const width = await preview.edit(xaml, { type: 'setProperty', node: button.id, name: 'Width', value: '180' });
    const next = { ...xaml, source: apply(xaml.source, width.edits) };
    assert.match(next.source, /Width="180"/);
    const reset = await preview.edit(next, { type: 'resetProperty', node: button.id, name: 'Width' });
    assert.ok(!apply(next.source, reset.edits).includes('Width='));

    const point = { type: 'canvas', x: 10, y: 10 } as const;
    const placement = { target: point, origin: { type: 'image', source: 'Assets/Preview%20Image.png' } } as const;
    assert.ok((await preview.placement(10, placement)).bounds, 'The wrapper gutter resolves to its content body');
    const placed = await preview.edit(xaml, { type: 'place', frame: 10, placement });
    xaml = { ...xaml, source: apply(xaml.source, placed.edits) };
    assert.match(xaml.source, /Source="Assets\/Preview%20Image.png"/);
    const withAsset = await preview.render(xaml, viewport, 11);
    assert.ok(withAsset.image, withAsset.renderError ?? 'Relative assets load in the artboard');
    assert.ok(withAsset.nodes.some(node => node.label === 'Image' && node.bounds?.height));

    let rust: Source = { language: 'rust', uri: rustUri, caret, source: `
fn view() { rsx! {
    StackPanel {
        // a sibling comment
        Button { name: "save", on_click: move |_| save(), "Save" }
        TextBlock { name: "label", text: move || title() }
    }
} }` };
    const partial = await preview.render(rust, viewport, 12);
    const save = partial.nodes.find(node => node.name === 'save')!;
    const label = partial.nodes.find(node => node.name === 'label')!;
    assert.ok(partial.image && save.canMove && label.canMove);
    const info = await preview.inspect(rust, 12, { type: 'source', node: save.id });
    assert.equal(info.events.find(event => event.name === 'Click')?.handler, 'move |_| save()');
    const movement = await preview.edit(rust, { type: 'place', frame: 12,
        placement: { origin: { type: 'node', node: save.id }, target: { type: 'outline', node: label.id, position: 'after' } } });
    const moved = apply(rust.source, movement.edits);
    assert.ok(moved.indexOf('name: "label"') < moved.indexOf('name: "save"'));
    assert.match(moved, /move \|_\| save\(\)/);
    assert.match(moved, /sibling comment/);
    assert.match(moved, /move \|\| title\(\)/);
    rust = { ...rust, source: moved };
    const movedFrame = await preview.render(rust, viewport, 13);
    const movedSave = movedFrame.nodes.find(node => node.name === 'save')!;
    const handler = await preview.edit(rust, { type: 'setEvent', node: movedSave.id, name: 'Click', value: 'move |_| apply()' });
    assert.equal(apply(rust.source, handler.edits), rust.source.replace('move |_| save()', 'move |_| apply()'));
    await assert.rejects(preview.edit(rust, { type: 'setEvent', node: movedSave.id, name: 'Click', value: 'move |' }), /Invalid handler/);
    const dynamic = { ...rust, source: 'fn view() { rsx! { StackPanel { {extra()} Button { "Save" } } } }' };
    const dynamicFrame = await preview.render(dynamic, viewport, 14);
    assert.ok(dynamicFrame.image);
    assert.equal(dynamicFrame.nodes[0].canInsert, false);
    await assert.rejects(preview.placement(14, { origin: { type: 'asset', control: 'Button' },
        target: { type: 'outline', node: dynamicFrame.nodes[0].id, position: 'inside' } }), /fully mapped children/);
    console.log('VS Code native hierarchy, grouped properties, resources, relative image drops and expression-preserving RSX placement passed');
}

export async function verifyMultiSelection(preview: NativePreview, xamlUri: string, rustUri: string): Promise<void> {
    for (const design of [
        { language: 'xaml', uri: xamlUri, caret, source: `<StackPanel xmlns="https://github.com/ferroui"><Button Name="First" Width="120"/><Button Name="Second" Width="72"/><Button Name="Third" Width="92"/></StackPanel>` },
        { language: 'rust', uri: rustUri, caret, source: `fn view() { let _ = rsx! { StackPanel { Button { name: "First", width: 120.0 } Button { name: "Second", width: 72.0 } Button { name: "Third", width: 92.0 } } }; }\n// outside the invocation` },
    ] as const) {
        const frame = await preview.render(design, viewport, 25);
        assert.ok(frame.image, frame.renderError ?? undefined);
        const ids = ['First', 'Second', 'Third'].map(name => frame.nodes.find(node => node.name === name)!.id);
        const mixed = await preview.inspect(design, 25, { type: 'selection', nodes: ids });
        assert.equal(mixed.properties.find(property => property.name === 'Width')?.mixed, true);
        const size = await preview.edit(design, { type: 'resize', frame: 25, node: ids[0], handle: 3, dx: 20, dy: 0 });
        assert.match(apply(design.source, size.edits), /140(?:\.0)?/);
        const placement = { origin: { type: 'nodes', nodes: ids.slice(0, 2), anchor: ids[0] },
            target: { type: 'outline', node: ids[2], position: 'after' } } as const;
        await preview.placement(25, placement);
        const moved = await preview.edit(design, { type: 'place', frame: 25, placement });
        const movedSource = apply(design.source, moved.edits);
        assert.ok(movedSource.indexOf('Third') < movedSource.indexOf('First') && movedSource.indexOf('First') < movedSource.indexOf('Second'));
        const layout = { operation: 'match_width' };
        const plan = await preview.studioQuery(25, 'designer_selection_layout', layout, ids) as { model: string; edit_count: number };
        assert.ok(plan.edit_count >= 2, 'The native plan can include companion layout resets');
        const resized = await preview.studioEdit(design, 25, 'designer_selection_layout_edit', { ...layout, expected_model: plan.model }, ids);
        assert.equal((apply(design.source, resized.edits).match(/120(?:\.0)?/g) ?? []).length, 3);
        if (design.language === 'rust') {
            assert.ok(movedSource.endsWith('// outside the invocation'));
            assert.ok(apply(design.source, resized.edits).endsWith('// outside the invocation'));
        }
    }
    console.log('VS Code native ordered multi-control placement and measured layout passed in XAML and RSX');
}

export async function verifyStudioPresentations(preview: NativePreview, uri: string): Promise<void> {
    const source: Source = { language: 'xaml', uri, caret, source: '<StackPanel xmlns="https://github.com/ferroui"><Button Name="A" Content="One"/><Button Name="B" Content="Two"/></StackPanel>' };
    const frame = await preview.render(source, viewport, 31);
    const first = frame.nodes.find(node => node.name === 'A')!;
    const tracks = await preview.studioQuery(31, 'designer_layers', {}) as { layer_epoch: number; scope_epoch: number };
    const isolation = await preview.studioPresent(31, 32, 'designer_layer', { action: 'isolate', nodes: [first.id],
        expected_layer_epoch: tracks.layer_epoch, expected_scope_epoch: tracks.scope_epoch }, [first.id]);
    assert.notEqual(isolation.image.png, frame.image?.png);
    assert.equal(isolation.nodes.find(node => node.id === first.id)?.selectable, true);
    await assert.rejects(preview.studioQuery(31, 'designer_layers', {}), /rendered layout changed/i);

    const board = await preview.syncStages(source);
    assert.equal(board.stages.length, 2);
    const target = board.stages[0];
    const screenshot = await preview.stageImage({ id: target.id, epoch: target.epoch });
    assert.equal(screenshot.width, 800);
    const changed = await preview.editStages({ action: 'update', id: target.id, epoch: board.epoch,
        configuration: { ...target.configuration, auto_update: false } });
    assert.equal(changed.stages[0].status.state, 'paused');
    await assert.rejects(preview.editStages({ action: 'remove', id: target.id, epoch: board.epoch }), /current stages board/i);
    const restored = await preview.syncStages(source, changed.manifest);
    assert.notEqual(restored.stages[0].id, target.id);
    assert.equal(restored.stages[0].configuration.auto_update, false);
    const second = { ...source, uri: source.uri.replace(/[^/]+$/, 'PrototypeSecond.xaml'), source: source.source.replace('One', 'Second screen') };
    const desktop = restored.stages[0];
    const resumed = await preview.editStages({ action: 'update', id: desktop.id, epoch: restored.epoch,
        configuration: { ...desktop.configuration, auto_update: true } });
    const screens = await preview.editStages({ action: 'add', epoch: resumed.epoch, name: 'Second screen', width: 640, height: 420, dark: false, source: second });
    assert.equal(screens.stages[2].configuration.source_uri, second.uri);
    const hotspots = await preview.stagePrototypeElements(screens.stages[0]);
    const trigger = hotspots.find(target => target.label === 'Button · A');
    assert.ok(trigger);
    const graph = {
        flows: [{ name: 'Settings flow', start: 0 }],
        links: [{ name: 'Continue', source: 0, trigger: trigger.anchor, action: { kind: 'navigate' as const, stage: 2 } }],
    };
    await assert.rejects(preview.editStages({ action: 'setPrototype', epoch: screens.epoch, graph: { ...graph, flows: [{ name: 'Invalid', start: 100 }] } }),
        /stage index is outside/i);
    const flow = await preview.editStages({ action: 'setPrototype', epoch: screens.epoch, graph });
    assert.deepEqual(flow.manifest.prototype, graph);
    const start = await preview.stagePrototype(flow.epoch, { action: 'start', flow: 'Settings flow' });
    assert.equal(start?.screen, flow.stages[0].id);
    const unchanged = await preview.syncStages(source, undefined, [second]);
    assert.equal(unchanged.epoch, flow.epoch, 'Syncing an unchanged buffer keeps the native stage and prototype running');
    assert.equal(unchanged.prototype?.screen, start?.screen);
    const destination = await preview.stagePrototype(flow.epoch, { action: 'activate', link: 0 });
    assert.equal(destination?.screen, flow.stages[2].id);
    assert.equal((await preview.stagePrototype(flow.epoch, { action: 'back' }))?.screen, flow.stages[0].id);
    const reopened = await preview.syncStages(source, flow.manifest, [second]);
    assert.deepEqual(reopened.manifest.prototype, graph);
    console.log('Studio layers, retained stage images, pause and restore, and a source-anchored multi-document prototype passed');
}

function apply(source: string, edits: TextEdit[]): string {
    const lines = source.split('\n');
    const offset = (line: number, character: number) =>
        lines.slice(0, line).reduce((length, text) => length + text.length + 1, 0) + character;
    const patches = edits.map(({ range, newText }) => ({
        start: offset(range.start.line, range.start.character), end: offset(range.end.line, range.end.character), text: newText,
    })).sort((a, b) => b.start - a.start);
    for (const patch of patches) source = source.slice(0, patch.start) + patch.text + source.slice(patch.end);
    return source;
}

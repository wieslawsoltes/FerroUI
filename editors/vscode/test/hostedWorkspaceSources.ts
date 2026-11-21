import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { pathToFileURL } from 'node:url';
import type { BuiltProject } from '../src/builtProjects';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedPaletteCatalog, HostedPaletteLocation, HostedPalettePlan, HostedPaletteReport } from '../src/hostedPaletteProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { PreviewSourceBuffer } from '../src/protocol';

export async function verifyHostedWorkspaceSources(preview: NativePreview, update: (revision: number, xaml: string) => HostedSource) {
    const directory = await mkdtemp(path.join(tmpdir(), 'ferroui-hosted-sources-'));
    const dictionary = (content: string) => `<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">${content}</ResourceDictionary>`;
    const palette = (color: string) => '\uFEFF' + dictionary(`\r\n  <!-- 🙂 --> <SolidColorBrush x:Key="PanelBrush" Color="${color}"/>\r\n`);
    const view = `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design" Width="380" Height="220">
      <Window.Resources>${dictionary('<ResourceDictionary.MergedDictionaries><ResourceInclude Source="Dictionaries/Outer.xaml"/></ResourceDictionary.MergedDictionaries>')}</Window.Resources>
      <Border Background="{StaticResource PanelBrush}" Padding="20"><app:PreviewBadge Text="Workspace draft"/></Border>
    </Window>`;
    try {
        const root = await realpath(directory);
        const outer = path.join(root, 'Dictionaries', 'Outer.xaml');
        const palettePath = path.join(root, 'Dictionaries', 'Color Set.xaml');
        const paletteUri = pathToFileURL(palettePath).toString();
        const viewPath = path.join(root, 'Test.xaml');
        const uri = pathToFileURL(viewPath).toString();
        await mkdir(path.dirname(outer));
        await writeFile(outer, dictionary('<ResourceDictionary.MergedDictionaries><ResourceInclude Source="Color%20Set.xaml"/></ResourceDictionary.MergedDictionaries>'));
        await writeFile(palettePath, palette('#FF162137'));
        await writeFile(viewPath, view);
        const project = await compiledPaletteProject(root);
        const render = async (revision: number, localSources: PreviewSourceBuffer[] = []) => {
            const { document, ...request } = update(revision, view);
            const source = await preview.inProject({ ...document, uri }, project);
            return preview.renderHostedWorkspace({ ...request, document: source }, { root, localSources });
        };

        const initial = await render(2000);
        assert.equal(initial.error, null);
        assert.deepEqual(initial.dependencies?.sort(), [pathToFileURL(outer).toString(), paletteUri].sort());
        assert.ok(initial.frame);
        const source = await editIncludedPalette(preview, initial.frame, uri, paletteUri, palette('#FF162137'));
        assert.equal(source, palette('#FF23765E'));
        const changed = await render(2001, [{ uri: paletteUri, source }]);
        assert.equal(changed.error, null);
        assert.ok(changed.frame);
        assert.notEqual(changed.frame.image.png, initial.frame.image.png);
        await assert.rejects(preview.hostedPalette(initial.frame, { action: 'catalog' }), /current app-hosted preview frame/);
        await assert.rejects(preview.hostedPalette({ ...changed.frame, sequence: changed.frame.sequence + 1 },
            { action: 'catalog' }), /current app-hosted preview frame/);
        const broken = await render(2002, [{ uri: paletteUri, source: '<ResourceDictionary' }]);
        assert.match(broken.error ?? '', /Repair included XAML/i);
        assert.ok(broken.dependencies?.includes(paletteUri));
        assert.equal(broken.frame, null);
        assert.equal(await preview.hostedFrame(), null);
        await assert.rejects(preview.hostedInput({ type: 'click', x: .5, y: .5, revision: changed.frame.revision, sequence: changed.frame.sequence }), /frame changed/i);
        const restored = await render(2003);
        assert.equal(restored.error, null);
        assert.equal(restored.frame?.image.png, initial.frame.image.png);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
}

async function compiledPaletteProject(root: string): Promise<BuiltProject> {
    const manifest = path.join(root, 'Cargo.toml');
    await writeFile(manifest, '[package]\nname = "todo-workbench"\nversion = "0.0.0"\n');
    const member = (source_path: string, key: string) => ({ source_path, key: `/Exported/${key}`, public: false, symbol: null });
    await writeFile(path.join(root, 'xaml-symbols.json'), JSON.stringify({
        version: 1, manifest, assembly: 'todo-workbench',
        groups: [{ module: 'preview', kind: 'documents', discovery_root: null, file_backed: true, members: [
            member('Test.xaml', 'Workspace.xaml'),
            member('Dictionaries/Outer.xaml', 'Dictionaries/Outer.xaml'),
            member('Dictionaries/Color Set.xaml', 'Dictionaries/Color Set.xaml'),
        ] }],
    }));
    return { workspaceRoot: root, metadata: { manifest, directories: [root], package: 'todo-workbench' } };
}

async function editIncludedPalette(preview: NativePreview, frame: HostedFrame, view: string, uri: string, source: string) {
    assert.equal(frame.paletteAvailable, true);
    const { documents } = await preview.hostedPalette<HostedPaletteCatalog>(frame, { action: 'catalog' });
    const root = documents.find(file => file.uri === view);
    const dictionary = documents.find(file => file.uri === uri);
    assert.ok(root && dictionary);
    assert.equal(dictionary.address, 'ferres://todo-workbench/Exported/Dictionaries/Color%20Set.xaml');
    assert.deepEqual(await preview.hostedPalette(frame, { action: 'source', address: dictionary.address }), { uri });
    await assert.rejects(preview.hostedPalette(frame, { action: 'source', address: uri }), /accepted app preview/);
    const consumer = await preview.hostedPalette<HostedPaletteReport>(frame, {
        action: 'read', target: { address: root.address, owner: root.scopes[0].node },
    });
    const resolved = consumer.palette.comparison.find(row => row.key === 'PanelBrush');
    assert.equal(resolved?.light?.value, '#FF162137');
    assert.equal(resolved?.dark?.source?.uri, uri);
    const target = { address: dictionary.address, owner: dictionary.scopes[0].node };
    const { palette } = await preview.hostedPalette<HostedPaletteReport>(frame, { action: 'read', target });
    const color = palette.layers.flatMap(layer => layer.colors).find(color => color.key === 'PanelBrush');
    assert.ok(color?.editable);
    const { range } = await preview.hostedPalette<HostedPaletteLocation>(frame, {
        action: 'locate', address: dictionary.address, range: color.range, source,
    });
    assert.equal(range.start.line, 1);
    assert.equal(source.split('\r\n')[1].slice(range.start.character, range.end.character), '#FF162137');
    const edit = { action: 'set', node: color.node, color: '#FF23765E' } as const;
    await assert.rejects(preview.hostedPalette(frame, { action: 'plan', target, edit, source: source.replace('🙂', 'changed') }), /buffer changed/);
    const plan = await preview.hostedPalette<HostedPalettePlan>(frame, { action: 'plan', target, edit, source });
    assert.equal(plan.uri, uri);
    assert.equal(plan.patches.length, 1);
    const [patch] = plan.patches;
    assert.equal(source.slice(patch.start, patch.end), patch.expected);
    return source.slice(0, patch.start) + patch.replacement + source.slice(patch.end);
}

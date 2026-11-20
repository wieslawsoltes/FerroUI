import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { pathToFileURL } from 'node:url';
import type { HostedSource } from '../src/hostedProtocol';
import type { NativePreview } from '../src/nativePreview';
import type { PreviewSourceBuffer } from '../src/protocol';

export async function verifyHostedWorkspaceSources(preview: NativePreview, update: (revision: number, xaml: string) => HostedSource) {
    const directory = await mkdtemp(path.join(tmpdir(), 'ferroui-hosted-sources-'));
    const dictionary = (content: string) => `<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">${content}</ResourceDictionary>`;
    const palette = (color: string) => dictionary(`<SolidColorBrush x:Key="PanelBrush">${color}</SolidColorBrush>`);
    const view = `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design" Width="380" Height="220">
      <Window.Resources>${dictionary('<ResourceDictionary.MergedDictionaries><ResourceInclude Source="Dictionaries/Outer.xaml"/></ResourceDictionary.MergedDictionaries>')}</Window.Resources>
      <Border Background="{StaticResource PanelBrush}" Padding="20"><app:PreviewBadge Text="Workspace draft"/></Border>
    </Window>`;
    try {
        const root = await realpath(directory);
        const outer = path.join(root, 'Dictionaries', 'Outer.xaml');
        const palettePath = path.join(root, 'Dictionaries', 'Color Set.xaml');
        const paletteUri = pathToFileURL(palettePath).toString();
        const uri = pathToFileURL(path.join(root, 'Test.xaml')).toString();
        await mkdir(path.dirname(outer));
        await writeFile(outer, dictionary('<ResourceDictionary.MergedDictionaries><ResourceInclude Source="Color%20Set.xaml"/></ResourceDictionary.MergedDictionaries>'));
        await writeFile(palettePath, palette('#FF162137'));
        const render = (revision: number, localSources: PreviewSourceBuffer[] = []) => {
            const { document, ...request } = update(revision, view);
            return preview.renderHostedWorkspace({ ...request, document: { ...document, uri } }, { root, localSources });
        };

        const initial = await render(2000);
        assert.equal(initial.error, null);
        assert.deepEqual(initial.dependencies?.sort(), [pathToFileURL(outer).toString(), paletteUri].sort());
        assert.ok(initial.frame);
        const changed = await render(2001, [{ uri: paletteUri, source: palette('#FF23765E') }]);
        assert.equal(changed.error, null);
        assert.ok(changed.frame);
        assert.notEqual(changed.frame.image.png, initial.frame.image.png);
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

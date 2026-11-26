import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedPaletteAction, HostedPaletteCatalog, HostedPaletteReport } from './hostedPaletteProtocol';
import type { HostedPalettes } from './hostedPalettes';
import { promptPaletteAddition } from './paletteActions';
import { choose } from './studioContext';

export async function runHostedPaletteAction(palettes: HostedPalettes, frame: HostedAnimationFrame,
    command: HostedPaletteAction, current: () => HostedAnimationFrame): Promise<HostedPaletteCatalog | HostedPaletteReport | undefined> {
    const signal = () => AbortSignal.timeout(15_000);
    const latest = () => {
        const fresh = current();
        if (fresh.revision !== frame.revision) throw new Error('The App Preview source changed while choosing a palette action.');
        return fresh;
    };
    switch (command.action) {
        case 'catalog': return palettes.catalog(frame, signal());
        case 'read': return palettes.report(frame, command.target, signal());
        case 'locate': await palettes.locate(frame, command.address, command.range, signal()); return;
        case 'edit': await palettes.edit(frame, command.target, command.edit, true, signal()); return;
        case 'add': {
            const { palette } = await palettes.report(frame, command.target, signal());
            const edit = await promptPaletteAddition(palette, command.layer);
            if (edit) await palettes.edit(latest(), command.target, edit, true, signal());
            return;
        }
        case 'references': {
            const { references } = await palettes.references(frame, command.target, command.node, signal());
            const reference = await choose('Authored resource references', references.map(entry => ({
                ...entry, description: `${entry.property} · line ${entry.range.start.line + 1}`,
                detail: entry.dynamic ? 'Dynamic resource' : 'Resource reference',
            })));
            if (reference) await palettes.locate(latest(), command.target.address, reference.range, signal());
            return;
        }
    }
}

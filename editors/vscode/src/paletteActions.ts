import { choose, editText } from './studioContext';
import { contrastProviderLabels, type ContrastProvider, type PaletteColorKind, type PaletteEdit,
    type PaletteLayer, type PaletteSheet } from './stylesProtocol';

export async function promptPaletteAddition(palette: PaletteSheet, layer: PaletteLayer): Promise<PaletteEdit | undefined> {
    const target = palette.layers.find(entry => entry.layer === layer);
    if (!target || target.unavailable) throw new Error(target?.unavailable ?? 'Choose an authored palette layer.');
    let provider: ContrastProvider | undefined;
    if (layer === 'high_contrast' && target.declaration == null) {
        const providers = palette.contrastProviders ?? [];
        if (!providers.length) throw new Error('Load a theme package to create the High contrast layer.');
        const item = await choose('Choose the Application Theme Package',
            providers.map(provider => ({ label: contrastProviderLabels[provider], provider })));
        if (!item) return;
        provider = item.provider;
    }
    const key = await editText('Color resource key', undefined, value => value.trim() ? undefined : 'Enter a resource key.', 'Add Palette Color');
    if (key === undefined) return;
    const kind = await choose<{ label: string; type: PaletteColorKind }>('Choose a Color Resource Type', [
        { label: 'SolidColorBrush', type: 'solid_color_brush' },
        { label: 'Color', type: 'color' },
    ]);
    if (!kind) return;
    const color = await editText('FerroUI color value', '#FF4C82D0', undefined, 'Add Palette Color');
    if (color !== undefined) return { action: 'add', owner: palette.owner, layer, key: key.trim(), kind: kind.type, color, provider };
}

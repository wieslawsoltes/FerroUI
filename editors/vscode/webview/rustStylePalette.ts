import { paletteLayerLabels, type PaletteComparisonRow, type PaletteLayer, type RustColorResource, type StylesAction } from '../src/stylesProtocol';
import { create } from './dom';
import { paletteColorRow, paletteComparison } from './paletteEditor';
import { actionButton } from './propertyEditor';

export interface RustPaletteDisplay { layer?: PaletteLayer; compare: boolean }

export function rustStylePalette<Base extends string | number>(resources: readonly RustColorResource[],
    comparison: readonly PaletteComparisonRow[] | undefined, send: (action: StylesAction<Base>) => void, display: RustPaletteDisplay): HTMLElement {
    const section = create('section');
    section.setAttribute('aria-label', 'Rust color resources');
    const header = create('div');
    header.className = 'style-subheader';
    const caption = create('span');
    caption.textContent = 'Color resources';
    const tools = create('div');
    tools.className = 'style-tools';
    const layer = create('select');
    layer.id = 'rust-palette-layer';
    layer.setAttribute('aria-label', 'Rust color layer');
    for (const [key, label] of Object.entries(paletteLayerLabels)) layer.append(new Option(label, key));
    const selectedLayer = () => comparison ? display.layer ?? resources.find(entry => entry.layer !== null)?.layer ?? 'shared' : 'shared';
    layer.value = selectedLayer();
    layer.hidden = !comparison;
    layer.addEventListener('change', () => { display.layer = layer.value as PaletteLayer; display.compare = false; render(); });
    const compare = actionButton('split-horizontal', 'Compare Rust theme colors', () => { display.compare = !display.compare; render(); });
    compare.id = 'rust-palette-compare';
    compare.hidden = !comparison?.length;
    const add = actionButton('add', 'Add Rust color resource', () => send({ type: 'add-color-resource', layer: selectedLayer() }));
    tools.append(layer, compare, add);
    header.append(caption, tools);
    const content = create('div');
    section.append(header, content);
    render();
    return section;

    function render(): void {
        compare.setAttribute('aria-pressed', String(display.compare && Boolean(comparison?.length)));
        if (display.compare && comparison?.length) {
            content.replaceChildren(paletteComparison(comparison, ['light', 'dark', 'high_contrast'], '', color => {
                if (color.range) send({ type: 'reveal', range: color.range });
            }));
            return;
        }
        const selected = selectedLayer();
        const layerResources = resources.filter(entry => (entry.layer === undefined ? 'shared' : entry.layer) === selected);
        content.replaceChildren(...layerResources.map(entry => paletteColorRow({ id: `rust-color-${entry.node}`, key: entry.key,
            value: entry.color ?? 'Rust expression', color: entry.color ?? undefined, description: paletteLayerLabels[selected] }, {
            change: entry.color ? color => send({ type: 'rust-resource-color', node: entry.node, color }) : undefined,
            reveal: () => send({ type: 'reveal', range: entry.range }),
        })));
        if (!layerResources.length) {
            const empty = create('p');
            empty.className = 'style-empty';
            empty.textContent = 'No colors in this layer.';
            content.append(empty);
        }
    }
}

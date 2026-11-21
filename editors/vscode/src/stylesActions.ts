import type { StudioContext } from './studioContext';
import { promptPaletteAddition } from './paletteActions';
import { chooseNewSetter, chooseNewStyle, chooseStyleResource } from './stylePrompts';
import type { PaletteLayer, PaletteSheet, StyleCatalog, StyleDefinition } from './stylesProtocol';

export async function createStyle(context: StudioContext, catalog: StyleCatalog, kind: 'style' | 'theme'): Promise<string | undefined> {
    const result = await chooseNewStyle(catalog, kind, context.panel.selectedNode?.label);
    if (!result) return;
    const { node: owner, edit } = result;
    if (edit.action === 'create') {
        await context.edit('designer_style_edit', { owner, ...edit });
        return edit.selector.trim();
    }
    await context.edit('designer_component', { owner, ...edit });
    return edit.key;
}

export async function addStyleSetter(context: StudioContext, definition: StyleDefinition): Promise<void> {
    const setter = await chooseNewSetter(definition.available);
    if (setter?.kind === 'resource') await assignStyleResource(context, definition.node, setter.property);
    if (setter?.kind === 'literal') {
        await context.edit('designer_style_edit', { action: 'setter', owner: definition.node, property: setter.property, value: setter.value });
    }
}

export async function assignStyleResource(context: StudioContext, node: number, property: string): Promise<void> {
    const { resources } = await context.read<{ resources: { node: number; key: string; type_name: string }[] }>(
        'designer_style_resources', { owner: node, property });
    const selected = await chooseStyleResource(property, resources);
    if (selected) await context.edit('designer_style_edit', { action: 'resource', owner: node, property, resource: selected.resource.node, dynamic: selected.dynamic });
}

export async function addPaletteColor(context: StudioContext, palette: PaletteSheet, layer: PaletteLayer): Promise<void> {
    const edit = await promptPaletteAddition(palette, layer);
    if (edit) await context.edit('designer_palette_edit', edit);
}

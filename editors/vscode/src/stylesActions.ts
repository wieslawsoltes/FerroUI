import type { StudioContext } from './studioContext';
import { choose, editText } from './studioContext';
import { promptPaletteAddition } from './paletteActions';
import type { PaletteLayer, PaletteSheet, StyleCatalog, StyleDefinition, StyleSetter } from './stylesProtocol';

export async function createStyle(context: StudioContext, catalog: StyleCatalog, kind: 'style' | 'theme'): Promise<string | undefined> {
    const scopes = kind === 'theme' ? catalog.palette_scopes : catalog.style_hosts;
    const scope = await choose(kind === 'style' ? 'Choose a Style Host' : 'Choose a Resource Scope',
        scopes.map(scope => ({ label: scope.label, description: 'type_name' in scope ? String(scope.type_name) : undefined, scope })));
    if (!scope) return;
    if (kind === 'style') {
        const selector = await editText('Selector', 'nested' in scope.scope && scope.scope.nested ? '^:pointerover' : context.panel.selectedNode?.label,
            value => value.trim() ? undefined : 'Enter a style selector.', 'New FerroUI Style');
        if (selector === undefined) return;
        await context.edit('designer_style_edit', { action: 'create', owner: scope.scope.node, selector });
        return selector.trim();
    }
    const key = await editText('Control theme resource key', undefined, value => value.trim() ? undefined : 'Enter a resource key.', 'New FerroUI Control Theme');
    if (key === undefined) return;
    const targetType = await editText('Control target type', context.panel.selectedNode?.label ?? 'Button', value => value.trim() ? undefined : 'Enter a XAML type.', 'New FerroUI Control Theme');
    if (targetType === undefined) return;
    await context.edit('designer_component', { action: 'create_theme', owner: scope.scope.node, key: key.trim(), target_type: targetType.trim() });
    return key.trim();
}

export async function addStyleSetter(context: StudioContext, definition: StyleDefinition): Promise<void> {
    const item = await choose('Choose a Setter Property', definition.available.map(setter => ({
        label: setter.property, description: setter.field.type_name, setter,
    })));
    if (!item) return;
    const kinds = [{ label: 'Literal value', resource: false }, { label: 'Resource reference', resource: true }];
    const mode = await choose(item.setter.property, kinds);
    if (!mode) return;
    if (mode.resource) {
        await assignStyleResource(context, definition.node, item.setter.property);
        return;
    }
    const value = await setterValue(item.setter);
    if (value !== undefined) await context.edit('designer_style_edit', { action: 'setter', owner: definition.node, property: item.setter.property, value });
}

async function setterValue(setter: StyleSetter): Promise<string | undefined> {
    const { property, field } = setter;
    if (field.options.length) return (await choose(property, field.options.map(label => ({ label }))))?.label;
    return editText(`${property} value`, field.kind === 'color' ? '#FF4C82D0' : undefined, undefined, 'New FerroUI Setter');
}

export async function assignStyleResource(context: StudioContext, node: number, property: string): Promise<void> {
    const { resources } = await context.read<{ resources: { node: number; key: string; type_name: string }[] }>(
        'designer_style_resources', { owner: node, property });
    if (!resources.length) throw new Error('This style scope has no compatible resource for that property.');
    const item = await choose(`Choose a Resource for ${property}`, resources.map(resource => ({
        label: resource.key, description: resource.type_name.replace(/^\{.*?\}/, ''), resource,
    })));
    if (!item) return;
    const mode = await choose('Reference Mode', [
        { label: 'Dynamic Resource', description: 'Resolve against the live theme and resource scope', dynamic: true },
        { label: 'Static Resource', description: 'Resolve when the definition loads', dynamic: false },
    ]);
    if (mode) await context.edit('designer_style_edit', { action: 'resource', owner: node, property, resource: item.resource.node, dynamic: mode.dynamic });
}

export async function addPaletteColor(context: StudioContext, palette: PaletteSheet, layer: PaletteLayer): Promise<void> {
    const edit = await promptPaletteAddition(palette, layer);
    if (edit) await context.edit('designer_palette_edit', edit);
}

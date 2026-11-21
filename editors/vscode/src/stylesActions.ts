import type { StudioContext } from './studioContext';
import { choose, editText } from './studioContext';
import { contrastProviderLabels, type ContrastProvider, type PaletteColorKind, type PaletteLayer, type PaletteSheet,
    type StyleCatalog, type StyleDefinition, type StyleSetter } from './stylesProtocol';

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
    if (color !== undefined) await context.edit('designer_palette_edit',
        { action: 'add', owner: palette.owner, layer, key: key.trim(), kind: kind.type, color, provider });
}

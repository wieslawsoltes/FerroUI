import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';

import { componentActions, componentKind, findDefinition, readLibrary } from './studioLibrary';
interface DesignValue { name: string; value: string; object: boolean }

export function contentSections(context: StudioContext): StudioItem[] {
    return [
        { id: 'library', label: 'Library', icon: 'library', children: () => library(context) },
        { id: 'data', label: 'Design Data', icon: 'database', children: () => data(context) },
    ];
}

async function library(context: StudioContext): Promise<StudioItem[]> {
    const { components } = await readLibrary(context);
    const items: StudioItem[] = components.map(component => ({
        id: `component:${component.node}`,
        label: component.display_name,
        description: component.usages ? `${componentKind(component.kind)} · ${component.usages} uses` : componentKind(component.kind),
        run: async () => { await (await choose(component.display_name, componentActions(context, component)))?.run?.(); },
    }));
    const owner = context.node;
    if (components.length) items.unshift(action('component:find', 'Find Definition…', () => findDefinition(context, components), 'search'));
    items.unshift(action('component:exit', 'Return to document preview', () => context.present('designer_definition', { action: 'exit' }), 'preview'));
    if (context.writable && owner !== undefined) items.unshift(action('component:create', 'Add resource…', async () => {
        const key = await editText('Resource key');
        if (!key?.trim()) return;
        const source = await editText('Resource XAML', '<SolidColorBrush Color="#675DF5"/>');
        if (source) await context.edit('designer_component', { action: 'create_resource', owner, key, source });
    }, 'add'));
    return items.length ? items : [{ id: 'component:empty', label: 'No definitions in this design' }];
}

async function data(context: StudioContext): Promise<StudioItem[]> {
    const node = context.node;
    if (node === undefined) return [];
    const result = await context.read<{ properties: DesignValue[] }>('designer_design_data', { node });
    const rows: StudioItem[] = result.properties.map((property, index) => ({
        id: `data:${index}`,
        label: property.name,
        description: property.object ? 'XAML object' : property.value,
        run: context.writable ? () => editDesignValue(context, node, property) : undefined,
    }));
    if (context.writable) rows.unshift(
        action('data:add', 'Set design value…', async () => {
            const property = await editText('Property name, for example Text or DataContext');
            if (property) await editDesignValue(context, node, { name: property, value: '', object: false });
        }, 'add'),
        action('data:sample', 'Set sample item count…', async () => {
            const value = await editText('Sample items from 0 to 200. Leave empty to clear.');
            if (value === undefined) return;
            const count = value.trim() ? Number(value) : null;
            if (count !== null && (!Number.isInteger(count) || count < 0 || count > 200)) throw new Error('Enter a whole number from 0 to 200.');
            await context.edit('designer_design_data_edit', { action: 'sample_items', node, count });
        }, 'list-ordered'),
    );
    rows.push(action('data:toggle', 'Toggle design-time values…', async () => {
        const selected = await choose('Design-time data', [{ label: 'Use design values', enabled: true }, { label: 'Use application values', enabled: false }]);
        if (selected) await context.present('designer_design_data_edit', { action: 'enabled', enabled: selected.enabled });
    }, 'eye'));
    return rows;
}

async function editDesignValue(context: StudioContext, node: number, property: DesignValue): Promise<void> {
    const form = await choose(`Design value · ${property.name}`, [{ label: 'Scalar', value: 'scalar' }, { label: 'XAML object', value: 'object' }, { label: 'Clear', value: 'clear' }]);
    if (!form) return;
    const object = form.value === 'object' || (form.value === 'clear' && property.object);
    const field = object ? 'source' : 'value';
    const value = form.value === 'clear' ? null : await editText(object ? 'Design-only XAML object' : 'Design-only value',
        property.object === object ? property.value : undefined);
    if (value !== undefined) await context.edit('designer_design_data_edit', {
        action: object ? 'set_object' : 'set_value', node, property: property.name, [field]: value,
    });
}

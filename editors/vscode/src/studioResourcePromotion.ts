import { choose, editText, type StudioContext } from './studioContext';

interface ResourceScope {
    id: number;
    label: string;
    name: string | null;
    root: boolean;
}

export async function promoteResource(context: StudioContext, node: number, property: string): Promise<void> {
    if (!context.writable) throw new Error('Open a XAML control to promote a property to a resource.');
    const { owners } = await context.read<{ owners: ResourceScope[] }>('designer_component_promotion', { node, property });
    const owner = await choose(`Promote ${property} · Resource scope`, owners.map(scope => ({
        label: scope.name ? `${scope.label} · ${scope.name}` : scope.label,
        description: scope.root ? 'Document resources' : 'Element resources',
        id: scope.id,
    })));
    if (!owner) return;
    context.ensure();
    const key = await editText('Key for the new resource', property);
    if (key === undefined) return;
    await context.edit('designer_component', { action: 'extract_property', node, property, owner: owner.id, key: key.trim() });
}

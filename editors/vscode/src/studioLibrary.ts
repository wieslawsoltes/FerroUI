import * as vscode from 'vscode';
import { action, choose, editText, type StudioContext, type StudioItem } from './studioContext';

export interface Component {
    node: number;
    display_name: string;
    kind: string;
    key: string | null;
    target_type: string | null;
    usages: number;
}

export interface ComponentLibrary {
    components: Component[];
    scope: { definition: { node: number; title: string } | null };
}

export function readLibrary(context: StudioContext): Promise<ComponentLibrary> {
    return context.read<ComponentLibrary>('designer_components');
}

export function componentKind(kind: string): string {
    return kind.replace(/([a-z])([A-Z])/g, '$1 $2');
}

export function componentActions(context: StudioContext, component: Component, editStyle?: () => Promise<void>): StudioItem[] {
    const { node, key } = component;
    const id = `definition:${node}`;
    const items = [
        action(`${id}:preview`, 'Preview definition', () => previewDefinition(context, node), 'preview'),
        action(`${id}:reveal`, 'Reveal definition in source', () => context.reveal(node), 'go-to-file'),
    ];
    if (component.usages) items.push(action(`${id}:references`, 'Find authored references…',
        () => showResourceReferences(context, node), 'references'));
    if (editStyle && context.panel.supportsStyleComposition && ['Style', 'ControlTheme'].includes(component.kind)) {
        items.unshift(action(`${id}:style`, 'Edit Style or Theme', editStyle, 'paintcan'));
    }
    if (context.writable && key) items.push(
        action(`${id}:rename`, 'Rename resource and references…', async () => {
            context.ensure();
            const document = context.panel.document;
            if (vscode.workspace.isTrusted && vscode.workspace.getWorkspaceFolder(document.uri)) {
                const target = context.panel.nodes.find(entry => entry.id === node);
                if (!target) throw new Error('The declaration is unavailable in the current design.');
                await vscode.commands.executeCommand('ferroui.renameResource', {
                    uri: document.uri.toString(),
                    request: { version: document.version, position: target.nameRange.start },
                });
                return;
            }
            const next = await editText('New resource key', key);
            if (next?.trim() && next !== key) await context.edit('designer_component', { action: 'rename_resource', resource: node, key: next });
        }, 'rename'),
        action(`${id}:duplicate`, 'Duplicate resource…', async () => {
            const next = await editText('Key for the new definition', `${key}Copy`);
            if (next?.trim()) await context.edit('designer_component', { action: 'duplicate_resource', resource: node, key: next });
        }, 'copy'),
    );
    return items;
}

export async function previewDefinition(context: StudioContext, node: number): Promise<void> {
    await context.present('designer_definition', { action: 'enter', node });
    if (!context.panel.isDisposed) context.panel.panel.reveal(undefined, true);
}

export async function findDefinition(context: StudioContext, components?: Component[]): Promise<void> {
    const available = components ?? (await readLibrary(context)).components;
    if (!available.length) throw new Error('This design has no authored definitions.');
    const selected = await choose('Find FerroUI Definition', available.map(component => ({
        label: component.display_name, description: [componentKind(component.kind), component.key, component.target_type].filter(Boolean).join(' · '), component,
    })));
    if (selected) await previewDefinition(context, selected.component.node);
}

export async function createTemplate(context: StudioContext, owner: number): Promise<void> {
    const kind = await choose('New FerroUI Template', [
        { label: 'Data Template', data: true, property: 'DataType' },
        { label: 'Control Template', data: false, property: 'TargetType' },
    ]);
    if (!kind) return;
    const key = await editText('Template resource key');
    if (!key?.trim()) return;
    const target = await editText(`${kind.property} (optional)`, kind.data ? '' : 'ContentControl');
    if (target === undefined) return;
    const content = await editText('Template content XAML', kind.data ? '<TextBlock Text="{Binding}"/>' : '<ContentPresenter/>');
    if (!content?.trim()) return;
    await context.edit('designer_component', { action: 'create_template', owner, key, target_type: target.trim() || null,
        content, data_template: kind.data });
}

export async function showResourceReferences(context: StudioContext, resource: number): Promise<void> {
    const { references: entries } = await context.read<{ references: { node: number; property: string; dynamic: boolean }[] }>(
        'designer_component_references', { resource });
    const nodes = new Map(context.panel.nodes.map(node => [node.id, node]));
    const target = await choose('Authored resource references', entries.flatMap(entry => {
        const node = nodes.get(entry.node);
        if (!node) return [];
        return [{ label: node.name ?? node.label, description: `${entry.property} · line ${node.nameRange.start.line + 1}`,
            detail: entry.dynamic ? 'Dynamic resource' : 'Resource reference', node: node.id }];
    }));
    if (target) await context.reveal(target.node);
}

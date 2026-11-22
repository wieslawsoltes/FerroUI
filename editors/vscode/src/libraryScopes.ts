import type { Range } from './protocol';
import { action, type StudioContext, type StudioItem } from './studioContext';
import { componentActions, type Component } from './studioLibrary';

interface ScopeObject {
    id: string;
    node: number | null;
    title: string;
    kind: string;
    detail: string | null;
    definition: number | null;
    range: Range;
    sourceUri: string | null;
}

interface LibraryScope {
    owner: number;
    title: string;
    path: string;
    ownerRange: Range | null;
    objects: ScopeObject[];
}

export interface LibraryScopes {
    uri: string;
    revision: number;
    scopeEpoch: number;
    scopes: LibraryScope[];
}

const icons: Record<string, string> = {
    Behavior: 'symbol-event',
    Include: 'file-symlink-file',
    Resource: 'symbol-property',
    'Resource dictionary': 'folder-library',
    'Control theme': 'color-mode',
    Style: 'paintcan',
    'Control template': 'symbol-structure',
    'Data template': 'symbol-interface',
    Template: 'symbol-interface',
};

export async function scopedLibraryItems(
    context: StudioContext, components: readonly Component[], editStyle: (node: number) => Promise<void>,
): Promise<StudioItem[]> {
    const { scopes } = await context.read<LibraryScopes>('designer_library_scopes');
    if (!scopes.length) return [{ id: 'scopes:empty', label: 'No authored objects in the current scope' }];
    const definitions = new Map(components.map(component => [component.node, component]));
    const reveal = (range: Range, uri?: string | null) => context.panel.language === 'rust'
        ? context.revealRustRange(range, uri ?? undefined)
        : context.revealRange(range);
    return scopes.map(scope => {
        const id = `scope:${scope.owner}`;
        const parent = scope.path.slice(0, -scope.title.length).replace(/ › $/, '');
        const line = scope.ownerRange ? `line ${scope.ownerRange.start.line + 1}` : undefined;
        return {
            id, label: scope.title, icon: 'folder-opened',
            description: [parent, line].filter(Boolean).join(' · '),
            tooltip: [scope.path, line].filter(Boolean).join(' · '),
            children: async () => {
                context.ensure();
                const rows: StudioItem[] = scope.objects.map(object => {
                    const key = `${id}:${object.id}`;
                    const definition = object.definition !== null ? definitions.get(object.definition) : undefined;
                    const open = () => reveal(object.range, object.sourceUri);
                    return {
                        id: key, label: object.title, icon: icons[object.kind],
                        description: [object.kind, object.detail].filter(Boolean).join(' · '),
                        run: open,
                        children: definition ? async () => [
                            action(`${key}:reveal`, 'Reveal definition in source', open, 'go-to-file'),
                            ...componentActions(context, definition, () => editStyle(definition.node))
                                .filter(item => !item.id.endsWith(':reveal'))
                                .map(item => ({ ...item, id: `${key}:${item.id}` })),
                        ] : undefined,
                    };
                });
                const owner = scope.ownerRange;
                if (owner) rows.push(action(`${id}:source`, 'Reveal scope owner', () => reveal(owner), 'go-to-file'));
                return rows;
            },
        };
    });
}

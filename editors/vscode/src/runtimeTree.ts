import type { RuntimeNode, RuntimeTreeDisclosure } from './runtimeProtocol';

export const openRuntimeTree: RuntimeTreeDisclosure = { collapsed: [] };

export function runtimeNodeLabel(node: RuntimeNode): string {
    const label = node.type || node.role || `Node ${node.node_id}`;
    const name = node.name || node.text;
    return name ? `${label} · ${name}` : label;
}

export function* walkRuntimeNodes(roots: readonly RuntimeNode[]): Generator<{ node: RuntimeNode; parent: number | null }> {
    const seen = new Set<number>();
    function* visit(nodes: readonly RuntimeNode[], parent: number | null): Generator<{ node: RuntimeNode; parent: number | null }> {
        for (const node of nodes) {
            // The inspector can report a truncated reference when a node already occurs in the snapshot.
            if (seen.has(node.node_id)) continue;
            seen.add(node.node_id);
            yield { node, parent };
            if (node.children?.length) yield* visit(node.children, node.node_id);
        }
    }
    yield* visit(roots, null);
}

export function runtimeNodePath(roots: readonly RuntimeNode[], id: number): RuntimeNode[] | undefined {
    for (const node of roots) {
        if (node.node_id === id) return [node];
        const path = node.children && runtimeNodePath(node.children, id);
        if (path) return [node, ...path];
    }
}

export function collapsedRuntimeNodes(disclosure: RuntimeTreeDisclosure, filter: string): readonly number[] {
    const query = filter.trim().toLowerCase();
    if (!query) return disclosure.collapsed;
    return disclosure.search?.query === query ? disclosure.search.collapsed : openRuntimeTree.collapsed;
}

export function setRuntimeBranch(disclosure: RuntimeTreeDisclosure, filter: string, node: number, expanded: boolean): RuntimeTreeDisclosure {
    const closed = collapsedRuntimeNodes(disclosure, filter);
    const isExpanded = !closed.includes(node);
    if (expanded === isExpanded) return disclosure;
    return withCollapsed(disclosure, filter, expanded ? closed.filter(id => id !== node) : [...closed, node]);
}

export function revealRuntimeBranch(disclosure: RuntimeTreeDisclosure, filter: string, roots: readonly RuntimeNode[], node: number): RuntimeTreeDisclosure {
    const closed = collapsedRuntimeNodes(disclosure, filter);
    if (!closed.length) return disclosure;
    const path = runtimeNodePath(roots, node);
    if (!path) return disclosure;
    const ancestors = new Set(path.slice(0, -1).map(item => item.node_id));
    const next = closed.filter(id => !ancestors.has(id));
    return next.length === closed.length ? disclosure : withCollapsed(disclosure, filter, next);
}

export function retainRuntimeBranches(disclosure: RuntimeTreeDisclosure, roots: readonly RuntimeNode[]): RuntimeTreeDisclosure {
    const { collapsed, search } = disclosure;
    if (!collapsed.length && !search?.collapsed.length) return disclosure;
    const branches = new Set<number>();
    for (const { node } of walkRuntimeNodes(roots)) {
        if (node.children?.length) branches.add(node.node_id);
    }
    const remaining = collapsed.filter(id => branches.has(id));
    const matching = search?.collapsed.filter(id => branches.has(id));
    if (remaining.length === collapsed.length && matching?.length === search?.collapsed.length) return disclosure;
    return { collapsed: remaining, ...(search && matching ? { search: { query: search.query, collapsed: matching } } : {}) };
}

function withCollapsed(disclosure: RuntimeTreeDisclosure, filter: string, collapsed: readonly number[]): RuntimeTreeDisclosure {
    const query = filter.trim().toLowerCase();
    return query ? { ...disclosure, search: { query, collapsed } } : { ...disclosure, collapsed };
}

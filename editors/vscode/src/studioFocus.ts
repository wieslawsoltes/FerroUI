import { choose, type StudioContext, type StudioItem } from './studioContext';

interface FocusCandidate { node: number; label: string }
interface FocusItem { node: number | null; label: string; tab_index: number; is_tab_stop: boolean; active: boolean }
interface FocusScope { id: number; label: string; mode: string; can_reorder: boolean; items: FocusItem[] }
interface FocusLabel extends FocusCandidate { target: number | null; candidates: FocusCandidate[] }

export async function focusItems(context: StudioContext): Promise<StudioItem[]> {
    const { model, catalog } = await context.read<{ model: string; catalog: { scopes: FocusScope[]; labels: FocusLabel[] } }>('designer_focus_order');
    const items = catalog.scopes.map(scope => ({
        id: `focus:scope:${scope.id}`,
        label: scope.label || 'Focus scope',
        description: scope.mode,
        children: async () => scope.items.map((item, index) => ({
            id: `focus:scope:${scope.id}:${index}`, label: item.label,
            description: item.active ? `Tab ${item.tab_index}` : 'Outside the focus order',
            run: item.node === null ? undefined : () => focusAction(context, model, scope, item.node!, item.is_tab_stop),
        })),
    })) as StudioItem[];
    for (const label of catalog.labels) items.push({
        id: `focus:label:${label.node}`,
        label: label.label,
        description: label.candidates.find(candidate => candidate.node === label.target)?.label ?? 'Choose a label target',
        run: context.writable ? async () => {
            const target = await choose('Label target', [
                { label: 'Clear label target', node: null }, ...label.candidates,
            ]);
            if (target) await context.edit('designer_focus_edit', { action: 'label_target', label: label.node, target: target.node, expected_model: model });
        } : undefined,
    });
    return items;
}

async function focusAction(context: StudioContext, model: string, scope: FocusScope, node: number, isTabStop: boolean): Promise<void> {
    if (!context.writable) return context.reveal(node);
    const order = scope.items.filter(item => item.active && item.node !== null).map(item => item.node!);
    const index = order.indexOf(node);
    const update = (args: object) => context.edit('designer_focus_edit', { ...args, expected_model: model });
    const items = [
        { label: 'Reveal in source', run: () => context.reveal(node) },
        { label: isTabStop ? 'Remove tab stop' : 'Enable tab stop', run: () => update({ action: 'tab_stop', node, value: !isTabStop }) },
    ];
    const move = (delta: number) => {
        const nodes = order.slice();
        [nodes[index], nodes[index + delta]] = [nodes[index + delta], nodes[index]];
        return update({ action: 'order', scope: scope.id, nodes });
    };
    if (scope.can_reorder && index > 0) items.push({ label: 'Move earlier', run: () => move(-1) });
    if (scope.can_reorder && index >= 0 && index < order.length - 1) items.push({ label: 'Move later', run: () => move(1) });
    await (await choose('Focus order', items))?.run();
}

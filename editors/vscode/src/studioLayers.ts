import { action, choose, type StudioContext, type StudioItem } from './studioContext';

interface Layer { node: number; locked: boolean; hidden: boolean; outside_isolation: boolean }

export async function layerItems(context: StudioContext): Promise<StudioItem[]> {
    const model = await context.read<{ layer_epoch: number; scope_epoch: number; layers: Layer[] }>('designer_layers');
    const nodes = context.selection;
    const compose = (action: string, args: object = {}) => context.present('designer_layer', {
        action, ...args, expected_layer_epoch: model.layer_epoch, expected_scope_epoch: model.scope_epoch,
    });
    const rows = [action('layers:reset', 'Reset layers', () => compose('reset'), 'discard')];
    if (model.layers.some(layer => layer.outside_isolation)) rows.unshift(action('layers:exit', 'Exit isolation', () => compose('exit_isolation'), 'screen-full'));
    if (!nodes.length) return rows;
    const group = model.layers.filter(layer => nodes.includes(layer.node));
    const hidden = group.every(layer => layer.hidden);
    const locked = group.every(layer => layer.locked);
    rows.unshift(action('layers:visibility', hidden ? 'Show selection' : 'Hide selection', () => compose('set_hidden', { nodes, hidden: !hidden }), 'eye'),
        action('layers:lock', locked ? 'Unlock selection' : 'Lock selection', () => compose('set_locked', { nodes, locked: !locked }), 'lock'),
        action('layers:isolate', 'Isolate selection', () => compose('isolate', { nodes }), 'screen-normal'),
        action('layers:select', 'Select related controls…', async () => {
            const choice = await choose('Select controls', [
                { label: 'Parent', action: 'select_parent' }, { label: 'Children', action: 'select_children' },
                { label: 'Previous sibling', action: 'select_previous' }, { label: 'Next sibling', action: 'select_next' },
                { label: 'Matching control type', action: 'select_matching_type' },
            ]);
            if (choice) await compose(choice.action);
        }, 'list-selection'));
    return rows;
}

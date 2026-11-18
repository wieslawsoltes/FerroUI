import type { Operation, Placement } from './protocol';

export function recordedMove({ origin, target }: Placement) {
    if (target.type !== 'canvas' || (origin.type !== 'node' && origin.type !== 'nodes')) return;
    return {
        action: 'move' as const, x: target.x, y: target.y, grab: origin.grab,
        nodes: origin.type === 'node' ? [origin.node] : origin.nodes,
        anchor: origin.type === 'node' ? origin.node : origin.anchor,
    };
}

export function recordedGeometry(operation: Operation) {
    if (operation.type === 'place') return recordedMove(operation.placement);
    if (operation.type === 'resize') {
        const { node, handle, dx, dy } = operation;
        return { action: 'resize' as const, node, handle, dx, dy };
    }
}

import { errorMessage, type DragGesture, type DragMessage, type DragOrigin, type DragUpdate, type FromDesigner } from './protocol';
import type { DesignerPanel } from './designerPanel';

export type DragRequest = DragMessage | Extract<FromDesigner, { type: 'previewDrop' }>;

export class DesignerDrag {
    private active?: DragGesture;
    private ending?: ReturnType<typeof setTimeout>;
    private expiry?: ReturnType<typeof setTimeout>;

    constructor(
        private readonly publish: (update: DragUpdate) => void,
        private readonly ownsImage: (origin: Extract<DragOrigin, { type: 'image' }>) => boolean,
        private readonly authorizeImage: (panel: DesignerPanel, origin: Extract<DragOrigin, { type: 'image' }>) => Promise<void>,
    ) {}

    async receive(panel: DesignerPanel, message: DragRequest): Promise<void> {
        if (message.type === 'beginDrag') {
            const gesture = message.gesture;
            if (!gesture || !/^[a-f0-9-]{36}$/.test(gesture.id) || !panel.canUse(gesture.frame)) return;
            const origin = gesture.origin;
            const movable = (id: number) => panel.nodes.some(node => node.id === id && node.canMove);
            if (!origin || !(origin.type === 'asset' ? panel.assets.some(asset => asset.id === origin.control)
                : origin.type === 'image' ? this.ownsImage(origin)
                    : origin.type === 'node' ? movable(origin.node) : origin.type === 'nodes'
                        && Array.isArray(origin.nodes) && origin.nodes.length > 1 && origin.nodes.length <= 128
                        && origin.nodes.includes(origin.anchor) && origin.nodes.every(movable))) return;
            clearTimeout(this.ending);
            clearTimeout(this.expiry);
            this.ending = undefined;
            this.active = gesture;
            this.expiry = setTimeout(() => this.clear(), 25_000);
            this.publish({ type: 'drag', gesture });
            return;
        }
        const gesture = this.active;
        if (!gesture || gesture.id !== message.id || gesture.frame.session !== panel.session) return;
        if (message.type === 'endDrag') {
            // Drop and dragend can arrive on different webview message queues.
            this.ending ??= setTimeout(() => this.clear(), 250);
            return;
        }
        const valid = panel.canUse(message.frame) && message.frame.token === gesture.frame.token;
        const placement = { origin: gesture.origin, target: message.target };
        if (message.type === 'drop') {
            this.clear();
            if (!valid) throw new Error('The design changed during the drag. Use the current view to retry.');
            if (gesture.origin.type === 'image') await this.authorizeImage(panel, gesture.origin);
            await panel.place(message.frame, placement);
        } else if (valid) {
            const response = { type: 'placement', id: gesture.id, sequence: message.sequence } as const;
            try {
                const hint = await panel.previewPlacement(message.frame, placement);
                if (this.active === gesture) panel.post({ ...response, hint });
            } catch (error) {
                if (this.active === gesture) panel.post({ ...response, error: errorMessage(error) });
            }
        }
    }

    clear(): void {
        clearTimeout(this.ending);
        clearTimeout(this.expiry);
        this.ending = this.expiry = undefined;
        if (!this.active) return;
        this.active = undefined;
        this.publish({ type: 'drag' });
    }
}

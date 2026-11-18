import { errorMessage, type DragGesture, type DragMessage, type DragUpdate, type FromDesigner } from './protocol';
import type { DesignerPanel } from './designerPanel';

export type DragRequest = DragMessage | Extract<FromDesigner, { type: 'previewDrop' }>;

export class DesignerDrag {
    private active?: DragGesture;
    private ending?: ReturnType<typeof setTimeout>;

    constructor(private readonly publish: (update: DragUpdate) => void) {}

    async receive(panel: DesignerPanel, message: DragRequest): Promise<void> {
        if (message.type === 'beginDrag') {
            const gesture = message.gesture;
            if (!gesture || !/^[a-f0-9-]{36}$/.test(gesture.id) || !panel.canUse(gesture.frame)) return;
            const origin = gesture.origin;
            if (!origin || !(origin.type === 'asset' ? panel.assets.some(asset => asset.id === origin.control)
                : origin.type === 'node' && panel.nodes.some(node => node.id === origin.node && node.canMove))) return;
            this.clear();
            this.active = gesture;
            this.publish({ type: 'drag', gesture });
            return;
        }
        const gesture = this.active;
        if (!gesture || gesture.id !== message.id || gesture.frame.session !== panel.session) return;
        if (message.type === 'endDrag') {
            // Drop and dragend can arrive on different webview message queues.
            clearTimeout(this.ending);
            this.ending = setTimeout(() => this.clear(), 250);
            return;
        }
        const valid = panel.canUse(message.frame) && message.frame.token === gesture.frame.token;
        const placement = { origin: gesture.origin, target: message.target };
        if (message.type === 'drop') {
            this.clear();
            if (!valid) throw new Error('The design changed during the drag. Use the current view to retry.');
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
        if (!this.active) return;
        this.active = undefined;
        this.publish({ type: 'drag' });
    }
}

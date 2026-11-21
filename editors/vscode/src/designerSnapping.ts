import { snapEnabled, type CanvasGuidesState, type CanvasSnapAdjustment, type CanvasSnapGesture, type CanvasSnapInput } from './canvasGuidesProtocol';
import type { NativePreview } from './nativePreview';
import type { FrameTarget, Placement, Point, ResizeGesture } from './protocol';

export class DesignerSnapping {
    constructor(private readonly state: () => CanvasGuidesState | undefined,
        private readonly current: (frame: FrameTarget) => boolean, private readonly client: () => Promise<NativePreview>) {}

    validate(frame: FrameTarget, input: CanvasSnapInput): CanvasGuidesState {
        const state = this.state();
        if (!this.current(frame) || !state?.snapping || input.epoch !== state.epoch) {
            throw new Error('The design or snapping settings changed. Start the gesture again.');
        }
        if (!Number.isFinite(input.zoom) || input.zoom < 0.01 || input.zoom > 8) throw new Error('Choose a canvas zoom between 1% and 800%.');
        return state;
    }

    async resolve(frame: FrameTarget, input: CanvasSnapInput, gesture: CanvasSnapGesture): Promise<CanvasSnapAdjustment> {
        const { settings } = this.validate(frame, input);
        if (input.bypass || !snapEnabled(settings)) return { delta: { x: 0, y: 0 }, guides: [] };
        const client = await this.client();
        this.validate(frame, input);
        const adjustment = await client.canvasSnap(frame.token, settings, input.zoom, gesture);
        this.validate(frame, input);
        return adjustment;
    }
}

export function snappedPlacement(placement: Placement, delta: Point): Placement {
    const target = placement.target;
    if (target.type !== 'canvas') return placement;
    return { ...placement, target: { type: 'canvas', x: target.x + delta.x, y: target.y + delta.y } };
}

export function snappedResize(gesture: ResizeGesture, delta: Point): ResizeGesture {
    const { node, handle, dx, dy } = gesture;
    return { node, handle, dx: dx + delta.x, dy: dy + delta.y };
}

import type { CanvasSnapInput, SnapLine } from '../src/canvasGuidesProtocol';
import type { FrameTarget } from '../src/protocol';
import { create, element } from './dom';

export interface SnapCapture extends CanvasSnapInput { active: boolean; frame: FrameTarget }

export function sameSnap(a?: SnapCapture, b?: SnapCapture): boolean {
    return a?.epoch === b?.epoch && a?.zoom === b?.zoom && a?.frame.token === b?.frame.token && a?.frame.session === b?.frame.session;
}

export function snapInput(capture: SnapCapture | undefined, bypass: boolean): CanvasSnapInput | undefined {
    if (capture) return { epoch: capture.epoch, zoom: capture.zoom, bypass };
}

export class SnapFeedback {
    private readonly grid = element('canvas-grid');
    private readonly lines = element('canvas-snap-lines');

    showGrid(spacing?: number): void {
        this.grid.hidden = spacing === undefined || spacing <= 0;
        if (spacing && spacing > 0) {
            const stride = 2 ** Math.max(0, Math.ceil(Math.log2(5 / spacing)));
            this.grid.style.setProperty('--grid-step', `${spacing * stride}px`);
        }
    }

    showLines(lines: readonly SnapLine[], width: number, height: number): void {
        this.lines.replaceChildren(...lines.map(({ axis, position }) => {
            const guide = create('span');
            guide.className = 'snap-line';
            const x = axis === 'Horizontal';
            guide.dataset.axis = x ? 'x' : 'y';
            guide.style.setProperty('--snap-position', `${position * 100 / (x ? width : height)}%`);
            return guide;
        }));
    }
}

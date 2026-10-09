export class WebRenderTarget {
    renderTargetType: string;
    constructor(protected canvas: HTMLCanvasElement | OffscreenCanvas, type: string) {
        this.renderTargetType = type;
    }

    // Not in the original: lets go of what the target holds besides the canvas. Called by the
    // registry when the view of the canvas is closed, on the thread the target belongs to.
    public release(): void {
    }

    static setSize(target: WebRenderTarget, w: number, h: number) {
        target.canvas.width = w;
        target.canvas.height = h;
    }
}

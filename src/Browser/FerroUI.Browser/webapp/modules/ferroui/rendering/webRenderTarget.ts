export class WebRenderTarget {
    renderTargetType: string;
    // Not in the original. For the canvas of a render worker: the port its frames go to the page
    // by (`CanvasPresenter`); none for a canvas the page draws to itself.
    public presentPort?: MessagePort;

    constructor(protected canvas: HTMLCanvasElement | OffscreenCanvas, type: string) {
        this.renderTargetType = type;
    }

    // Not in the original: lets go of what the target holds besides the canvas. Called by the
    // registry when the view of the canvas is closed, on the thread the target belongs to.
    public release(): void {
        this.presentPort?.close();
        this.presentPort = undefined;
    }

    static setSize(target: WebRenderTarget, w: number, h: number) {
        target.canvas.width = w;
        target.canvas.height = h;
    }

    // Not in the original. Called by the thread that draws when it has drawn a frame of `w` by `h`
    // device pixels for the scaling `scaling`. A canvas of the page shows the frame by itself. The
    // canvas of a render worker hands it to the page, which shows it at that size
    // (`CanvasPresenter`).
    static present(target: WebRenderTarget, w: number, h: number, scaling: number) {
        const port = target.presentPort;
        if (port == null || !(target.canvas instanceof OffscreenCanvas)) { return; }
        const bitmap = target.canvas.transferToImageBitmap();
        port.postMessage({ bitmap, width: w, height: h, scaling }, [bitmap]);
    }
}

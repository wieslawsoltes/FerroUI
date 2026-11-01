export class WebRenderTarget {
    renderTargetType: string;
    // Not in the original. For the canvas of a render worker: the port to the page's
    // `CanvasPresenter`, and whether the page asks for copies of the frames (while the view is
    // resized).
    private presentPort?: MessagePort;
    private resizeFrames = false;

    constructor(protected canvas: HTMLCanvasElement | OffscreenCanvas, type: string) {
        this.renderTargetType = type;
    }

    // Not in the original. Connects the canvas of a render worker to the page's `CanvasPresenter`.
    public connectPresenter(port: MessagePort): void {
        this.presentPort = port;
        port.onmessage = ev => {
            if (ev.data?.ferrouiCmd === "resizeFrames") { this.resizeFrames = ev.data.active === true; }
        };
    }

    // Not in the original: lets go of what the target holds besides the canvas. Called by the
    // registry when the view of the canvas is closed, on the thread the target belongs to.
    public release(): void {
        if (this.presentPort != null) {
            this.presentPort.onmessage = null;
            this.presentPort.close();
            this.presentPort = undefined;
        }
    }

    static setSize(target: WebRenderTarget, w: number, h: number) {
        target.canvas.width = w;
        target.canvas.height = h;
    }

    // Not in the original. Called by the thread that draws when it has drawn a frame of `w` by `h`
    // device pixels for the scaling `scaling`. The frame is committed to the canvas at the end of
    // the task, as always. While the page resizes the view of a render worker's canvas, it also gets
    // a copy of the frame, which it shows at the frame's size (`CanvasPresenter`).
    static present(target: WebRenderTarget, w: number, h: number, scaling: number) {
        const port = target.presentPort;
        if (port == null || !target.resizeFrames || !(target.canvas instanceof OffscreenCanvas)) { return; }
        // The copy is taken now (the frame stays in the canvas and is committed with the task).
        createImageBitmap(target.canvas).then(
            bitmap => port.postMessage({ bitmap, width: w, height: h, scaling }, [bitmap]),
            () => { /* a frame that cannot be copied is not shown by the page; the canvas has it */ });
    }
}

// Not in the original. The cooperative resize of a canvas whose control went to a render worker
// (`transferControlToOffscreen`).
//
// The worker commits its frames to the canvas directly, and the browser shows them on the
// worker's schedule: nothing waits for the thread of the page, so what the render thread animates
// keeps moving while the page is busy. The size of the canvas element follows the layout of the
// page on the page's schedule, and no API orders the two: while the view is resized, the browser
// would show frames of the old size stretched to the new one.
//
// So for as long as the view is resized (and only then) the page shows the frames itself, each at
// its own size: when the resize starts, the current frame is copied into an overlay canvas of the
// frame's size (`drawImage` of the canvas, synchronous), which covers the canvas; the worker still
// commits each frame to its canvas and also hands the page a copy (`createImageBitmap`), which the
// page draws into the overlay, giving the overlay the size of that frame in the same task. When
// the size has not changed for two animation frames and the canvas has a frame of the size of the
// view, the overlay is removed: both show a frame of that size.
//
// The overlay is a 2D canvas: what the page draws into it is a copy on the GPU, made only while a
// resize lasts.

export interface PresentedFrame {
    bitmap: ImageBitmap;
    // The size of the frame in device pixels and the device pixel ratio it was rendered for.
    width: number;
    height: number;
    scaling: number;
}

export class CanvasPresenter {
    private overlay?: HTMLCanvasElement;
    private overlayContext?: CanvasRenderingContext2D;
    private quietFrames = 0;
    private checking = false;
    // The size of the view in device pixels, as the last resize reported it.
    private viewWidth = 0;
    private viewHeight = 0;

    constructor(private readonly canvas: HTMLCanvasElement, private readonly port: MessagePort) {
        port.onmessage = ev => this.present(ev.data as PresentedFrame);
    }

    // Whether the browser has what this needs: a worker copy of a frame (`createImageBitmap` of an
    // `OffscreenCanvas`) and a 2D context to show it. Without them the canvas is resized as the
    // original resizes it.
    public static isSupported(): boolean {
        return typeof OffscreenCanvas !== "undefined" && typeof createImageBitmap === "function";
    }

    // Called by the resize observer of the canvas, after the browser's layout and before its
    // paint, with the new size of the view in device pixels.
    public resized(width: number, height: number): void {
        this.viewWidth = width;
        this.viewHeight = height;
        this.quietFrames = 0;
        if (this.overlay == null) { this.begin(); }
        this.scheduleCheck();
    }

    private begin(): void {
        const canvas = this.canvas;
        if (canvas.width <= 1 || canvas.height <= 1) {
            // No frame yet: there is nothing to show stretched.
            return;
        }
        const overlay = document.createElement("canvas");
        overlay.classList.add("ferroui-resize-overlay");
        overlay.style.position = "absolute";
        overlay.style.left = "0px";
        overlay.style.top = "0px";
        overlay.style.pointerEvents = "none";
        const context = overlay.getContext("2d", { alpha: true, desynchronized: false });
        if (context == null) { return; }
        this.show(overlay, context, canvas, canvas.width, canvas.height, devicePixelRatio);
        canvas.after(overlay);
        canvas.style.visibility = "hidden";
        this.overlay = overlay;
        this.overlayContext = context;
        this.port.postMessage({ ferrouiCmd: "resizeFrames", active: true });
    }

    private present(frame: PresentedFrame): void {
        if (this.overlay == null || this.overlayContext == null) {
            frame.bitmap.close();
            return;
        }
        this.show(this.overlay, this.overlayContext, frame.bitmap, frame.width, frame.height, frame.scaling);
        frame.bitmap.close();
    }

    // Shows `source` in `overlay` at its own size: the size of the overlay and its picture change
    // in one task.
    private show(overlay: HTMLCanvasElement, context: CanvasRenderingContext2D, source: CanvasImageSource, width: number, height: number, scaling: number): void {
        if (overlay.width !== width) { overlay.width = width; }
        if (overlay.height !== height) { overlay.height = height; }
        overlay.style.width = `${width / scaling}px`;
        overlay.style.height = `${height / scaling}px`;
        context.globalCompositeOperation = "copy";
        context.drawImage(source, 0, 0);
    }

    private scheduleCheck(): void {
        if (this.checking) { return; }
        this.checking = true;
        requestAnimationFrame(() => {
            this.checking = false;
            if (this.overlay == null) { return; }
            this.quietFrames++;
            // The canvas has a frame of the size of the view: the size attributes of a canvas whose
            // control went to a worker change with the frame that has that size.
            const settled = this.quietFrames >= 2 &&
                this.canvas.width === this.viewWidth && this.canvas.height === this.viewHeight;
            if (settled) { this.end(); } else { this.scheduleCheck(); }
        });
    }

    private end(): void {
        this.canvas.style.visibility = "";
        this.overlay?.remove();
        this.overlay = undefined;
        this.overlayContext = undefined;
        this.port.postMessage({ ferrouiCmd: "resizeFrames", active: false });
    }

    public close(): void {
        if (this.overlay != null) { this.end(); }
        this.port.onmessage = null;
        this.port.close();
    }
}

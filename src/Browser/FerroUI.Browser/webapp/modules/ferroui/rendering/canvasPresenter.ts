// Not in the original, whose render worker draws to a canvas transferred to it
// (`transferControlToOffscreen`). The page then shows whatever the worker last committed, on
// the worker's schedule, while the size of the canvas element follows the layout of the page on
// the page's schedule: no API orders the two, so while a page is resized the browser shows
// frames of the old size stretched to the new one.
//
// Here the worker draws to an `OffscreenCanvas` of its own and hands each frame it renders to
// the page as an `ImageBitmap` (`transferToImageBitmap`, without a copy), with the size and
// scaling it was rendered at. The page shows the frame (`transferFromImageBitmap`, without a
// copy) and gives the canvas element the size of that frame in the same task, so the browser
// never shows a frame at a size other than its own. The view's size comes from its container
// (`CanvasSurface` observes the container in this mode), not from the canvas.

export interface PresentedFrame {
    bitmap: ImageBitmap;
    // The size of the frame in device pixels and the device pixel ratio it was rendered for.
    width: number;
    height: number;
    scaling: number;
}

export class CanvasPresenter {
    private readonly context: ImageBitmapRenderingContext;

    constructor(private readonly canvas: HTMLCanvasElement, private readonly port: MessagePort) {
        const context = canvas.getContext("bitmaprenderer");
        if (context == null) {
            throw new Error("HTMLCanvasElement.getContext(\"bitmaprenderer\") returned null.");
        }
        this.context = context;
        port.onmessage = ev => this.present(ev.data as PresentedFrame);
    }

    private present(frame: PresentedFrame): void {
        const canvas = this.canvas;
        // Setting the size clears the canvas; the frame replaces it in the same task.
        if (canvas.width !== frame.width) { canvas.width = frame.width; }
        if (canvas.height !== frame.height) { canvas.height = frame.height; }
        const width = `${frame.width / frame.scaling}px`;
        const height = `${frame.height / frame.scaling}px`;
        if (canvas.style.width !== width) { canvas.style.width = width; }
        if (canvas.style.height !== height) { canvas.style.height = height; }
        this.context.transferFromImageBitmap(frame.bitmap);
    }

    public close(): void {
        this.port.onmessage = null;
        this.port.close();
    }
}

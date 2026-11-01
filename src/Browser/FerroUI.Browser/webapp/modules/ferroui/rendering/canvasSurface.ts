import { ResizeHandler } from "./resizeHandler";
import { WebRenderTargetRegistry } from "./webRenderTargetRegistry";
import { FerroDOM } from "../dom";
import { BrowserRenderingMode } from "./renderingMode";
import { FerroExports } from "../ferroExports";

export class CanvasSurface {
    public targetId: number;
    private sizeParams?: [number, number, number];

    constructor(public canvas: HTMLCanvasElement, container: HTMLElement, modes: BrowserRenderingMode[], topLevelId: number, threadId: number) {
        this.targetId = WebRenderTargetRegistry.create(threadId, canvas, modes);
        // The size of the view. A canvas the page draws to fills its container and is observed
        // itself. The canvas of a render worker has the size of the frame it shows
        // (`CanvasPresenter`), so the view's size is its container's.
        const observed = threadId === 0 ? canvas : container;
        if (threadId !== 0) {
            // The canvas is shown at the size of its frame, which differs from the view's for the
            // moment between a resize and the frame of the new size: the container is the region
            // of the view, which the canvas positions itself in and never draws outside of (a
            // frame larger than a view that shrank would otherwise overflow the page and change
            // its layout).
            if (getComputedStyle(container).position === "static") { container.style.position = "relative"; }
            container.style.overflow = "hidden";
            canvas.style.left = "0px";
            canvas.style.top = "0px";
        }
        ResizeHandler.observeSize(observed, (width, height, dpr) => {
            this.sizeParams = [width, height, dpr];

            FerroExports.CanvasHelper?.OnSizeChanged(topLevelId, width, height, dpr);
        });
    }

    public get width() {
        if (this.sizeParams) { return this.sizeParams[0]; }
        return 1;
    }

    public get height() {
        if (this.sizeParams) { return this.sizeParams[1]; }
        return 1;
    }

    public get scaling() {
        if (this.sizeParams) { return this.sizeParams[2]; }
        return 1;
    }

    public destroy(): void {
    }

    public static create(container: HTMLElement, modes: BrowserRenderingMode[] | Int32Array, topLevelId: number, threadId: number): CanvasSurface {
        const canvas = FerroDOM.createFerroCanvas(container);
        FerroDOM.attachCanvas(container, canvas);
        try {
            return new CanvasSurface(canvas, container, Array.from(modes), topLevelId, threadId);
        } catch (ex) {
            FerroDOM.detachCanvas(container, canvas);
            throw ex;
        }
    }

    public static destroy(surface: CanvasSurface) {
        surface.destroy();
    }
}

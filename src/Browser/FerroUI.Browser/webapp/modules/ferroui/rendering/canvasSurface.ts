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
        const presenter = WebRenderTargetRegistry.presenterOf(this.targetId);
        if (presenter != null) {
            // The canvas of a render worker: while the view is resized, the page shows the frames
            // in an overlay at their own size (`CanvasPresenter`). The container is the region of
            // the view, which the overlay is positioned in and never drawn outside of (the frame of
            // a view that shrank is larger than the view until the frame of the new size).
            if (getComputedStyle(container).position === "static") { container.style.position = "relative"; }
            container.style.overflow = "hidden";
            canvas.style.left = "0px";
            canvas.style.top = "0px";
        }
        ResizeHandler.observeSize(canvas, (width, height, dpr) => {
            const changed = this.sizeParams != null && (this.sizeParams[0] !== width || this.sizeParams[1] !== height);
            this.sizeParams = [width, height, dpr];
            if (changed) { presenter?.resized(width, height); }

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

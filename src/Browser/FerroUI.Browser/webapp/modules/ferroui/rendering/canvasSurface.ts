import { ResizeHandler } from "./resizeHandler";
import { WebRenderTargetRegistry } from "./webRenderTargetRegistry";
import { FerroDOM } from "../dom";
import { BrowserRenderingMode } from "./renderingMode";
import { FerroExports } from "../ferroExports";
import { clearViewCanvas, setViewCanvas } from "../pointerPosition";

export class CanvasSurface {
    public targetId: number;
    private sizeParams: [number, number, number];
    private readonly container: HTMLElement;

    constructor(public canvas: HTMLCanvasElement, container: HTMLElement, modes: BrowserRenderingMode[], topLevelId: number, threadId: number) {
        this.container = container;
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
        // The framework reads these synchronously while constructing its top-level, before the
        // first ResizeObserver delivery. Start with the CSS content size and the real scale, so
        // initial layout and composition do not use a 1x1 surface at scale 1 on a Retina display.
        this.sizeParams = ResizeHandler.getInitialSize(canvas);
        ResizeHandler.observeSize(canvas, (width, height, dpr) => {
            const changed = this.sizeParams[0] !== width || this.sizeParams[1] !== height;
            this.sizeParams = [width, height, dpr];
            if (changed) { presenter?.resized(width, height); }

            FerroExports.CanvasHelper?.OnSizeChanged(topLevelId, width, height, dpr);
        });
        // Not in the original: the pointer positions of the view are measured on this canvas, at
        // the size and scaling the framework renders it at (pointerPosition.ts).
        setViewCanvas(container, this);
    }

    public get width() {
        return this.sizeParams[0];
    }

    public get height() {
        return this.sizeParams[1];
    }

    public get scaling() {
        return this.sizeParams[2];
    }

    public destroy(): void {
        clearViewCanvas(this.container, this);
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

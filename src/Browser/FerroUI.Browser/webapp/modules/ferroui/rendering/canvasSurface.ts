import { ResizeHandler } from "./resizeHandler";
import { WebRenderTargetRegistry } from "./webRenderTargetRegistry";
import { FerroDOM } from "../dom";
import { BrowserRenderingMode } from "./renderingMode";
import { FerroExports } from "../ferroExports";

export class CanvasSurface {
    public targetId: number;
    private sizeParams?: [number, number, number];

    constructor(public canvas: HTMLCanvasElement, modes: BrowserRenderingMode[], topLevelId: number) {
        this.targetId = WebRenderTargetRegistry.create(canvas, modes);
        ResizeHandler.observeSize(canvas, (width, height, dpr) => {
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

    public static create(container: HTMLElement, modes: BrowserRenderingMode[] | Int32Array, topLevelId: number): CanvasSurface {
        const canvas = FerroDOM.createFerroCanvas(container);
        FerroDOM.attachCanvas(container, canvas);
        try {
            return new CanvasSurface(canvas, Array.from(modes), topLevelId);
        } catch (ex) {
            FerroDOM.detachCanvas(container, canvas);
            throw ex;
        }
    }

    public static destroy(surface: CanvasSurface) {
        surface.destroy();
    }
}

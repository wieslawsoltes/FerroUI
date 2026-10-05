import { BrowserRenderingMode } from "./renderingMode";
import { WebGlRenderTarget } from "./webGlRenderTarget";
import { WebRenderTarget } from "./webRenderTarget";
import { SoftwareRenderTarget } from "./softwareRenderTarget";

export class WebRenderTargetRegistry {
    private static targets: { [id: number]: (WebRenderTarget) } = {};
    private static registry: { [id: number]: ({
        canvas: HTMLCanvasElement;
    }); } = {};

    private static nextId = 1;

    static create(canvas: HTMLCanvasElement, preferredModes: BrowserRenderingMode[]): number {
        const id = WebRenderTargetRegistry.nextId++;
        WebRenderTargetRegistry.registry[id] = {
            canvas
        };
        WebRenderTargetRegistry.targets[id] = WebRenderTargetRegistry.createRenderTarget(canvas, preferredModes);
        return id;
    }

    static getRenderTarget(id: number): WebRenderTarget | undefined {
        return WebRenderTargetRegistry.targets[id];
    }

    private static createRenderTarget(canvas: HTMLCanvasElement | OffscreenCanvas, modes: BrowserRenderingMode[]): WebRenderTarget {
        for (const mode of modes) {
            try {
                if (mode === BrowserRenderingMode.Software2D) { return new SoftwareRenderTarget(canvas); }
                return new WebGlRenderTarget(canvas, mode);
            } catch (e) {
                let message = "";
                if (e instanceof Error) { message = ": " + e.message; }
                console.error(`Failed to create render target for mode ${mode} ${message}`);
            }
        }
        // Still try software as a fallback
        return new SoftwareRenderTarget(canvas);
    }
}

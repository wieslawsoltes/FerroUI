import { WebRenderTarget } from "./webRenderTarget";

// The part of `navigator.gpu` this file uses: the types of WebGPU are not in the libraries of the
// project.
interface PageGpu {
    getPreferredCanvasFormat(): string;
    requestAdapter(options?: { powerPreference?: string }): Promise<{ requestDevice(): Promise<unknown> } | null>;
}

function pageGpu(): PageGpu | undefined {
    return (navigator as unknown as { gpu?: PageGpu }).gpu;
}

// A canvas a render backend draws to with the WebGPU of the page. The backend makes its surface
// from the selector of the canvas, so the canvas stays on the thread of the page and has an id.
export class WebGpuRenderTarget extends WebRenderTarget {
    private static nextId = 0;
    private readonly selector: string;

    constructor(canvas: HTMLCanvasElement | OffscreenCanvas) {
        if (pageGpu() == null) {
            throw new Error("The browser has no WebGPU.");
        }
        if (typeof HTMLCanvasElement === "undefined" || !(canvas instanceof HTMLCanvasElement)) {
            throw new Error("WebGPU draws to a canvas on the thread of the page.");
        }
        super(canvas, "webgpu");
        if (canvas.id === "") {
            canvas.id = `ferroui-webgpu-${++WebGpuRenderTarget.nextId}`;
        }
        this.selector = `#${CSS.escape(canvas.id)}`;
    }

    static selectorOf(target: WebGpuRenderTarget): string {
        return target.selector;
    }

    static prefersRgba8(): boolean {
        return pageGpu()?.getPreferredCanvasFormat() === "rgba8unorm";
    }

    // The device for the module, which has to exist before the module is created (its option
    // `preinitializedWebGPUDevice`); null when the browser gives none.
    static async requestDevice(): Promise<unknown> {
        try {
            const adapter = await pageGpu()?.requestAdapter({ powerPreference: "high-performance" });
            return adapter == null ? null : await adapter.requestDevice();
        } catch (e) {
            console.error("No WebGPU device:", e);
            return null;
        }
    }
}

import { BrowserRenderingMode } from "./renderingMode";
import { WebGlRenderTarget } from "./webGlRenderTarget";
import { WebRenderTarget } from "./webRenderTarget";
import { SoftwareRenderTarget } from "./softwareRenderTarget";
import { FerroExports } from "../ferroExports";

export class WebRenderTargetRegistry {
    private static targets: { [id: number]: (WebRenderTarget) } = {};
    private static registry: { [id: number]: ({
        canvas: HTMLCanvasElement;
        worker?: Worker;
    }); } = {};

    private static nextId = 1;

    static create(pthreadId: number, canvas: HTMLCanvasElement, preferredModes: BrowserRenderingMode[]): number {
        const id = WebRenderTargetRegistry.nextId++;
        if (pthreadId === 0) {
            WebRenderTargetRegistry.registry[id] = {
                canvas
            };
            WebRenderTargetRegistry.targets[id] = WebRenderTargetRegistry.createRenderTarget(canvas, preferredModes);
        } else {
            // The module the host page attached: its table of threads is how the worker of a
            // thread is found (`PThread` is an exported runtime method of the threaded build).
            const module = FerroExports.runtime;
            const pthreads = module?.PThread;
            if (pthreads == null) { throw new Error("Unable to access emscripten PThread api"); }
            const pthread = pthreads.pthreads[pthreadId];
            if (pthread == null) { throw new Error(`Unable get pthread with id ${pthreadId}`); }
            let worker: Worker | undefined;
            if (pthread.postMessage != null) { worker = pthread as Worker; } else { worker = pthread.worker; }

            if (worker == null) { throw new Error(`Unable get Worker for pthread ${pthreadId}`); }
            const offscreen = canvas.transferControlToOffscreen();
            worker.postMessage({
                ferrouiCmd: "registerCanvas",
                canvas: offscreen,
                modes: preferredModes,
                id
            }, [offscreen]);
            WebRenderTargetRegistry.registry[id] = {
                canvas,
                worker
            };
        }
        return id;
    }

    static initializeWorker() {
        const oldHandler = self.onmessage;
        self.onmessage = ev => {
            const msg = ev;
            if (msg.data.ferrouiCmd === "registerCanvas") {
                const target = WebRenderTargetRegistry.createRenderTarget(msg.data.canvas, msg.data.modes);
                WebRenderTargetRegistry.targets[msg.data.id] = target;
                // Not in the original, which asks its registry for the target on each frame. Here
                // the thread that created the canvas has a registry of its own, which never has
                // the target, so the worker reports it: 2 for WebGL, 1 for software.
                FerroExports.CanvasHelper?.OnRenderTargetRegistered(msg.data.id, target.renderTargetType === "webgl" ? 2 : 1);
            } else if (msg.data.ferrouiCmd === "unregisterCanvas") {
                /* eslint-disable */
                // Our keys are _always_ numbers and are safe to delete
                delete WebRenderTargetRegistry.targets[msg.data.id];
                /* eslint-enable */
            } else if (oldHandler != null) { oldHandler.call(self, ev); }
        };
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

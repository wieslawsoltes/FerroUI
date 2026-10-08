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

    // Not in the original, which waits for its render worker before it creates a view. Here the
    // start of the application cannot wait for a thread, so a canvas may be created for a render
    // thread that has been started and has not installed its handler yet: it is created with this
    // id in place of the id of the thread. Its control is transferred at once and the message for
    // the worker is held back until `workerStarted` names the thread. A message posted earlier
    // would reach the handler of the runtime, which does not know it.
    private static readonly pendingThread = -1;
    // The render thread `workerStarted` named; 0 until then.
    private static workerThreadId = 0;
    private static heldBack: Array<{
        id: number;
        canvas: OffscreenCanvas;
        modes: BrowserRenderingMode[];
    }> = [];

    static create(pthreadId: number, canvas: HTMLCanvasElement, preferredModes: BrowserRenderingMode[]): number {
        const id = WebRenderTargetRegistry.nextId++;
        if (pthreadId === 0) {
            WebRenderTargetRegistry.registry[id] = {
                canvas
            };
            WebRenderTargetRegistry.targets[id] = WebRenderTargetRegistry.createRenderTarget(canvas, preferredModes);
        } else if (pthreadId === WebRenderTargetRegistry.pendingThread && WebRenderTargetRegistry.workerThreadId === 0) {
            WebRenderTargetRegistry.heldBack.push({
                id,
                canvas: canvas.transferControlToOffscreen(),
                modes: preferredModes
            });
            WebRenderTargetRegistry.registry[id] = {
                canvas
            };
        } else {
            const worker = WebRenderTargetRegistry.getWorker(
                pthreadId === WebRenderTargetRegistry.pendingThread ? WebRenderTargetRegistry.workerThreadId : pthreadId);
            WebRenderTargetRegistry.postCanvas(worker, id, canvas.transferControlToOffscreen(), preferredModes);
            WebRenderTargetRegistry.registry[id] = {
                canvas,
                worker
            };
        }
        return id;
    }

    // Not in the original. The thread `pthreadId` has installed its handler (`initializeWorker`):
    // called on the thread that creates the canvases, once, when it hears of the thread. The
    // canvases that were created for it before are posted to its worker now.
    static workerStarted(pthreadId: number) {
        WebRenderTargetRegistry.workerThreadId = pthreadId;
        const heldBack = WebRenderTargetRegistry.heldBack;
        if (heldBack.length === 0) { return; }
        WebRenderTargetRegistry.heldBack = [];
        const worker = WebRenderTargetRegistry.getWorker(pthreadId);
        for (const held of heldBack) {
            WebRenderTargetRegistry.postCanvas(worker, held.id, held.canvas, held.modes);
            const entry = WebRenderTargetRegistry.registry[held.id];
            if (entry != null) { entry.worker = worker; }
        }
    }

    private static getWorker(pthreadId: number): Worker {
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
        return worker;
    }

    private static postCanvas(worker: Worker, id: number, offscreen: OffscreenCanvas, modes: BrowserRenderingMode[]) {
        worker.postMessage({
            ferrouiCmd: "registerCanvas",
            canvas: offscreen,
            modes,
            id
        }, [offscreen]);
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

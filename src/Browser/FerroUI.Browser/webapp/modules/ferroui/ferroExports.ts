// The WebAssembly module of the application: the functions the framework exports to this script,
// Emscripten's GL object and the module memory (`heapU8`).
//
// The host page creates the module, registers it here and only then runs the application:
//
//     const runtime = await createRuntime();
//     FerroExports.attach(runtime);
//     runtime.runMain();
//
// so that every export is resolved before the first callback can fire.
//
// A thread of a module built with threads is a web worker with its own copy of this script and of
// the module. Nobody calls `attach` by hand there: the script of the module does it when it is
// loaded into the worker (scripts/browser/threads/ferroui-worker-attach.js).
export interface FerroRuntime {
    GL?: any;
    // The memory of the module (Emscripten's `wasmMemory`, a WebAssembly.Memory), read through
    // `FerroExports.heapU8()`.
    wasmMemory?: { readonly buffer: ArrayBufferLike };
    [name: string]: any;
}

export class FerroExports {
    public static resolvedExports?: FerroRuntime;

    public static attach(runtime: FerroRuntime): void {
        FerroExports.resolvedExports = runtime;
        FerroExports.heap = undefined;
        for (const key of Object.keys(FerroExports.groups)) {
            // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
            delete FerroExports.groups[key];
        }
    }

    public static get runtime(): FerroRuntime | undefined {
        return FerroExports.resolvedExports;
    }

    private static heap?: Uint8Array;

    // The memory of the attached module as bytes. Call it at every use and do not keep the result:
    // a view is only good for the buffer the memory has now. A memory that grows gets a new buffer.
    // In a module without threads the old one is detached; in a module with threads the memory is
    // shared and the old buffer stays as it is, ending where the memory ended before, also when
    // another thread did the growing. Reading `buffer` of the memory object gives the current one
    // in both cases, so the view is made anew whenever that is not the buffer it is over.
    //
    // The view may be over shared memory, which several interfaces of the browser refuse (ImageData,
    // Blob, the write of a stream): hand them a copy (`slice` copies into an ordinary buffer).
    public static heapU8(): Uint8Array {
        const buffer = FerroExports.resolvedExports?.wasmMemory?.buffer;
        if (!buffer) {
            throw new Error("The module is not attached, or its memory is not exported");
        }
        let heap = FerroExports.heap;
        if (heap?.buffer !== buffer) {
            heap = new Uint8Array(buffer);
            FerroExports.heap = heap;
        }
        return heap;
    }

    // Not in the original. How many calls the runtime has carried from the thread of this script
    // to the main thread of the page (a call of the C library that only the main thread can serve),
    // or -1 when they are not counted: the script of a module built with threads counts them in
    // the worker of each thread (scripts/browser/threads/ferroui-worker-attach.js), nothing counts
    // on the main thread or without threads.
    public static proxiedCalls(): number {
        const count: unknown = FerroExports.resolvedExports?.ferrouiProxiedCalls;
        return typeof count === "number" ? count : -1;
    }

    // Not in the original. The index of the function of the last such call in the table of the
    // script of the module (`proxiedFunctionTable`), or -1 when there was none.
    public static lastProxiedFunction(): number {
        const index: unknown = FerroExports.resolvedExports?.ferrouiLastProxiedFunction;
        return typeof index === "number" ? index : -1;
    }

    // Not in the original. The messages of the panics of the render thread, oldest first, for the
    // host page (and for the tests of the port).
    public static readonly renderThreadPanics: string[] = [];

    // Not in the original, whose render loop logs the exception of a frame through the logger of
    // the process. Called on the thread of the page with the message of a panic of the render
    // thread (the framework queues the call from that thread): what a worker writes to its own
    // console does not reach the console of the page, so the page logs it. The render loop goes on
    // with its next tick, as the original's does after an exception.
    public static reportRenderThreadPanic(message: string): void {
        FerroExports.renderThreadPanics.push(message);
        console.error(`FerroUI: the render thread panicked: ${message}`);
    }

    private static readonly groups: { [key: string]: any } = {};

    // The functions of one group, looked up once per attached module.
    private static group<T extends { [name: string]: string }>(key: string, names: T): { [K in keyof T]: (...args: any[]) => any } | undefined {
        const exports = FerroExports.resolvedExports;
        if (!exports) { return undefined; }
        let group = FerroExports.groups[key];
        if (!group) {
            group = {};
            for (const name of Object.keys(names)) {
                group[name] = exports[names[name]];
            }
            FerroExports.groups[key] = group;
        }
        return group;
    }

    public static get InputHelper() {
        return FerroExports.group("InputHelper", {
            OnKeyDown: "InputHelper_OnKeyDown",
            OnKeyUp: "InputHelper_OnKeyUp",
            OnBeforeInput: "InputHelper_OnBeforeInput",
            OnCompositionStart: "InputHelper_OnCompositionStart",
            OnCompositionUpdate: "InputHelper_OnCompositionUpdate",
            OnCompositionEnd: "InputHelper_OnCompositionEnd",
            OnPointerMove: "InputHelper_OnPointerMove",
            OnPointerDown: "InputHelper_OnPointerDown",
            OnPointerUp: "InputHelper_OnPointerUp",
            OnPointerCancel: "InputHelper_OnPointerCancel",
            OnWheel: "InputHelper_OnWheel",
            OnKeyboardGeometryChange: "InputHelper_OnKeyboardGeometryChange",
            OnLostFocus: "InputHelper_OnLostFocus",
            OnDragDrop: "InputHelper_OnDragDrop"
        });
    }

    public static get DomHelper() {
        return FerroExports.group("DomHelper", {
            DarkModeChanged: "DomHelper_DarkModeChanged",
            DocumentVisibilityChanged: "DomHelper_DocumentVisibilityChanged",
            LanguageChanged: "DomHelper_LanguageChanged",
            ScreensChanged: "DomHelper_ScreensChanged"
        });
    }

    public static get TimerHelper() {
        return FerroExports.group("TimerHelper", {
            JsExportOnAnimationFrame: "TimerHelper_JsExportOnAnimationFrame"
        });
    }

    public static get SingleThreadedDispatcherImpl() {
        return FerroExports.group("SingleThreadedDispatcherImpl", {
            OnSignaled: "BrowserSingleThreadedDispatcherImpl_OnSignaled",
            OnReadyForBackgroundProcessing: "BrowserSingleThreadedDispatcherImpl_OnReadyForBackgroundProcessing",
            OnTimer: "BrowserSingleThreadedDispatcherImpl_OnTimer"
        });
    }

    public static get CanvasHelper() {
        return FerroExports.group("CanvasHelper", {
            OnSizeChanged: "CanvasHelper_OnSizeChanged",
            OnRenderTargetRegistered: "CanvasHelper_OnRenderTargetRegistered"
        });
    }

    public static get NavigationHelper() {
        return FerroExports.group("NavigationHelper", {
            OnBackRequested: "NavigationHelper_OnBackRequested"
        });
    }

    public static get CompletionHelper() {
        return FerroExports.group("CompletionHelper", {
            OnResolved: "CompletionHelper_OnResolved",
            OnRejected: "CompletionHelper_OnRejected"
        });
    }
}

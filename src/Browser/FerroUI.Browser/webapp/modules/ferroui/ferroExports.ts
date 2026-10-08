// The WebAssembly module of the application: the functions the framework exports to this script,
// Emscripten's GL object and the module memory.
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
    HEAPU8: Uint8Array;
    [name: string]: any;
}

export class FerroExports {
    public static resolvedExports?: FerroRuntime;

    public static attach(runtime: FerroRuntime): void {
        FerroExports.resolvedExports = runtime;
        for (const key of Object.keys(FerroExports.groups)) {
            // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
            delete FerroExports.groups[key];
        }
    }

    public static get runtime(): FerroRuntime | undefined {
        return FerroExports.resolvedExports;
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

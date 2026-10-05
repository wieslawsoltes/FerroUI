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

    public static get DomHelper() {
        return FerroExports.group("DomHelper", {
            DarkModeChanged: "DomHelper_DarkModeChanged",
            DocumentVisibilityChanged: "DomHelper_DocumentVisibilityChanged",
            LanguageChanged: "DomHelper_LanguageChanged"
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
            OnSizeChanged: "CanvasHelper_OnSizeChanged"
        });
    }
}

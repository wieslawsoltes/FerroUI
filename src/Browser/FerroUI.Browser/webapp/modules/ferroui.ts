import { InputHelper } from "./ferroui/input";
import { FerroDOM } from "./ferroui/dom";
import { Caniuse } from "./ferroui/caniuse";
import { TimerHelper } from "./ferroui/timer";
import { SingleThreadedDispatcherHelper } from "./ferroui/singleThreadedDispatcher";
import { CanvasSurface } from "./ferroui/rendering/canvasSurface";
import { WebRenderTargetRegistry } from "./ferroui/rendering/webRenderTargetRegistry";
import { WebRenderTarget } from "./ferroui/rendering/webRenderTarget";
import { SoftwareRenderTarget } from "./ferroui/rendering/softwareRenderTarget";
import { WebGlRenderTarget } from "./ferroui/rendering/webGlRenderTarget";
import { FerroExports } from "./ferroui/ferroExports";
import { CompletionHelper } from "./ferroui/completionHelper";
import { ScreenHelper } from "./ferroui/screens";
import { NavigationHelper } from "./ferroui/navigationHelper";
import { StreamHelper } from "./ferroui/stream";
import { NativeControlHost } from "./ferroui/nativeControlHost";

function getModuleUrl(): string {
    return import.meta.url;
}

function resolveModuleUrl(name: string): string {
    const meta = import.meta as ImportMeta & { resolve?: (specifier: string) => string };
    return meta.resolve ? meta.resolve(name) : new URL(name, import.meta.url).href;
}

async function registerServiceWorker(path: string, scope: string | undefined) {
    if ("serviceWorker" in navigator) {
        await globalThis.navigator.serviceWorker.register(path, scope ? { scope } : undefined);
    }
}

// The storage bundle, imported on first use: it carries the file system polyfill. The framework
// calls into it through StorageModule.module.
const StorageModule: { module?: any } = {};

async function importStorage(): Promise<void> {
    if (!StorageModule.module) {
        StorageModule.module = await import(resolveModuleUrl("./storage.js"));
    }
}

export {
    Caniuse,
    InputHelper,
    FerroDOM,
    TimerHelper,
    SingleThreadedDispatcherHelper,
    WebRenderTarget,
    CanvasSurface,
    WebRenderTargetRegistry,
    SoftwareRenderTarget,
    WebGlRenderTarget,
    FerroExports,
    CompletionHelper,
    ScreenHelper,
    NavigationHelper,
    StreamHelper,
    NativeControlHost,
    StorageModule,
    registerServiceWorker,
    getModuleUrl,
    resolveModuleUrl,
    importStorage
};

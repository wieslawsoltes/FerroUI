import { FerroExports } from "../ferroExports";
import { BrowserRenderingMode } from "./renderingMode";
import { WebRenderTarget } from "./webRenderTarget";
interface EmscriptenGlContext {
    handle: number;
}

interface EmscriptenGL {
    registerContext: (ctx: WebGLRenderingContext, attrs: WebGLContextAttributes) => number;
    currentContext?: EmscriptenGlContext;
    makeContextCurrent: (handle: number) => boolean;
    deleteContext?: (handle: number) => void;
}

function getGL(): EmscriptenGL {
    return FerroExports.runtime?.GL as EmscriptenGL;
}
export interface FrameGlContext {
    context: WebGLRenderingContext;
    generation: number;
}

export class WebGlRenderTarget extends WebRenderTarget {
    public contextHandle?: number;
    public attrs: WebGLContextAttributes;
    public fboId?: number;
    public stencil?: number;
    public sample?: number;
    public depth?: number;
    private static _gl: EmscriptenGL | null = null;
    private static readonly frameContexts = new Map<number, FrameGlContext>();

    constructor(public canvas: HTMLCanvasElement | OffscreenCanvas, mode: BrowserRenderingMode) {
        // Skia only understands WebGL context wrapped in Emscripten.
        if (WebGlRenderTarget._gl == null) { WebGlRenderTarget._gl = getGL(); }
        if (!WebGlRenderTarget._gl) {
            throw new Error("Module.GL object wasn't initialized, WebGL can't be used.");
        }

        const attrs: WebGLContextAttributes | any =
            {
                alpha: true,
                depth: true,
                stencil: true,
                antialias: false,
                premultipliedAlpha: true,
                preserveDrawingBuffer: false,
                // only supported on older browsers, which is perfect as we want to fallback to 2d there.
                failIfMajorPerformanceCaveat: true,
                // attrs used by Emscripten:
                majorVersion: mode === BrowserRenderingMode.WebGL1 ? 1 : 2,
                minorVersion: 0,
                enableExtensionsByDefault: 1,
                explicitSwapControl: 0
            };

        const context = (mode === BrowserRenderingMode.WebGL1
            ? canvas.getContext("webgl", attrs)
            : canvas.getContext("webgl2", attrs)) as WebGLRenderingContext;
        if (!context) {
            throw new Error("HTMLCanvasElement.getContext returned null.");
        }

        const handle = WebGlRenderTarget._gl.registerContext(context, attrs);
        (context as any).gl_handle = handle;
        super(canvas, "webgl");
        const frameContext = { context, generation: 1 };
        WebGlRenderTarget.frameContexts.set(handle, frameContext);
        canvas.addEventListener("webglcontextlost", event => event.preventDefault());
        canvas.addEventListener("webglcontextrestored", () => { frameContext.generation++; });

        this.contextHandle = handle;
        this.fboId = context.getParameter(context.FRAMEBUFFER_BINDING)?.id ?? 0;
        this.stencil = context.getParameter(context.STENCIL_BITS);
        this.sample = context.getParameter(context.SAMPLES);
        this.depth = context.getParameter(context.DEPTH_BITS);
        this.attrs = attrs;
    }

    // Not in the original, where a context lives as long as the page. The context leaves the
    // table of Emscripten's GL object, which is what keeps a WebGL context alive after its view
    // is closed. Whatever drew with it (Skia's context) has been released before, with this
    // context current.
    public release(): void {
        super.release();
        const handle = this.contextHandle;
        this.contextHandle = undefined;
        if (handle != null) { WebGlRenderTarget.frameContexts.delete(handle); }
        if (handle != null) { WebGlRenderTarget._gl?.deleteContext?.(handle); }
    }

    public static getCurrentContext(): number {
        return WebGlRenderTarget._gl?.currentContext?.handle ?? 0;
    }

    public static makeContextCurrent(handle: number): boolean {
        if (WebGlRenderTarget._gl == null) { return false; }
        const ret = WebGlRenderTarget._gl.makeContextCurrent(handle);
        return handle === 0 || ret;
    }

    public static frameContext(handle: number): FrameGlContext | undefined {
        // The context of a canvas of this thread: of the page, or of the render worker, which takes
        // media frames transferred from the page (#601). Which route a producer uses is decided by
        // where the compositor's context lives (`renders_on_render_thread`).
        return WebGlRenderTarget.frameContexts.get(handle);
    }

    public static frameGeneration(handle: number): number {
        return WebGlRenderTarget.frameContext(handle)?.generation ?? 0;
    }

    public static contextLost(handle: number): boolean {
        const context = WebGlRenderTarget.frameContexts.get(handle)?.context;
        return !context || context.isContextLost();
    }
}

import { FerroExports } from "../ferroExports";
import { FrameGlContext, WebGlRenderTarget } from "./webGlRenderTarget";

interface FrameTexture { texture: WebGLTexture; context: WebGL2RenderingContext; generation: number }
/** Shared immutable GL storage, not a media decoder or DOM presentation layer. */
export class SharedGlFrames {
    private static readonly textures = new Map<number, FrameTexture>();
    private static readonly freeNames: number[] = [];

    public static graphics(handle: number): number {
        const gl = WebGlRenderTarget.frameContext(handle)?.context;
        if (!gl || gl.isContextLost()) { return -1; }
        const extension = gl.getExtension("WEBGL_debug_renderer_info");
        const renderer = String(extension ? gl.getParameter(extension.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER));
        // Identification is useful to reject known software adapters. An
        // unrecognized/hidden renderer remains Unknown, never hardware proof.
        return /swiftshader|llvmpipe|softpipe|software rasterizer/i.test(renderer) ? 2 : 0;
    }

    private static admitted(handle: number, generation: number): FrameGlContext {
        const entry = WebGlRenderTarget.frameContext(handle);
        if (!entry || entry.generation !== generation || entry.context.isContextLost() ||
            !(entry.context instanceof WebGL2RenderingContext)) {
            throw new Error("An active page-owned WebGL2 context and matching generation are required");
        }
        return entry;
    }

    public static create(handle: number, generation: number): number {
        const gl = SharedGlFrames.admitted(handle, generation).context as WebGL2RenderingContext;
        const texture = gl.createTexture();
        if (!texture) { throw new Error("WebGL texture allocation failed"); }
        const runtime = FerroExports.runtime?.GL;
        if (!runtime) { gl.deleteTexture(texture); throw new Error("Emscripten GL registry is unavailable"); }
        let name: number | undefined;
        while (SharedGlFrames.freeNames.length) {
            const candidate = SharedGlFrames.freeNames.pop();
            if (candidate === undefined) { break; }
            if (!runtime.textures[candidate]) { name = candidate; break; }
        }
        const allocatedName = name ?? Number(runtime.getNewId(runtime.textures));
        (texture as any).name = allocatedName;
        runtime.textures[allocatedName] = texture;
        SharedGlFrames.textures.set(allocatedName, { texture, context: gl, generation });
        return allocatedName;
    }

    public static release(name: number): void {
        const entry = SharedGlFrames.textures.get(name);
        if (!entry) { return; }
        SharedGlFrames.textures.delete(name);
        // Keep the actual context/object, so unregistering a canvas does not
        // prevent post-fence destruction and a recycled name cannot be deleted.
        entry.context.deleteTexture(entry.texture);
        const table = FerroExports.runtime?.GL?.textures;
        if (table?.[name] === entry.texture) {
            table[name] = null;
            SharedGlFrames.freeNames.push(name);
        }
    }

    private static write(name: number, operation: (gl: WebGL2RenderingContext) => void): void {
        const entry = SharedGlFrames.textures.get(name);
        if (!entry || entry.context.isContextLost()) { throw new Error("Frame texture was retired or lost"); }
        const gl = entry.context;
        const previous = gl.getParameter(gl.TEXTURE_BINDING_2D);
        try {
            gl.bindTexture(gl.TEXTURE_2D, entry.texture);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
            operation(gl);
            const error = gl.getError();
            if (error !== gl.NO_ERROR) { throw new Error(`Frame GPU operation failed (${error})`); }
        } finally { gl.bindTexture(gl.TEXTURE_2D, previous); }
    }

    public static isVideoSource(source: unknown): boolean {
        return source instanceof HTMLVideoElement;
    }

    /** Only native video images: CPU-backed ImageData/canvas routes are not admitted. */
    public static upload(name: number, source: HTMLVideoElement, width: number, height: number): void {
        if (!(source instanceof HTMLVideoElement)) { throw new Error("Only a native video source is admitted"); }
        if (source.videoWidth !== width || source.videoHeight !== height) { throw new Error("Native source dimensions changed or require an unsupported pixel-aspect transform"); }
        SharedGlFrames.write(name, gl => {
            const flip = gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
            const premultiplied = gl.getParameter(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL);
            const conversion = gl.getParameter(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL);
            const buffer = gl.getParameter(gl.PIXEL_UNPACK_BUFFER_BINDING);
            const rowLength = gl.getParameter(gl.UNPACK_ROW_LENGTH);
            const skipRows = gl.getParameter(gl.UNPACK_SKIP_ROWS);
            const skipPixels = gl.getParameter(gl.UNPACK_SKIP_PIXELS);
            const unpackColorSpace = (gl as any).unpackColorSpace;
            try {
                gl.bindBuffer(gl.PIXEL_UNPACK_BUFFER, null);
                gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
                gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
                gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
                gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, 0);
                gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
                gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.BROWSER_DEFAULT_WEBGL);
                if (unpackColorSpace != null) { (gl as any).unpackColorSpace = "srgb"; }
                gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, source);
            } finally {
                gl.bindBuffer(gl.PIXEL_UNPACK_BUFFER, buffer);
                gl.pixelStorei(gl.UNPACK_ROW_LENGTH, rowLength);
                gl.pixelStorei(gl.UNPACK_SKIP_ROWS, skipRows);
                gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, skipPixels);
                gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, flip);
                gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, premultiplied);
                gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, conversion);
                if (unpackColorSpace != null) { (gl as any).unpackColorSpace = unpackColorSpace; }
            }
        });
    }

    /** Independent GPU producer: four known quadrants, no CPU pixel upload. */
    public static quadrants(name: number, width: number, height: number): void {
        SharedGlFrames.write(name, gl => {
            gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, width, height);
            const previous = gl.getParameter(gl.DRAW_FRAMEBUFFER_BINDING);
            const scissor = gl.isEnabled(gl.SCISSOR_TEST); const box = gl.getParameter(gl.SCISSOR_BOX);
            const clear = gl.getParameter(gl.COLOR_CLEAR_VALUE); const mask = gl.getParameter(gl.COLOR_WRITEMASK);
            const fbo = gl.createFramebuffer();
            try {
                gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, fbo);
                gl.framebufferTexture2D(gl.DRAW_FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, gl.getParameter(gl.TEXTURE_BINDING_2D), 0);
                if (gl.checkFramebufferStatus(gl.DRAW_FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) { throw new Error("Frame framebuffer is incomplete"); }
                gl.enable(gl.SCISSOR_TEST); gl.colorMask(true, true, true, true);
                const w = Math.floor(width / 2); const h = Math.floor(height / 2);
                for (const [x, y, c] of [[0, h, [1, 0, 0, 1]], [w, h, [0, 1, 0, 1]], [0, 0, [0, 0, 1, 1]], [w, 0, [1, 1, 0, 1]]] as const) {
                    gl.scissor(x, y, x === 0 ? w : width - w, y === 0 ? h : height - h);
                    gl.clearColor(c[0], c[1], c[2], c[3]); gl.clear(gl.COLOR_BUFFER_BIT);
                }
            } finally {
                gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, previous); gl.deleteFramebuffer(fbo);
                gl.scissor(box[0], box[1], box[2], box[3]); if (!scissor) { gl.disable(gl.SCISSOR_TEST); }
                gl.clearColor(clear[0], clear[1], clear[2], clear[3]); gl.colorMask(mask[0], mask[1], mask[2], mask[3]);
            }
        });
    }

    /** Poll only with zero timeout, yielding between polls. No UI fence wait. */
    public static complete(handle: number, signal: (success: number) => void): boolean {
        const entry = WebGlRenderTarget.frameContext(handle);
        if (!entry || !(entry.context instanceof WebGL2RenderingContext) || entry.context.isContextLost()) { return false; }
        const gl = entry.context; const sync = gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0);
        if (!sync) { return false; }
        gl.flush();
        const poll = () => {
            const status = gl.isContextLost() ? gl.WAIT_FAILED : gl.clientWaitSync(sync, 0, 0);
            if (status === gl.TIMEOUT_EXPIRED) { setTimeout(poll, 16); return; }
            gl.deleteSync(sync);
            signal(status === gl.ALREADY_SIGNALED || status === gl.CONDITION_SATISFIED ? 1 : 0);
        };
        setTimeout(poll, 0);
        return true;
    }
}

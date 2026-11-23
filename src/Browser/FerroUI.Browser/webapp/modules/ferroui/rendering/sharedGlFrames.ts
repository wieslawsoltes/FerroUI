import { QrScanner } from "./qrScanner";
import { FerroExports } from "../ferroExports";
import { FrameGlContext, WebGlRenderTarget } from "./webGlRenderTarget";
import { WebRenderTargetRegistry } from "./webRenderTargetRegistry";

interface FrameTexture { texture: WebGLTexture; context: WebGL2RenderingContext; generation: number }
/** Shared immutable GL storage, not a media decoder or DOM presentation layer. */
export class SharedGlFrames {
    private static readonly textures = new Map<number, FrameTexture>();
    private static readonly freeNames: number[] = [];
    // #601. On the render worker: the media frames the page transferred, by their number, until
    // the compositor imports them (`importTransferred`) or the page discards them. On the page:
    // the next number.
    private static readonly transferred = new Map<number, { frame?: VideoFrame; receipt?: MessagePort }>();
    private static readonly sent = new Map<number, {
        analysis?: VideoFrame;
        dispose: () => Promise<void>;
    }>();

    private static nextTransfer = 1;

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
        SharedGlFrames.write(name, gl => SharedGlFrames.writeImage(gl, source));
    }

    /** Page (#601): whether media frames can go to the render worker (WebCodecs and a started worker). */
    public static canTransfer(): boolean {
        return typeof VideoFrame === "function" && WebRenderTargetRegistry.renderWorker() !== undefined;
    }

    /**
     * Page (#601): wraps the current image of the media element as a WebCodecs `VideoFrame` and
     * transfers it to the render worker, whose context samples it; returns the number of the
     * transfer. The frame stays GPU-backed; no pixel array is made.
     */
    public static transfer(source: HTMLVideoElement, width: number, height: number): number {
        return SharedGlFrames.beginTransfer(source, width, height, false).id;
    }

    /** Receipt, not postMessage, makes a frame importable by a shared-memory render batch. */
    public static beginTransfer(source: HTMLVideoElement, width: number, height: number, analyze: boolean): { id: number; ready: Promise<void> } {
        if (!(source instanceof HTMLVideoElement)) { throw new Error("Only a native video source is admitted"); }
        if (typeof VideoFrame !== "function") { throw new Error("The render worker needs WebCodecs VideoFrame"); }
        if (source.videoWidth !== width || source.videoHeight !== height) { throw new Error("Native source dimensions changed"); }
        const worker = WebRenderTargetRegistry.renderWorker();
        if (!worker) { throw new Error("The render worker has not started"); }
        const frame = new VideoFrame(source, { timestamp: Math.round(source.currentTime * 1e6) });
        let id: number;
        do {
            id = SharedGlFrames.nextTransfer;
            SharedGlFrames.nextTransfer = id === 0x7fffffff ? 1 : id + 1;
        } while (SharedGlFrames.sent.has(id));
        let channel: MessageChannel;
        let analysis: VideoFrame | undefined;
        try {
            analysis = analyze ? frame.clone() : undefined;
            channel = new MessageChannel();
        } catch (error) {
            frame.close(); analysis?.close(); throw error;
        }
        let accept!: () => void; let rejectReady!: (error: Error) => void;
        let release!: () => void; let rejectRetired!: (error: Error) => void;
        const ready = new Promise<void>((resolve, reject) => { accept = resolve; rejectReady = reject; });
        const retired = new Promise<void>((resolve, reject) => { release = resolve; rejectRetired = reject; });
        // The lease may be cancelled before a Rust future starts tracking these promises.
        void ready.catch(() => {}); void retired.catch(() => {});
        let disposing = false; let failed: Error | undefined;
        let timer: ReturnType<typeof setTimeout>;
        const cleanup = () => {
            clearTimeout(timer); channel.port1.close(); analysis?.close();
            worker.removeEventListener("error", onError);
        };
        const fail = (error: Error) => {
            failed = error; cleanup(); rejectReady(error); rejectRetired(error);
            if (disposing) { SharedGlFrames.sent.delete(id); }
            // The receipt port is closed now, so a worker that is slow rather than gone would keep
            // the frame for good. Its messages arrive in order: the frame first, then this.
            try { worker.postMessage({ ferrouiCmd: "discardMediaFrame", id }); } catch { /* The worker is gone. */ }
        };
        const onError = () => fail(new Error("The media render worker failed"));
        worker.addEventListener("error", onError);
        channel.port1.onmessage = event => {
            if (event.data === "received" && !disposing) { clearTimeout(timer); accept(); }
            if (event.data === "retired") {
                cleanup(); SharedGlFrames.sent.delete(id);
                rejectReady(new DOMException("The frame was retired before receipt", "AbortError")); release();
            }
        };
        channel.port1.onmessageerror = () => fail(new Error("Invalid media transfer receipt"));
        const dispose = async () => {
            if (!disposing) {
                disposing = true;
                rejectReady(new DOMException("Media transfer cancelled", "AbortError"));
                if (failed) { SharedGlFrames.sent.delete(id); return await retired; }
                clearTimeout(timer);
                // A port buffers this even when the worker has not received the frame yet.
                try {
                    channel.port1.postMessage("retire");
                    timer = setTimeout(() => fail(new Error("Media frame retirement timed out")), 15000);
                } catch (error) { fail(error instanceof Error ? error : new Error(String(error))); }
            }
            return await retired;
        };
        SharedGlFrames.sent.set(id, { analysis, dispose });
        timer = setTimeout(() => fail(new Error("Media frame receipt timed out")), 15000);
        try {
            worker.postMessage({ ferrouiCmd: "mediaFrame", id, frame, receipt: channel.port2 },
                [frame as unknown as Transferable, channel.port2]);
        } catch (error) {
            frame.close(); channel.port2.close(); cleanup(); SharedGlFrames.sent.delete(id);
            rejectReady(error as Error); release(); throw error;
        }
        return { id, ready };
    }

    /** The worker records ownership before acknowledging a frame to the producer. */
    public static receive(id: number, frame: VideoFrame, receipt?: MessagePort): void {
        const entry = { frame: frame as VideoFrame | undefined, receipt };
        SharedGlFrames.transferred.set(id, entry);
        if (receipt) {
            receipt.onmessage = event => { if (event.data === "retire") { SharedGlFrames.retireReceived(id); } };
            receipt.postMessage("received");
        }
    }

    private static retireReceived(id: number): void {
        const entry = SharedGlFrames.transferred.get(id);
        if (!entry) { return; }
        SharedGlFrames.transferred.delete(id);
        entry.frame?.close(); entry.receipt?.postMessage("retired"); entry.receipt?.close();
    }

    /** Completes only after the worker releases its native image, including a cancelled transfer. */
    public static async discard(id: number): Promise<void> {
        const entry = SharedGlFrames.sent.get(id);
        if (entry) { return await entry.dispose(); }
        SharedGlFrames.retireReceived(id);
    }

    /** Explicit analysis of the immutable published frame, never a preview fallback. */
    public static async recognize(id: number, transferred: boolean, width: number, height: number): Promise<string> {
        return await QrScanner.scan(width, height, () => {
            if (transferred) {
                const frame = SharedGlFrames.sent.get(id)?.analysis;
                if (!frame) { throw new Error("The frame has no retained analysis image"); }
                return frame.clone();
            }
            const entry = SharedGlFrames.textures.get(id);
            if (!entry || entry.context.isContextLost()) { throw new Error("QR frame was retired or lost"); }
            const gl = entry.context;
            const previous = gl.getParameter(gl.READ_FRAMEBUFFER_BINDING);
            const buffer = gl.getParameter(gl.PIXEL_PACK_BUFFER_BINDING);
            const parameters = [gl.PACK_ALIGNMENT, gl.PACK_ROW_LENGTH, gl.PACK_SKIP_ROWS, gl.PACK_SKIP_PIXELS];
            const values = parameters.map(p => gl.getParameter(p));
            const fbo = gl.createFramebuffer();
            if (!fbo) { throw new Error("QR readback framebuffer is unavailable"); }
            try {
                gl.bindFramebuffer(gl.READ_FRAMEBUFFER, fbo);
                gl.framebufferTexture2D(gl.READ_FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, entry.texture, 0);
                if (gl.checkFramebufferStatus(gl.READ_FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) { throw new Error("QR framebuffer is incomplete"); }
                gl.bindBuffer(gl.PIXEL_PACK_BUFFER, null);
                for (const parameter of parameters) { gl.pixelStorei(parameter, parameter === gl.PACK_ALIGNMENT ? 1 : 0); }
                const pixels = new Uint8ClampedArray(width * height * 4);
                gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
                if (gl.getError() !== gl.NO_ERROR) { throw new Error("QR frame readback failed"); }
                return pixels;
            } finally {
                gl.bindFramebuffer(gl.READ_FRAMEBUFFER, previous); gl.deleteFramebuffer(fbo);
                gl.bindBuffer(gl.PIXEL_PACK_BUFFER, buffer);
                parameters.forEach((p, i) => gl.pixelStorei(p, values[i]));
            }
        });
    }

    /**
     * Render worker (#601): uploads the transferred frame `id` into a new texture of the context
     * `handle` and closes the frame; returns the name of the texture, which `release` deletes.
     */
    public static importTransferred(handle: number, generation: number, id: number, width: number, height: number): number {
        const entry = SharedGlFrames.transferred.get(id);
        const frame = entry?.frame;
        if (!entry || !frame) { throw new Error("The transferred media frame is not on the render worker (retired, or not arrived)"); }
        entry.frame = undefined;
        if (!entry.receipt) { SharedGlFrames.transferred.delete(id); }
        try {
            if (frame.displayWidth !== width || frame.displayHeight !== height) {
                throw new Error("The transferred media frame has other dimensions than its metadata");
            }
            const name = SharedGlFrames.create(handle, generation);
            try {
                SharedGlFrames.write(name, gl => SharedGlFrames.writeImage(gl, frame));
            } catch (error) {
                SharedGlFrames.release(name);
                throw error;
            }
            return name;
        } finally {
            frame.close();
        }
    }

    /** Uploads a native image as RGBA8, with the unpack state of the context restored after. */
    private static writeImage(gl: WebGL2RenderingContext, source: TexImageSource): void {
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

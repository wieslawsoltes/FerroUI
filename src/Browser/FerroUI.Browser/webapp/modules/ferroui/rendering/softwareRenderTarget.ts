import { FerroExports } from "../ferroExports";
import { WebRenderTarget } from "./webRenderTarget";

export class SoftwareRenderTarget extends WebRenderTarget {
    private readonly context: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
    constructor(canvas: HTMLCanvasElement | OffscreenCanvas) {
        const context = canvas.getContext("2d", {
            alpha: true
        });
        if (!context) {
            throw new Error("HTMLCanvasElement.getContext(2d) returned null.");
        }

        super(canvas, "software");
        this.context = context;
    }

    // The pixels as the canvas wants them. Kept between frames; replaced when the size changes.
    private straight?: Uint8ClampedArray<ArrayBuffer>;

    public putPixelData(pointer: number, length: number, width: number, height: number): void {
        // The view is created per call: the module memory is replaced when it grows.
        const heap8 = FerroExports.runtime?.HEAPU8;

        let source: Uint8Array;
        if (heap8?.buffer) {
            source = new Uint8Array(heap8.buffer, pointer, length);
        } else throw new Error("Unable to access the module memory");

        if (this.straight?.length !== length) {
            this.straight = new Uint8ClampedArray(new ArrayBuffer(length));
        }

        SoftwareRenderTarget.unpremultiply(source, this.straight);

        const imageData = new ImageData(this.straight, width, height);
        (this.context).putImageData(imageData, 0, 0);
    }

    // The framework renders with premultiplied alpha; putImageData expects straight alpha.
    // Opaque and fully transparent pixels, nearly all of a frame, are copied as they are.
    public static unpremultiply(source: Uint8Array, target: Uint8ClampedArray): void {
        const length = source.length;
        for (let i = 0; i < length; i += 4) {
            const alpha = source[i + 3];
            if (alpha === 255 || alpha === 0) {
                target[i] = source[i];
                target[i + 1] = source[i + 1];
                target[i + 2] = source[i + 2];
            } else {
                const scale = 255 / alpha;
                // Clamped and rounded by the target array.
                target[i] = source[i] * scale;
                target[i + 1] = source[i + 1] * scale;
                target[i + 2] = source[i + 2] * scale;
            }
            target[i + 3] = alpha;
        }
    }

    public static staticPutPixelData(target: SoftwareRenderTarget, pointer: number, length: number, width: number, height: number): void {
        target.putPixelData(pointer, length, width, height);
    }
}

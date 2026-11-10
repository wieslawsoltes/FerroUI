/*! QR decoding uses jsQR; see qr-worker.LICENSE.txt for its Apache-2.0 license. */
import jsQR from "jsqr";

interface Request { id: number; width: number; height: number; pixels?: Uint8ClampedArray; frame?: VideoFrame }
const worker = self as unknown as {
    onmessage: (event: MessageEvent<Request>) => void;
    postMessage: (value: unknown) => void;
};

worker.onmessage = event => {
    const { id, width, height, frame } = event.data;
    try {
        if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || width > 1280 || height > 720) {
            throw new Error("QR image exceeds the analysis bound");
        }
        let pixels = event.data.pixels;
        if (frame) {
            if (frame.displayWidth !== width || frame.displayHeight !== height) { throw new Error("QR frame dimensions changed"); }
            const canvas = new OffscreenCanvas(width, height);
            const context = canvas.getContext("2d", { willReadFrequently: true });
            if (!context) { throw new Error("QR analysis canvas is unavailable"); }
            context.drawImage(frame, 0, 0);
            pixels = context.getImageData(0, 0, width, height).data;
            canvas.width = 0; canvas.height = 0;
        }
        if (!pixels || pixels.length !== width * height * 4) { throw new Error("Invalid QR image"); }
        const results: Array<{ text: string; payload: number[]; corners: number[][] }> = [];
        const inputCopies: number[] = [];
        const regions = [{ x: 0, y: 0, width, height, depth: 0 }];
        for (let r = 0; r < regions.length && results.length < 8; r++) {
            const region = regions[r];
            let input = pixels;
            if (r !== 0) {
                input = new Uint8ClampedArray(region.width * region.height * 4);
                inputCopies.push(input.byteLength);
                for (let y = 0; y < region.height; y++) {
                    const start = ((region.y + y) * width + region.x) * 4;
                    input.set(pixels.subarray(start, start + region.width * 4), y * region.width * 4);
                }
            }
            let found = false;
            for (let i = 0; i < 8 && results.length < 8; i++) {
                const code = jsQR(input, region.width, region.height, { inversionAttempts: "attemptBoth" });
                if (!code) { break; }
                found = true;
                const l = code.location;
                const local = [l.topLeftCorner, l.topRightCorner, l.bottomRightCorner, l.bottomLeftCorner];
                const corners = local.map(p => [(p.x + region.x) / width, (p.y + region.y) / height]);
                const center = corners.reduce((a, p) => [a[0] + p[0] / 4, a[1] + p[1] / 4], [0, 0]);
                if (!results.some(v => v.text === code.data && Math.hypot(
                    v.corners.reduce((a, p) => a + p[0] / 4, 0) - center[0],
                    v.corners.reduce((a, p) => a + p[1] / 4, 0) - center[1]) < 0.05)) {
                    results.push({ text: code.data, payload: code.binaryData, corners });
                }
                const left = Math.max(0, Math.floor(Math.min(...local.map(p => p.x))));
                const right = Math.min(region.width, Math.ceil(Math.max(...local.map(p => p.x))));
                const top = Math.max(0, Math.floor(Math.min(...local.map(p => p.y))));
                const bottom = Math.min(region.height, Math.ceil(Math.max(...local.map(p => p.y))));
                for (let y = top; y < bottom; y++) { input.fill(255, (y * region.width + left) * 4, (y * region.width + right) * 4); }
            }
            // A single-code finder may combine finder patterns from separate symbols.
            // Retry bounded overlapping regions, including vertically arranged portrait images.
            if (!found && region.depth < 2 && Math.min(region.width, region.height) >= 64) {
                const horizontal = region.width >= region.height;
                const span = Math.ceil((horizontal ? region.width : region.height) * 0.55);
                for (const second of [false, true]) {
                    regions.push({
                        x: region.x + (horizontal && second ? region.width - span : 0),
                        y: region.y + (!horizontal && second ? region.height - span : 0),
                        width: horizontal ? span : region.width,
                        height: horizontal ? region.height : span,
                        depth: region.depth + 1
                    });
                }
            }
        }
        worker.postMessage({ id, results, inputCopies });
    } catch (error) {
        worker.postMessage({ id, error: String(error) });
    } finally { frame?.close(); }
};

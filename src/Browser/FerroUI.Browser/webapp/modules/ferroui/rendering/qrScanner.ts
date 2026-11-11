/** One lazy decoder worker; preview never starts it or reads pixels. */
export class QrScanner {
    private static worker?: Worker;
    private static next = 0;
    private static pending?: { id: number; resolve: (value: string) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> };

    public static async scan(width: number, height: number, acquire: () => VideoFrame | Uint8ClampedArray): Promise<string> {
        if (QrScanner.pending) { throw new DOMException("QR analysis is busy", "InvalidStateError"); }
        if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || width > 1280 || height > 720) {
            throw new Error("QR image exceeds the analysis bound");
        }
        if (!QrScanner.worker) {
            const worker = new Worker(new URL("./qr-worker.js", import.meta.url), { type: "module" });
            QrScanner.worker = worker;
            worker.onmessage = event => {
                const pending = QrScanner.pending;
                if (!pending || event.data.id !== pending.id) { return; }
                clearTimeout(pending.timer); QrScanner.pending = undefined;
                if (event.data.error) { pending.reject(new Error(String(event.data.error))); } else { pending.resolve(JSON.stringify({ results: event.data.results, inputCopies: event.data.inputCopies })); }
            };
            worker.onerror = () => QrScanner.fail(new Error("QR decoder worker failed"));
            worker.onmessageerror = () => QrScanner.fail(new Error("Invalid QR decoder reply"));
        }
        return await new Promise((resolve, reject) => {
            const id = ++QrScanner.next;
            const timer = setTimeout(() => QrScanner.fail(new Error("QR decode timed out")), 5000);
            QrScanner.pending = { id, resolve, reject, timer };
            let image: VideoFrame | Uint8ClampedArray | undefined;
            try {
                image = acquire();
                const pixels = image instanceof Uint8ClampedArray ? image : undefined;
                const frame = pixels ? undefined : image as VideoFrame;
                const worker = QrScanner.worker;
                if (!worker) { throw new Error("QR worker is unavailable"); }
                worker.postMessage({ id, width, height, pixels, frame },
                    [pixels ? pixels.buffer as ArrayBuffer : frame as unknown as Transferable]);
            } catch (error) {
                if (image && !(image instanceof Uint8ClampedArray)) { image.close(); }
                clearTimeout(timer); QrScanner.pending = undefined; reject(error instanceof Error ? error : new Error(String(error)));
            }
        });
    }

    private static fail(error: Error): void {
        const pending = QrScanner.pending;
        QrScanner.pending = undefined;
        if (pending) { clearTimeout(pending.timer); pending.reject(error); }
        QrScanner.worker?.terminate(); QrScanner.worker = undefined;
    }
}

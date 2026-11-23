/** Decode the native preview without depending on a paint in the visible webview. */
export class PreviewBitmap {
    private readonly context: CanvasRenderingContext2D;
    private displayedPng?: string;

    constructor(private readonly canvas: HTMLCanvasElement) {
        // The renderer has already rasterized the preview. Keep its backing pixels off the compositor.
        this.context = canvas.getContext('2d', { willReadFrequently: true })!;
    }

    async decode(png: string): Promise<ImageBitmap | null> {
        if (png === this.displayedPng) return null;
        const binary = atob(png);
        const bytes = new Uint8Array(binary.length);
        for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
        return createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    }

    present(png: string, bitmap: ImageBitmap | null): void {
        if (bitmap) {
            try {
                if (this.canvas.width !== bitmap.width) this.canvas.width = bitmap.width;
                if (this.canvas.height !== bitmap.height) this.canvas.height = bitmap.height;
                // Replace transparent pixels in the same paint as the opaque ones.
                this.context.globalCompositeOperation = 'copy';
                this.context.drawImage(bitmap, 0, 0);
            } finally {
                bitmap.close();
            }
        }
        this.displayedPng = png;
    }
}

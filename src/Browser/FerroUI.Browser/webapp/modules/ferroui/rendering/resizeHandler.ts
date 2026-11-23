type ResizeObserverWithCallbacks = {
    callbacks: Map<Element, ((width: number, height: number, dpr: number) => void)>;
} & ResizeObserver;

export class ResizeHandler {
    private static resizeObserver?: ResizeObserverWithCallbacks;
    private static resolutionQuery?: MediaQueryList;
    private static observedDpr?: number;

    private static currentDpr(): number {
        const dpr = window.devicePixelRatio;
        return Number.isFinite(dpr) && dpr > 0 ? dpr : 1;
    }

    // A synchronous estimate for construction, before the first observer delivery. Measure the
    // untransformed content box; the observer will refine its device-pixel rounding.
    public static getInitialSize(element: HTMLElement): [number, number, number] {
        const dpr = this.currentDpr();
        if (element.getClientRects().length === 0) { return [0, 0, dpr]; }
        const style = getComputedStyle(element);
        let width = parseFloat(style.width);
        let height = parseFloat(style.height);
        if (style.boxSizing === "border-box") {
            width -= parseFloat(style.paddingLeft) + parseFloat(style.paddingRight) +
                parseFloat(style.borderLeftWidth) + parseFloat(style.borderRightWidth);
            height -= parseFloat(style.paddingTop) + parseFloat(style.paddingBottom) +
                parseFloat(style.borderTopWidth) + parseFloat(style.borderBottomWidth);
        }
        return [Math.max(0, Math.round(width * dpr)), Math.max(0, Math.round(height * dpr)), dpr];
    }

    public static observeSize(element: HTMLElement, callback: (width: number, height: number, dpr: number) => void) : (() => void) {
        if (!this.resizeObserver) {
            this.resizeObserver = new ResizeObserver(this.onResize) as ResizeObserverWithCallbacks;
            this.resizeObserver.callbacks = new Map<Element, ((width: number, height: number, dpr: number) => void)>();
        }

        this.resizeObserver.callbacks.set(element, callback);
        this.resizeObserver.observe(element, { box: "content-box" });
        if (this.resizeObserver.callbacks.size === 1) {
            this.watchDpr();
            window.addEventListener("resize", this.onDprChanged);
        }

        return () => {
            if (this.resizeObserver?.callbacks.get(element) !== callback) { return; }
            this.resizeObserver?.callbacks.delete(element);
            this.resizeObserver?.unobserve(element);
            if (this.resizeObserver?.callbacks.size === 0) {
                this.resolutionQuery?.removeEventListener("change", this.onDprChanged);
                this.resolutionQuery = undefined;
                this.observedDpr = undefined;
                window.removeEventListener("resize", this.onDprChanged);
            }
        };
    }

    private static watchDpr(): void {
        this.resolutionQuery?.removeEventListener("change", this.onDprChanged);
        this.observedDpr = this.currentDpr();
        this.resolutionQuery = window.matchMedia?.(`(resolution: ${this.observedDpr}dppx)`);
        this.resolutionQuery?.addEventListener("change", this.onDprChanged);
    }

    private static readonly onDprChanged = (): void => {
        if (!this.resizeObserver?.callbacks.size || this.currentDpr() === this.observedDpr) { return; }
        // A content-box observer need not fire when moving between displays or
        // changing emulation DPR without changing CSS size. Re-arm the query at
        // the new ratio and measure current CSS, never a cached device box.
        this.watchDpr();
        this.resizeObserver?.callbacks.forEach((resize, element) => {
            resize(...this.getInitialSize(element as HTMLElement));
        });
    };

    private static onResize(entries: ResizeObserverEntry[], observer: ResizeObserver) {
        for (const entry of entries) {
            const callback = (observer as ResizeObserverWithCallbacks).callbacks.get(entry.target);
            if (!callback) {
                continue;
            }

            const dpr = ResizeHandler.currentDpr();
            // contentRect gives physical width/height in untransformed CSS
            // pixels, including in vertical writing modes and older observers.
            const width = entry.contentRect.width * dpr;
            const height = entry.contentRect.height * dpr;
            const deviceBox = entry.devicePixelContentBoxSize?.[0];
            // Preserve the browser's precise pixel snapping when it agrees with
            // the scale sent to the renderer. DevTools emulation and a queued
            // delivery during a DPR change can expose a box at another ratio.
            // Edge snapping can differ from CSS * DPR by up to one device pixel.
            const consistent = deviceBox != null &&
                deviceBox.inlineSize >= 0 && deviceBox.blockSize >= 0 &&
                Math.abs(deviceBox.inlineSize - width) <= 1 &&
                Math.abs(deviceBox.blockSize - height) <= 1;
            callback(
                Math.max(0, Math.round(consistent ? deviceBox.inlineSize : width)),
                Math.max(0, Math.round(consistent ? deviceBox.blockSize : height)),
                dpr);
        }
    }
}

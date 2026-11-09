type ResizeObserverWithCallbacks = {
    callbacks: Map<Element, ((width: number, height: number, dpr: number) => void)>;
} & ResizeObserver;

export class ResizeHandler {
    private static resizeObserver?: ResizeObserverWithCallbacks;

    // A synchronous estimate for construction, before the first observer delivery. Measure the
    // untransformed content box; the observer will refine its device-pixel rounding.
    public static getInitialSize(element: HTMLElement): [number, number, number] {
        const dpr = window.devicePixelRatio;
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

        return () => {
            this.resizeObserver?.callbacks.delete(element);
            this.resizeObserver?.unobserve(element);
        };
    }

    private static onResize(entries: ResizeObserverEntry[], observer: ResizeObserver) {
        for (const entry of entries) {
            const callback = (observer as ResizeObserverWithCallbacks).callbacks.get(entry.target);
            if (!callback) {
                continue;
            }

            const trueDpr = window.devicePixelRatio;
            let width;
            let height;
            let dpr = trueDpr;
            if (entry.devicePixelContentBoxSize) {
                // NOTE: Only this path gives the correct answer
                // The other paths are imperfect fallbacks
                // for browsers that don't provide anyway to do this
                width = entry.devicePixelContentBoxSize[0].inlineSize;
                height = entry.devicePixelContentBoxSize[0].blockSize;
                dpr = 1; // it's already in width and height
            } else if (entry.contentBoxSize) {
                if (entry.contentBoxSize[0]) {
                    width = entry.contentBoxSize[0].inlineSize;
                    height = entry.contentBoxSize[0].blockSize;
                } else {
                    width = (entry.contentBoxSize as any).inlineSize;
                    height = (entry.contentBoxSize as any).blockSize;
                }
            } else {
                width = entry.contentRect.width;
                height = entry.contentRect.height;
            }
            const displayWidth = Math.round(width * dpr);
            const displayHeight = Math.round(height * dpr);
            callback(displayWidth, displayHeight, trueDpr);
        }
    }
}

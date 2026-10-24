import { FerroExports } from "./ferroExports";

export class FerroDOM {
    public static getGlobalThis() {
        return globalThis;
    }

    public static addClass(element: HTMLElement, className: string): void {
        element.classList.add(className);
    }

    static getFirstElementById(className: string, parent: HTMLElement | Window): Element | null {
        const parentNode = parent instanceof Window
            ? parent.document
            : parent.ownerDocument;

        return parentNode.getElementById(className);
    }

    static getFirstElementByClassName(className: string, parent: HTMLElement | Window): Element | null {
        const parentNode = parent instanceof Window
            ? parent.document
            : parent;

        const elements = parentNode.getElementsByClassName(className);
        return elements ? elements[0] : null;
    }

    static createFerroCanvas(host: HTMLElement): HTMLCanvasElement {
        const containerId = host.getAttribute("data-containerId") ?? "0000";

        const canvas = document.createElement("canvas");
        canvas.id = `canvas${containerId}`;
        canvas.classList.add("ferroui-canvas");
        canvas.style.width = "100%";
        canvas.style.height = "100%";
        canvas.style.position = "absolute";

        return canvas;
    }

    static attachCanvas(host: HTMLElement, canvas: HTMLCanvasElement): void {
        host.prepend(canvas);
    }

    static detachCanvas(host: HTMLElement, canvas: HTMLCanvasElement): void {
        host.removeChild(canvas);
    }

    static createFerroHost(host: HTMLElement) {
        const containerId = Math.random().toString(36).replace(/[^a-z]+/g, "").substr(2, 10);

        // Root element
        host.classList.add("ferroui-container");
        host.tabIndex = 0;
        host.setAttribute("data-containerId", containerId);
        host.oncontextmenu = function () { return false; };
        host.style.overflow = "hidden";
        host.style.touchAction = "none";

        // Canvas is lazily created depending on the rendering mode. See createFerroCanvas usage.

        // Native controls host
        const nativeHost = document.createElement("div");
        nativeHost.id = `nativeHost${containerId}`;
        nativeHost.classList.add("ferroui-native-host");
        nativeHost.style.left = "0px";
        nativeHost.style.top = "0px";
        nativeHost.style.width = "100%";
        nativeHost.style.height = "100%";
        nativeHost.style.position = "absolute";

        // IME
        const inputElement = document.createElement("input");
        inputElement.id = `inputElement${containerId}`;
        inputElement.classList.add("ferroui-input-element");
        // Not in the original: the element takes the focus only when the framework gives it, so
        // that Tab leaves the view in one step instead of stopping on a hidden element.
        inputElement.tabIndex = -1;
        inputElement.autocapitalize = "none";
        inputElement.type = "text";
        inputElement.spellcheck = false;
        inputElement.style.padding = "0";
        inputElement.style.margin = "0";
        inputElement.style.borderWidth = "0";
        inputElement.style.position = "absolute";
        inputElement.style.overflow = "hidden";
        inputElement.style.borderStyle = "hidden";
        inputElement.style.outline = "none";
        inputElement.style.background = "transparent";
        inputElement.style.color = "transparent";
        inputElement.style.display = "none";
        inputElement.style.height = "20px";
        // Not in the original, whose element has the default font of an input (13.33 pixels): a
        // mobile browser zooms the page into a focused input whose text is smaller than 16 pixels
        // (Safari on iOS always, Chrome on Android to a legible size), and the view, which takes
        // every touch (`touch-action: none`), then cannot be pinched back to scale 1 when the
        // keyboard hides. The text of the element is transparent, so the size shows nowhere.
        inputElement.style.fontSize = "16px";
        inputElement.style.zIndex = "-1";
        inputElement.onpaste = function () { return false; };
        inputElement.oncopy = function () { return false; };
        inputElement.oncut = function () { return false; };

        host.prepend(inputElement);
        host.prepend(nativeHost);

        return {
            host,
            nativeHost,
            inputElement
        };
    }

    public static isFullscreen(globalThis: Window): boolean {
        return globalThis.document.fullscreenElement != null;
    }

    public static async setFullscreen(globalThis: Window, isFullscreen: boolean) {
        if (isFullscreen) {
            const doc = globalThis.document.documentElement;
            await doc.requestFullscreen();
        } else {
            await globalThis.document.exitFullscreen();
        }
    }

    public static initGlobalDomEvents(globalThis: Window): void {
        // Init Safe Area properties.
        globalThis.document.documentElement.style.setProperty("--ferro-sat", "env(safe-area-inset-top)");
        globalThis.document.documentElement.style.setProperty("--ferro-sar", "env(safe-area-inset-right)");
        globalThis.document.documentElement.style.setProperty("--ferro-sab", "env(safe-area-inset-bottom)");
        globalThis.document.documentElement.style.setProperty("--ferro-sal", "env(safe-area-inset-left)");

        // Subscribe on DarkMode changes.
        if (globalThis.matchMedia !== undefined) {
            const colorSchemeMedia = globalThis.matchMedia("(prefers-color-scheme: dark)");
            const prefersContrastMedia = globalThis.matchMedia("(prefers-contrast: more)");

            colorSchemeMedia.addEventListener("change", (args: MediaQueryListEvent) => {
                FerroExports.DomHelper?.DarkModeChanged(args.matches, prefersContrastMedia.matches);
            });
            prefersContrastMedia.addEventListener("change", (args: MediaQueryListEvent) => {
                FerroExports.DomHelper?.DarkModeChanged(colorSchemeMedia.matches, args.matches);
            });
        }

        globalThis.addEventListener("languagechange", () => {
            FerroExports.DomHelper?.LanguageChanged(globalThis.navigator.language);
        });

        // Not in the original: a mobile browser scrolls the document to bring a focused input into
        // the part of the page the on-screen keyboard leaves (Safari on iOS even when the page
        // cannot scroll), and does not always scroll it back when the keyboard hides, which leaves
        // the view shifted with nothing to scroll it back with. When the keyboard has gone (the
        // visual viewport grows back to the height of the layout viewport) and the document has no
        // content to scroll, any offset it is left at is such a leftover and is undone. A page
        // that scrolls (a view embedded in a longer document) is left alone.
        const visualViewport = globalThis.visualViewport;
        if (visualViewport) {
            let lastHeight = visualViewport.height;
            visualViewport.addEventListener("resize", () => {
                const root = globalThis.document.scrollingElement ?? globalThis.document.documentElement;
                const grown = visualViewport.height > lastHeight;
                lastHeight = visualViewport.height;
                const keyboardHidden = grown && visualViewport.height >= globalThis.innerHeight - 1;
                const nothingToScroll = root.scrollHeight <= root.clientHeight && root.scrollWidth <= root.clientWidth;
                if (keyboardHidden && nothingToScroll && (globalThis.scrollX !== 0 || globalThis.scrollY !== 0)) {
                    globalThis.scrollTo(0, 0);
                }
            });
        }

        globalThis.document.addEventListener("visibilitychange", () => {
            FerroExports.DomHelper?.DocumentVisibilityChanged(globalThis.document.visibilityState);
        });

        // Report initial value.
        if (globalThis.document.visibilityState === "visible") {
            globalThis.setTimeout(() => {
                FerroExports.DomHelper?.DocumentVisibilityChanged(globalThis.document.visibilityState);
            }, 10);
        }
    }

    // Differs from the original, which returns left, top, bottom, right while the framework reads
    // left, top, right, bottom, so that the bottom inset became the right one and the other way
    // round.
    public static getSafeAreaPadding(globalThis: Window): number[] {
        const top = parseFloat(getComputedStyle(globalThis.document.documentElement).getPropertyValue("--ferro-sat"));
        const bottom = parseFloat(getComputedStyle(globalThis.document.documentElement).getPropertyValue("--ferro-sab"));
        const left = parseFloat(getComputedStyle(globalThis.document.documentElement).getPropertyValue("--ferro-sal"));
        const right = parseFloat(getComputedStyle(globalThis.document.documentElement).getPropertyValue("--ferro-sar"));

        return [left, top, right, bottom];
    }

    public static getDarkMode(globalThis: Window): number[] {
        if (globalThis.matchMedia === undefined) return [0, 0];

        const colorSchemeMedia = globalThis.matchMedia("(prefers-color-scheme: dark)");
        const prefersContrastMedia = globalThis.matchMedia("(prefers-contrast: more)");
        return [
            colorSchemeMedia.matches ? 1 : 0,
            prefersContrastMedia.matches ? 1 : 0
        ];
    }

    public static getNavigatorLanguage(globalThis: Window): string | null {
        return globalThis.navigator?.language ?? null;
    }

    // Whether the user lets animations play (not in the original): asked for each animation,
    // so that a change of the preference applies to the next one.
    public static getAnimationsEnabled(globalThis: Window): boolean {
        return globalThis.matchMedia === undefined ||
            !globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches;
    }
}

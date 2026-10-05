import { CaretHelper } from "./caretHelper";
import { FerroExports } from "./ferroExports";

enum RawInputModifiers {
    None = 0,
    Alt = 1,
    Control = 2,
    Shift = 4,
    Meta = 8,

    LeftMouseButton = 16,
    RightMouseButton = 32,
    MiddleMouseButton = 64,
    XButton1MouseButton = 128,
    XButton2MouseButton = 256,
    KeyboardMask = Alt | Control | Shift | Meta,

    PenInverted = 512,
    PenEraser = 1024,
    PenBarrelButton = 2048
}

export class InputHelper {
    static backgroundHandlersInitialized = false;

    public static initializeBackgroundHandlers() {
        if (this.backgroundHandlersInitialized) {
            return;
        }

        // The only handler of the page that is not bound to a view is the "paste" listener of the
        // clipboard fallback for old browsers. It is installed here together with the clipboard.
        this.backgroundHandlersInitialized = true;
    }

    public static subscribeInputEvents(element: HTMLInputElement, topLevelId: number) {
        const keySub = this.subscribeKeyEvents(element, topLevelId);
        const pointerSub = this.subscribePointerEvents(element, topLevelId);
        const textSub = this.subscribeTextEvents(element, topLevelId);
        const paneSub = this.subscribeKeyboardGeometryChange(element, topLevelId);
        // Not in the original, which never tells the framework that the view lost the focus.
        const focusSub = this.subscribeFocusEvents(element, topLevelId);

        return () => {
            keySub();
            pointerSub();
            textSub();
            paneSub();
            focusSub();
        };
    }

    // Not in the original, which never ends a subscription: the framework holds the function that
    // subscribeInputEvents returns and passes it back when the top-level is disposed.
    public static unsubscribeInputEvents(subscription: () => void) {
        subscription();
    }

    // Which default actions of the browser a key keeps while a view has the focus.
    //
    // The original answers the page asynchronously and, in effect, suppresses the default action
    // of almost every key. Here the framework answers synchronously whether it handled the key,
    // and the policy is:
    //
    // 1. A key that belongs to a composition of the input method, or is a dead key (an accent
    //    waiting for its letter), is left to the browser: preventing it would break the
    //    composition.
    // 2. A key the application handled does not also act on the page.
    // 3. Of the keys the application did not handle, these keep their default action:
    //    - every combination with Ctrl, Meta or Alt: the shortcuts of the browser and of the
    //      system (new tab, reload, address bar, zoom, history, developer tools, paste);
    //    - the function keys F1 to F24 (reload, full screen, developer tools);
    //    - Tab, so that the keyboard focus can leave the view when the application has nothing
    //      more to move it to.
    // 4. Every other key the application did not handle is prevented: characters (the framework
    //    has already received them as text, and they must not be typed into the hidden input
    //    element or start the quick find of the browser), and Space, the arrows, Page Up, Page
    //    Down, Home, End and Backspace, which would scroll the page or navigate its history.
    //
    // The default action of a key lies in the key going down; a key going up is prevented only
    // when the application handled it.
    private static shouldPreventDefault(args: KeyboardEvent, handled: boolean): boolean {
        if (args.isComposing || args.key === "Process" || args.key === "Dead" || args.keyCode === 229) {
            return false;
        }

        if (handled) {
            return true;
        }

        if (args.type !== "keydown") {
            return false;
        }

        if (args.ctrlKey || args.metaKey || args.altKey) {
            return false;
        }

        if (args.key === "Tab" || /^F\d{1,2}$/.test(args.key)) {
            return false;
        }

        return true;
    }

    public static subscribeKeyEvents(element: HTMLInputElement, topLevelId: number) {
        // Differs from the original, which awaits the answer of the framework and prevents the
        // default action afterwards: the answer is synchronous, so that the decision is made while
        // the event is still being dispatched. See shouldPreventDefault for the policy.
        const keyDownHandler = (args: KeyboardEvent) => {
            const handled: boolean =
                FerroExports.InputHelper?.OnKeyDown(topLevelId, args.code, args.key, this.getModifiers(args)) ?? false;
            if (this.shouldPreventDefault(args, handled)) {
                args.preventDefault();
            }
        };
        element.addEventListener("keydown", keyDownHandler);

        const keyUpHandler = (args: KeyboardEvent) => {
            const handled: boolean =
                FerroExports.InputHelper?.OnKeyUp(topLevelId, args.code, args.key, this.getModifiers(args)) ?? false;
            if (this.shouldPreventDefault(args, handled)) {
                args.preventDefault();
            }
        };

        element.addEventListener("keyup", keyUpHandler);

        return () => {
            element.removeEventListener("keydown", keyDownHandler);
            element.removeEventListener("keyup", keyUpHandler);
        };
    }

    public static subscribeTextEvents(
        element: HTMLInputElement,
        topLevelId: number) {
        const compositionStartHandler = (args: CompositionEvent) => {
            FerroExports.InputHelper?.OnCompositionStart(topLevelId);
        };
        element.addEventListener("compositionstart", compositionStartHandler);

        const beforeInputHandler = (args: InputEvent) => {
            const ranges = args.getTargetRanges();
            let start = -1;
            let end = -1;
            if (ranges.length > 0) {
                start = ranges[0].startOffset;
                end = ranges[0].endOffset;
            }

            if (args.inputType === "insertCompositionText") {
                start = 2;
                end = start + 2;
            }

            FerroExports.InputHelper?.OnBeforeInput(topLevelId, args.inputType, start, end);
        };
        element.addEventListener("beforeinput", beforeInputHandler);

        const compositionUpdateHandler = (args: CompositionEvent) => {
            FerroExports.InputHelper?.OnCompositionUpdate(topLevelId, args.data);
        };
        element.addEventListener("compositionupdate", compositionUpdateHandler);

        const compositionEndHandler = (args: CompositionEvent) => {
            FerroExports.InputHelper?.OnCompositionEnd(topLevelId, args.data);
            args.preventDefault();
        };
        element.addEventListener("compositionend", compositionEndHandler);

        return () => {
            element.removeEventListener("compositionstart", compositionStartHandler);
            // Differs from the original, which leaves the "beforeinput" listener in place.
            element.removeEventListener("beforeinput", beforeInputHandler);
            element.removeEventListener("compositionupdate", compositionUpdateHandler);
            element.removeEventListener("compositionend", compositionEndHandler);
        };
    }

    public static subscribePointerEvents(
        element: HTMLInputElement,
        topLevelId: number
    ) {
        const pointerMoveHandler = (args: PointerEvent) => {
            FerroExports.InputHelper?.OnPointerMove(
                topLevelId, args.pointerType, args.pointerId, args.offsetX, args.offsetY,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args), args);
            args.preventDefault();
        };

        const pointerDownHandler = (args: PointerEvent) => {
            FerroExports.InputHelper?.OnPointerDown(
                topLevelId, args.pointerType, args.pointerId, args.button, args.offsetX, args.offsetY,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args));
            args.preventDefault();
        };

        const pointerUpHandler = (args: PointerEvent) => {
            FerroExports.InputHelper?.OnPointerUp(
                topLevelId, args.pointerType, args.pointerId, args.button, args.offsetX, args.offsetY,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args));
            args.preventDefault();
        };

        const pointerCancelHandler = (args: PointerEvent) => {
            FerroExports.InputHelper?.OnPointerCancel(
                topLevelId, args.pointerType, args.pointerId, args.offsetX, args.offsetY,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args));
        };

        // Differs from the original, which does not pass the unit of the deltas: a browser reports
        // them in pixels, lines or pages (deltaMode), and the framework converts them.
        const wheelHandler = (args: WheelEvent) => {
            FerroExports.InputHelper?.OnWheel(
                topLevelId, args.offsetX, args.offsetY, args.deltaX, args.deltaY, args.deltaMode, this.getModifiers(args));
            args.preventDefault();
        };

        element.addEventListener("pointermove", pointerMoveHandler);
        element.addEventListener("pointerdown", pointerDownHandler);
        element.addEventListener("pointerup", pointerUpHandler);
        element.addEventListener("wheel", wheelHandler);
        element.addEventListener("pointercancel", pointerCancelHandler);

        return () => {
            // Differs from the original, which removes a "pointerover" listener that was never
            // added and so leaves the "pointermove" listener in place.
            element.removeEventListener("pointermove", pointerMoveHandler);
            element.removeEventListener("pointerdown", pointerDownHandler);
            element.removeEventListener("pointerup", pointerUpHandler);
            element.removeEventListener("pointercancel", pointerCancelHandler);
            element.removeEventListener("wheel", wheelHandler);
        };
    }

    // Not in the original. The focus leaves a view when it goes to something else in the document
    // outside the element of the view; going from the element to the hidden input element inside
    // it (or back) is not a loss, and neither is the window or the tab being deactivated.
    // "focusout" is the form of "blur" that also reports the elements inside.
    public static subscribeFocusEvents(
        element: HTMLInputElement,
        topLevelId: number
    ) {
        const focusOutHandler = (args: FocusEvent) => {
            const next = args.relatedTarget;
            if (next instanceof Node) {
                if (element.contains(next)) {
                    return;
                }
            } else if (!element.ownerDocument.hasFocus()) {
                // Nothing in the document takes the focus because the window or the tab is being
                // deactivated. The view keeps its focused element: when the window comes back the
                // browser focuses the same element again, and nothing would tell the framework.
                return;
            }

            FerroExports.InputHelper?.OnLostFocus(topLevelId);
        };
        element.addEventListener("focusout", focusOutHandler);

        return () => {
            element.removeEventListener("focusout", focusOutHandler);
        };
    }

    public static getCoalescedEvents(pointerEvent: PointerEvent): number[] {
        return pointerEvent.getCoalescedEvents()
            .flatMap(e => [e.offsetX, e.offsetY, e.pressure, e.tiltX, e.tiltY, e.twist ?? 0]);
    }

    public static subscribeKeyboardGeometryChange(
        element: HTMLInputElement,
        topLevelId: number) {
        if ("virtualKeyboard" in navigator) {
            // (navigator as any).virtualKeyboard.overlaysContent = true;
            const listener = (event: any) => {
                const elementRect = element.getBoundingClientRect();
                const keyboardRect = event.target.boundingRect as DOMRect;

                FerroExports.InputHelper?.OnKeyboardGeometryChange(
                    topLevelId,
                    keyboardRect.x - elementRect.x,
                    keyboardRect.y - elementRect.y,
                    keyboardRect.width,
                    keyboardRect.height);
            };
            (navigator as any).virtualKeyboard.addEventListener("geometrychange", listener);
            return () => {
                (navigator as any).virtualKeyboard.removeEventListener("geometrychange", listener);
            };
        }

        return () => { };
    }

    public static clearInput(inputElement: HTMLInputElement) {
        inputElement.value = "";
    }

    public static focusElement(inputElement: HTMLElement) {
        inputElement.focus();
    }

    public static setCursor(inputElement: HTMLInputElement, kind: string) {
        if (kind === "default") {
            inputElement.style.removeProperty("cursor");
        } else {
            inputElement.style.cursor = kind;
        }
    }

    public static setBounds(inputElement: HTMLInputElement, x: number, y: number, caretWidth: number, caretHeight: number, caret: number) {
        inputElement.style.left = (x).toFixed(0) + "px";
        inputElement.style.top = (y).toFixed(0) + "px";

        const { left, top } = CaretHelper.getCaretCoordinates(inputElement, caret);

        inputElement.style.left = (x - left).toFixed(0) + "px";
        inputElement.style.top = (y - top).toFixed(0) + "px";
    }

    public static hide(inputElement: HTMLInputElement) {
        inputElement.style.display = "none";
    }

    public static show(inputElement: HTMLInputElement) {
        inputElement.style.display = "block";
    }

    public static setSurroundingText(inputElement: HTMLInputElement, text: string, start: number, end: number) {
        if (!inputElement) {
            return;
        }

        inputElement.value = text;
        inputElement.setSelectionRange(start, end);
        inputElement.style.width = "20px";
        inputElement.style.width = `${inputElement.scrollWidth}px`;
    }

    private static getModifiers(args: KeyboardEvent | PointerEvent | WheelEvent | DragEvent): number {
        let modifiers = RawInputModifiers.None;

        if (args.ctrlKey) { modifiers |= RawInputModifiers.Control; }
        if (args.altKey) { modifiers |= RawInputModifiers.Alt; }
        if (args.shiftKey) { modifiers |= RawInputModifiers.Shift; }
        if (args.metaKey) { modifiers |= RawInputModifiers.Meta; }

        const buttons = (args as PointerEvent).buttons;
        if (buttons) {
            if (buttons & 1) { modifiers |= RawInputModifiers.LeftMouseButton; }
            // Differs from the original, which compares the type of the event ("pointerdown", ...)
            // with "pen" and so never reports the barrel button: the kind of the pointer decides.
            if (buttons & 2) { modifiers |= ((args as PointerEvent).pointerType === "pen" ? RawInputModifiers.PenBarrelButton : RawInputModifiers.RightMouseButton); }
            if (buttons & 4) { modifiers |= RawInputModifiers.MiddleMouseButton; }
            if (buttons & 8) { modifiers |= RawInputModifiers.XButton1MouseButton; }
            if (buttons & 16) { modifiers |= RawInputModifiers.XButton2MouseButton; }
            if (buttons & 32) { modifiers |= RawInputModifiers.PenEraser; }
        }

        return modifiers;
    }

    public static setPointerCapture(containerElement: HTMLInputElement, pointerId: number): void {
        containerElement.setPointerCapture(pointerId);
    }

    public static releasePointerCapture(containerElement: HTMLInputElement, pointerId: number): void {
        if (containerElement.hasPointerCapture(pointerId)) {
            containerElement.releasePointerCapture(pointerId);
        }
    }
}

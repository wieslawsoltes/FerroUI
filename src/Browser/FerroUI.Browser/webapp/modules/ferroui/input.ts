import { CaretHelper } from "./caretHelper";
import { FerroExports } from "./ferroExports";
import { StorageItem } from "../storage/storageItem";
import { applyTextInputOptions } from "./inputOptions";
import { ViewBox, viewBoxOf, viewPoint } from "./pointerPosition";
import { readTextSelection, setTextInputState as writeTextInputState, subscribeTextInput, TextInputElement, writeTextSelection } from "./textInput";

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

interface WriteableClipboardItem {
    data: Record<string, string | Blob>;
}

interface WriteableClipboardSource {
    items: WriteableClipboardItem[];
}

interface ClipboardResult {
    error?: string;
    result?: ReadableDataItem[];
}

interface ClipboardPaste {
    keyDown: KeyboardEvent;
    promise: Promise<ClipboardResult>;
    complete: (result: ClipboardResult) => void;
}

// Differs from the original: an item of a drag operation also holds the data transfer of its
// event, whose getData reads string values while the event is dispatched.
type ReadableDataItem = {
    type: "clipboardItem";
    value: ClipboardItem;
} | {
    type: "dataTransferItem";
    value: DataTransferItem;
    dataTransfer?: DataTransfer;
} | {
    type: "string";
    value: string;
    // Not in the original: the format of a string read from a paste event (text/plain when absent).
    format?: string;
};

type ReadableDataValue = {
    type: "string";
    value: string;
} | {
    type: "bytes";
    value: Uint8Array;
} | {
    type: "file";
    value: StorageItem;
};

// Not in the original, which stamps a pointer event with the time the framework handles it: the
// milliseconds since the browser raised the event. The page handles input between the frames it
// draws, and a frame that takes long made a moving finger look as if it had stopped.
const eventAge = (args: Event): number => performance.now() - args.timeStamp;

// The box a pointer move was measured on, for its coalesced points (getCoalescedEvents).
const moveBoxes = new WeakMap<PointerEvent, ViewBox>();

// Not in the original: the hidden textarea of a client of the exact-edit connection (dom.ts).
const textInputElementClass = "ferroui-text-input-element";
interface SurroundingProjection {
    value: string;
    selection: [number, number];
}
const isTextInputElement = (target: EventTarget | null): boolean =>
    (target as Element | null)?.classList?.contains(textInputElementClass) === true;

export class InputHelper {
    private static clipboardHandlerInstalled = false;
    private static pasteKeyDown?: KeyboardEvent;
    private static clipboardPaste?: ClipboardPaste;
    private static readonly textInputResets = new WeakMap<EventTarget, () => void>();
    private static readonly surroundingProjections = new WeakMap<HTMLInputElement, SurroundingProjection>();

    public static initializeBackgroundHandlers() {
        if (this.clipboardHandlerInstalled) {
            return;
        }

        globalThis.document.addEventListener("paste", args => {
            const paste = this.clipboardPaste;
            if (!paste || args.target !== paste.keyDown.target) {
                return;
            }

            // Differs from the original, which resolves with the items of the event: they are read
            // after the event, when the browser no longer gives their values (getAsString never
            // calls back), so the read never completed. The string values are read now; a file
            // stays an item of the event.
            const clipboardData = args.clipboardData;
            const items: ReadableDataItem[] = this.getDataTransferItems(clipboardData).map((item) => item.kind === "string" && clipboardData != null
                ? { type: "string", value: clipboardData.getData(item.type), format: item.type }
                : { type: "dataTransferItem", value: item });
            args.preventDefault();
            this.completeClipboardPaste(paste, { result: items });
        });
        this.clipboardHandlerInstalled = true;
    }

    private static getDataTransferItems(dataTransfer?: DataTransfer | null): DataTransferItem[] {
        const dataTransferList = dataTransfer?.items;
        return dataTransferList == null ? [] : Array.from(dataTransferList);
    }

    public static isClipboardFormatSupported(format: string): boolean {
        // Some browsers have no ClipboardItem (Firefox before 127): the formats below are assumed.
        const supports = typeof ClipboardItem === "undefined"
            ? undefined
            : (ClipboardItem as any).supports as ((type: string) => boolean) | undefined;
        if (supports) {
            return supports.call(ClipboardItem, format);
        }

        return format === "text/plain" || format === "text/html" || format === "image/png";
    }

    public static createWriteableClipboardSource(): WriteableClipboardSource {
        return { items: [] };
    }

    public static createWriteableClipboardItem(source: WriteableClipboardSource): WriteableClipboardItem {
        const item = { data: {} };
        source.items.push(item);
        return item;
    }

    public static addStringToWriteableClipboardItem(item: WriteableClipboardItem, format: string, value: string) {
        item.data[format] = value;
    }

    // The bytes are a view over the memory of the module, valid only during the call: they are
    // copied.
    public static addBytesToWriteableClipboardItem(item: WriteableClipboardItem, format: string, value: Uint8Array) {
        const bytes = value.slice(0, value.byteLength);
        const mimeType = format.startsWith("web ") ? format.slice(4) : format;
        item.data[format] = new Blob([bytes], { type: mimeType });
    }

    public static async readClipboard(window: Window): Promise<ClipboardResult> {
        const clipboard = window.navigator.clipboard;

        try {
            if (clipboard?.read) {
                const clipboardItems = await clipboard.read();
                const result: ClipboardResult = { result: clipboardItems.map((item) => ({ type: "clipboardItem", value: item })) };
                return result;
            } else if (clipboard?.readText) {
                const item: ReadableDataItem = {
                    type: "string",
                    value: await clipboard.readText()
                };
                const result: ClipboardResult = { result: [item] };
                return result;
            }
            return await this.readClipboardFromPaste(window);
        } catch (ex: unknown) {
            if (ex instanceof Error && ex.name === "NotAllowedError") {
                const result: ClipboardResult = { error: "denied" };
                return result;
            }
            throw ex;
        }
    }

    private static readClipboardFromPaste(window: Window): Promise<ClipboardResult> | ClipboardResult {
        const keyDown = this.pasteKeyDown;
        if (!keyDown) {
            return { error: "denied" };
        }
        if (this.clipboardPaste?.keyDown === keyDown) {
            return this.clipboardPaste.promise;
        }
        if (this.clipboardPaste) {
            this.completeClipboardPaste(this.clipboardPaste, { error: "denied" });
        }
        let complete!: (result: ClipboardResult) => void;
        const promise = new Promise<ClipboardResult>(resolve => { complete = resolve; });
        const paste = { keyDown, promise, complete };
        this.clipboardPaste = paste;
        // A browser's default paste follows the keydown. Release the request
        // on the next task if the browser cancels it or focus moves away.
        window.setTimeout(() => this.completeClipboardPaste(paste, { error: "denied" }), 0);
        return promise;
    }

    private static completeClipboardPaste(paste: ClipboardPaste, result: ClipboardResult): void {
        if (this.clipboardPaste === paste) {
            this.clipboardPaste = undefined;
            paste.complete(result);
        }
    }

    public static async writeClipboard(window: Window, source?: WriteableClipboardSource | null): Promise<string> {
        try {
            const items = source?.items ?? [];
            if (items.length === 0) {
                await window.navigator.clipboard.writeText("");
                return "";
            }

            if (window.navigator.clipboard.write) {
                await window.navigator.clipboard.write(items.map(item => new ClipboardItem(item.data)));
            } else {
                await this.writeFirstText(window, items);
            }

            return "";
        } catch (error: unknown) {
            if (error instanceof Error && error.name === "NotAllowedError") {
                return "denied";
            }
            throw error;
        }
    }

    private static async writeFirstText(window: Window, items: WriteableClipboardItem[]): Promise<void> {
        for (const item of items) {
            for (const format in item.data) {
                if (!format.startsWith("text/")) {
                    continue;
                }

                let value = item.data[format];
                if (typeof value !== "string") {
                    value = "";
                }

                await window.navigator.clipboard.writeText(value);
                return;
            }
        }
    }

    public static getReadableDataItemFormats(item: ReadableDataItem): readonly string[] {
        /* eslint-disable indent */
        switch (item.type) {
            case "clipboardItem":
                return item.value.types;
            case "dataTransferItem":
                switch (item.value.kind) {
                    case "string":
                        return [item.value.type];
                    case "file":
                        return ["Files"];
                    default:
                        return [];
                }
            case "string":
                return [item.format ?? "text/plain"];
            default:
                return [];
        }
        /* eslint-enable indent */
    }

    // Asynchronous, used to read the clipboard.
    public static async tryGetReadableDataItemValueAsync(item: ReadableDataItem, format: string): Promise<ReadableDataValue | null> {
        const type = item.type;

        /* eslint-disable indent */
        switch (type) {
            case "clipboardItem": {
                const clipboardItem = item.value;
                if (!clipboardItem.types.includes(format)) {
                    return null;
                }

                const blob = await clipboardItem.getType(format);

                return format.startsWith("text/")
                    ? { type: "string", value: await blob.text() }
                    : { type: "bytes", value: await this.getBlobBytes(blob) };
            }

            case "dataTransferItem": {
                const dataTransferItem = item.value;

                switch (dataTransferItem.kind) {
                    case "string": {
                        if (format !== dataTransferItem.type) {
                            return null;
                        }

                        const stringValue = await new Promise<string>((resolve) => dataTransferItem.getAsString((str) => resolve(str)));
                        return { type: "string", value: stringValue };
                    }

                    case "file": {
                        if (format !== "Files") {
                            return null;
                        }

                        const file = dataTransferItem.getAsFile();
                        return file == null ? null : { type: "file", value: StorageItem.createFromFile(file) };
                    }

                    default:
                        return null;
                }
            }

            case "string": {
                if (item.format !== undefined && format !== item.format) {
                    return null;
                }

                return format.startsWith("text/")
                    ? { type: "string", value: item.value }
                    : { type: "bytes", value: await this.getBlobBytes(new Blob([item.value])) };
            }

            default:
                return null;
        }
        /* eslint-enable indent */
    }

    // Synchronous, used only to read a drag-and-drop item.
    public static tryGetReadableDataItemValue(item: ReadableDataItem, format: string): ReadableDataValue | null {
        const type = item.type;

        if (type !== "dataTransferItem") {
            return null;
        }

        const dataTransferItem = item.value;

        /* eslint-disable indent */
        switch (dataTransferItem.kind) {
            case "string": {
                if (format !== dataTransferItem.type) {
                    return null;
                }

                // Differs from the original, which reads the value with getAsString: its callback
                // runs after the event, so the value read here was always empty. getData of the
                // data transfer of the event answers synchronously (during a drop; in the other
                // events of the operation the browser protects the values and answers "").
                let stringValue = "";
                if (item.dataTransfer !== undefined) {
                    stringValue = item.dataTransfer.getData(format);
                } else {
                    dataTransferItem.getAsString(function (str) { stringValue = str; });
                }
                return { type: "string", value: stringValue };
            }

            case "file": {
                if (format !== "Files") {
                    return null;
                }

                const file = dataTransferItem.getAsFile();
                return file == null ? null : { type: "file", value: StorageItem.createFromFile(file) };
            }

            default:
                return null;
        }
        /* eslint-enable indent */
    }

    private static async getBlobBytes(blob: Blob): Promise<Uint8Array> {
        const bytes = (blob as any).bytes as (() => Promise<Uint8Array>) | undefined;
        return bytes
            ? await bytes.call(blob)
            : new Uint8Array(await blob.arrayBuffer());
    }

    public static subscribeInputEvents(element: HTMLInputElement, topLevelId: number) {
        const keySub = this.subscribeKeyEvents(element, topLevelId);
        const pointerSub = this.subscribePointerEvents(element, topLevelId);
        const textSub = this.subscribeTextEvents(element, topLevelId);
        const dndSub = this.subscribeDropEvents(element, topLevelId);
        const paneSub = this.subscribeKeyboardGeometryChange(element, topLevelId);
        // Not in the original, which never tells the framework that the view lost the focus.
        const focusSub = this.subscribeFocusEvents(element, topLevelId);

        return () => {
            keySub();
            pointerSub();
            textSub();
            dndSub();
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
    //
    // One exception to 2: a key down whose handling started a read of the clipboard through the
    // paste event (browsers without the asynchronous Clipboard API) keeps its
    // default action, which is the paste.
    private static shouldPreventDefault(args: KeyboardEvent, handled: boolean): boolean {
        if (this.clipboardPaste?.keyDown === args) {
            return false;
        }

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

    private static readonly handledDeleteKeys = new WeakSet<EventTarget>();

    public static subscribeKeyEvents(element: HTMLInputElement, topLevelId: number) {
        // Differs from the original, which awaits the answer of the framework and prevents the
        // default action afterwards: the answer is synchronous, so that the decision is made while
        // the event is still being dispatched. See shouldPreventDefault for the policy.
        const keyDownHandler = (args: KeyboardEvent) => {
            const previous = this.pasteKeyDown;
            const command = args.ctrlKey || args.metaKey;
            const pasteShortcut = !args.isComposing && !args.altKey &&
                ((command && args.key.toLowerCase() === "v") || (!command && args.shiftKey && args.key === "Insert"));
            this.pasteKeyDown = pasteShortcut ? args : undefined;
            try {
                const handled: boolean =
                    FerroExports.InputHelper?.OnKeyDown(topLevelId, args.code, args.key, this.getModifiers(args)) ?? false;
                // A browser that reports a delete key with the input method code keeps its
                // default action, which arrives as a delete input event after this key has
                // already deleted.
                if (handled && !args.isComposing && args.keyCode === 229 &&
                    (args.key === "Backspace" || args.key === "Delete")) {
                    this.handledDeleteKeys.add(element);
                } else {
                    this.handledDeleteKeys.delete(element);
                }
                if (this.shouldPreventDefault(args, handled)) {
                    args.preventDefault();
                }
            } finally {
                this.pasteKeyDown = previous;
            }
        };
        element.addEventListener("keydown", keyDownHandler);

        const keyUpHandler = (args: KeyboardEvent) => {
            this.handledDeleteKeys.delete(element);
            const handled: boolean =
                FerroExports.InputHelper?.OnKeyUp(topLevelId, args.code, args.key, this.getModifiers(args)) ?? false;
            if (this.shouldPreventDefault(args, handled)) {
                args.preventDefault();
            }

            const paste = this.clipboardPaste;
            if (paste && paste.keyDown.code === args.code && paste.keyDown.target === args.target) {
                this.completeClipboardPaste(paste, { error: "denied" });
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
        let composing = false;
        const legacyInput = element.querySelector?.<HTMLInputElement>(":scope > .ferroui-input-element");
        const isLegacyEdit = (args: InputEvent) => legacyInput != null && args.target === legacyInput &&
            legacyInput.matches(":focus") && !composing && !args.isComposing;
        const isStandaloneDelete = (args: InputEvent) => isLegacyEdit(args) &&
            (args.inputType === "deleteContentBackward" || args.inputType === "deleteContentForward");
        // Undefined means the browser sent no beforeinput. Null marks a
        // beforeinput whose selection or owner was superseded.
        let pendingReplacement: { projection: SurroundingProjection; selection: [number, number] } | null | undefined;
        const invalidateReplacement = () => {
            if (pendingReplacement !== undefined) pendingReplacement = null;
        };
        const standaloneDelete = (args: InputEvent) => FerroExports.InputHelper?.OnStandaloneDelete?.(
            topLevelId, args.inputType === "deleteContentBackward") ?? false;
        let compositionCommit: string | undefined;
        let compositionCommitTimer: ReturnType<typeof setTimeout> | undefined;
        const clearCompositionCommit = () => {
            compositionCommit = undefined;
            if (compositionCommitTimer !== undefined) {
                clearTimeout(compositionCommitTimer);
                compositionCommitTimer = undefined;
            }
        };
        const resetComposition = () => {
            composing = false;
            clearCompositionCommit();
            invalidateReplacement();
        };
        this.textInputResets.set(element, resetComposition);
        legacyInput?.addEventListener("blur", invalidateReplacement);

        // Not in the original: the events of the textarea of a connection client bubble here too.
        // They belong to its own protocol (subscribeConnectionTextEvents), never to this path.
        const compositionStartHandler = (args: CompositionEvent) => {
            if (isTextInputElement(args.target)) return;
            clearCompositionCommit();
            invalidateReplacement();
            composing = true;
            FerroExports.InputHelper?.OnCompositionStart(topLevelId);
        };
        element.addEventListener("compositionstart", compositionStartHandler);

        const beforeInputHandler = (args: InputEvent) => {
            if (isTextInputElement(args.target)) return;
            if (legacyInput && isLegacyEdit(args) && args.inputType === "insertReplacementText") {
                const projection = this.surroundingProjections.get(legacyInput);
                pendingReplacement = projection && projection.value === legacyInput.value && !args.defaultPrevented
                    ? { projection, selection: readTextSelection(legacyInput) }
                    : null;
                return;
            }
            if (isStandaloneDelete(args) && this.handledDeleteKeys.has(element)) {
                if (args.cancelable) {
                    args.preventDefault();
                    this.handledDeleteKeys.delete(element);
                }
                return;
            }
            if (isStandaloneDelete(args)) {
                // Real keydown already prevents the browser default when it
                // handles the key. For a DOM-only edit, prefer the cancelable
                // event so deletion works at the edge of the proxy text too.
                if (args.cancelable && !args.defaultPrevented && standaloneDelete(args)) args.preventDefault();
                return;
            }
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

        // Physical keys are already delivered synchronously by OnKeyDown and
        // their DOM default is prevented. Emoji pickers, dictation and virtual
        // keyboards can instead send insertText without a keydown. Forward the
        // insertion, never the input's potentially stale surrounding value.
        const inputHandler = (event: Event) => {
            const args = event as InputEvent;
            if (isTextInputElement(args.target)) return;
            if (typeof args.inputType !== "string" || composing || args.isComposing || args.inputType.includes("Composition")) {
                return;
            }
            const isCompositionCommit = compositionCommit !== undefined && compositionCommit === args.data;
            clearCompositionCommit();
            const replacement = pendingReplacement;
            pendingReplacement = undefined;
            if (isStandaloneDelete(args) && this.handledDeleteKeys.delete(element)) {
                // The key has deleted: the proxy goes back to what the framework published.
                const current = legacyInput ? this.surroundingProjections.get(legacyInput) : undefined;
                if (legacyInput && current) {
                    if (legacyInput.value !== current.value) legacyInput.value = current.value;
                    writeTextSelection(legacyInput, ...current.selection);
                }
            } else if (isStandaloneDelete(args)) {
                standaloneDelete(args);
            } else if (legacyInput && isLegacyEdit(args) && args.inputType === "insertReplacementText") {
                const projection = this.surroundingProjections.get(legacyInput);
                if (projection && replacement !== null && (!replacement || replacement.projection === projection) && !isCompositionCommit) {
                    FerroExports.InputHelper?.OnReplacementText?.(topLevelId, projection.value, ...projection.selection,
                        ...(replacement?.selection ?? [-1, -1]), legacyInput.value, ...readTextSelection(legacyInput));
                }
                // An accepted edit publishes a new native projection. Restore
                // the latest projection even if the edit was blocked or stale.
                const current = this.surroundingProjections.get(legacyInput);
                if (current) {
                    if (legacyInput.value !== current.value) legacyInput.value = current.value;
                    writeTextSelection(legacyInput, ...current.selection);
                }
            } else if (args.inputType === "insertText" && !isCompositionCommit && args.data) {
                FerroExports.InputHelper?.OnTextInput(topLevelId, args.data);
            }
        };
        element.addEventListener("input", inputHandler);

        const textKeyDownHandler = (args: KeyboardEvent) => {
            if (!args.isComposing && !isTextInputElement(args.target)) {
                clearCompositionCommit();
                invalidateReplacement();
            }
        };
        element.addEventListener("keydown", textKeyDownHandler);
        element.addEventListener("blur", clearCompositionCommit);

        const compositionUpdateHandler = (args: CompositionEvent) => {
            if (composing && !isTextInputElement(args.target)) {
                FerroExports.InputHelper?.OnCompositionUpdate(topLevelId, args.data);
            }
        };
        element.addEventListener("compositionupdate", compositionUpdateHandler);

        const compositionEndHandler = (args: CompositionEvent) => {
            if (isTextInputElement(args.target)) return;
            const wasComposing = composing;
            composing = false;
            clearCompositionCommit();
            if (wasComposing) {
                FerroExports.InputHelper?.OnCompositionEnd(topLevelId, args.data);
            }
            // Some browsers send a final non-composing insertText in the same
            // event turn. An active end owns its commit; a canceled end and its
            // trailing input must both stay out of a replacement native client.
            // Set the guard after native dispatch, which can itself move focus.
            // Expire it so a later identical insertion remains independent.
            if (args.data) {
                compositionCommit = args.data;
                compositionCommitTimer = setTimeout(clearCompositionCommit, 0);
            }
            args.preventDefault();
        };
        element.addEventListener("compositionend", compositionEndHandler);

        const textInput = element.querySelector?.<TextInputElement>(`:scope > .${textInputElementClass}`) ?? null;
        const textInputSub = textInput ? this.subscribeConnectionTextEvents(textInput, topLevelId) : () => { };

        return () => {
            textInputSub();
            clearCompositionCommit();
            if (this.textInputResets.get(element) === resetComposition) {
                this.textInputResets.delete(element);
            }
            element.removeEventListener("compositionstart", compositionStartHandler);
            // Differs from the original, which leaves the "beforeinput" listener in place.
            element.removeEventListener("beforeinput", beforeInputHandler);
            element.removeEventListener("input", inputHandler);
            element.removeEventListener("keydown", textKeyDownHandler);
            element.removeEventListener("blur", clearCompositionCommit);
            legacyInput?.removeEventListener("blur", invalidateReplacement);
            element.removeEventListener("compositionupdate", compositionUpdateHandler);
            element.removeEventListener("compositionend", compositionEndHandler);
        };
    }

    // Not in the original: the protocol of the textarea of a client of the exact-edit connection
    // (textInput.ts; docs/porting/browser-platform.md, section 14).
    public static subscribeConnectionTextEvents(input: TextInputElement, topLevelId: number) {
        return subscribeTextInput(input, {
            beforeInput: (revision, type, value, anchor, active) =>
                FerroExports.InputHelper?.OnConnectionBeforeInput(topLevelId, revision, type, value, anchor, active) ?? 0,
            input: (revision, token, type, value, anchor, active) =>
                FerroExports.InputHelper?.OnConnectionInput(topLevelId, revision, token, type, value, anchor, active),
            compositionStart: (revision, value, anchor, active) =>
                FerroExports.InputHelper?.OnConnectionCompositionStart(topLevelId, revision, value, anchor, active) ?? 0,
            compositionUpdate: (token, data) =>
                FerroExports.InputHelper?.OnConnectionCompositionUpdate(topLevelId, token, data),
            compositionEnd: (token, data, value, anchor, active) =>
                FerroExports.InputHelper?.OnConnectionCompositionEnd(topLevelId, token, data, value, anchor, active),
            // A read through the paste event owns the event for its input.
            ownsPaste: () => this.clipboardPaste?.keyDown.target === input
        });
    }

    public static subscribePointerEvents(
        element: HTMLInputElement,
        topLevelId: number
    ) {
        // Not in the original, which takes the events of the native controls (elements in the
        // native host of the view) as events of the view: with their coordinates, which are relative
        // to the native control, and with the implicit capture of a press, which moves the release
        // to the element of the view so that the native control never gets its click. Events whose
        // target is inside a native control are left to it; while the view captures the pointer the
        // target is the element of the view.
        const isForNativeControl = (args: Event) => {
            const target = args.target;
            if (!(target instanceof Element) || target === element) {
                return false;
            }
            const nativeHost = target.closest(".ferroui-native-host");
            return nativeHost !== null && nativeHost !== target && element.contains(nativeHost);
        };

        // Differs from the original, which passes `offsetX` and `offsetY`: those are relative to the
        // target of the event, and the target changes with the capture of the pointer. Every
        // position is measured on the canvas instead (pointerPosition.ts).
        const positionOf = (args: MouseEvent) => viewPoint(viewBoxOf(element), args.clientX, args.clientY);

        const pointerMoveHandler = (args: PointerEvent) => {
            if (isForNativeControl(args)) {
                return;
            }
            const box = viewBoxOf(element);
            moveBoxes.set(args, box);
            const [x, y] = viewPoint(box, args.clientX, args.clientY);
            FerroExports.InputHelper?.OnPointerMove(
                topLevelId, args.pointerType, args.pointerId, x, y,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args), eventAge(args), args);
            args.preventDefault();
        };

        const pointerDownHandler = (args: PointerEvent) => {
            if (isForNativeControl(args)) {
                return;
            }
            const [x, y] = positionOf(args);
            FerroExports.InputHelper?.OnPointerDown(
                topLevelId, args.pointerType, args.pointerId, args.button, x, y,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args), eventAge(args));
            args.preventDefault();
        };

        const pointerUpHandler = (args: PointerEvent) => {
            if (isForNativeControl(args)) {
                return;
            }
            const [x, y] = positionOf(args);
            FerroExports.InputHelper?.OnPointerUp(
                topLevelId, args.pointerType, args.pointerId, args.button, x, y,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args), eventAge(args));
            args.preventDefault();
        };

        const pointerCancelHandler = (args: PointerEvent) => {
            if (isForNativeControl(args)) {
                return;
            }
            const [x, y] = positionOf(args);
            FerroExports.InputHelper?.OnPointerCancel(
                topLevelId, args.pointerType, args.pointerId, x, y,
                args.pressure, args.tiltX, args.tiltY, args.twist ?? 0, this.getModifiers(args), eventAge(args));
        };

        // Differs from the original, which does not pass the unit of the deltas: a browser reports
        // them in pixels, lines or pages (deltaMode), and the framework converts them.
        const wheelHandler = (args: WheelEvent) => {
            if (isForNativeControl(args)) {
                return;
            }
            const [x, y] = positionOf(args);
            FerroExports.InputHelper?.OnWheel(
                topLevelId, x, y, args.deltaX, args.deltaY, args.deltaMode, this.getModifiers(args));
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

    public static subscribeDropEvents(
        element: HTMLInputElement,
        topLevelId: number
    ) {
        const handler = (args: DragEvent) => {
            const dataTransfer = args.dataTransfer;
            if (dataTransfer == null) {
                return;
            }

            const items: ReadableDataItem[] =
                this.getDataTransferItems(dataTransfer).map((item) => ({ type: "dataTransferItem", value: item, dataTransfer }));

            // Measured on the canvas, as the pointer events are (pointerPosition.ts).
            const [x, y] = viewPoint(viewBoxOf(element), args.clientX, args.clientY);
            FerroExports.InputHelper?.OnDragDrop(topLevelId, args.type, x, y, this.getModifiers(args), dataTransfer, items);
        };
        const overAndDropHandler = (args: DragEvent) => {
            args.preventDefault();
            handler(args);
        };
        element.addEventListener("dragover", overAndDropHandler);
        element.addEventListener("dragenter", handler);
        element.addEventListener("dragleave", handler);
        element.addEventListener("drop", overAndDropHandler);

        return () => {
            element.removeEventListener("dragover", overAndDropHandler);
            element.removeEventListener("dragenter", handler);
            element.removeEventListener("dragleave", handler);
            element.removeEventListener("drop", overAndDropHandler);
        };
    }

    // Differs from the original, which passes `offsetX` and `offsetY` (see subscribePointerEvents):
    // the points are measured on the box the move itself was measured on.
    public static getCoalescedEvents(pointerEvent: PointerEvent): number[] {
        const box = moveBoxes.get(pointerEvent);
        return pointerEvent.getCoalescedEvents()
            .flatMap(e => {
                const [x, y] = box != null ? viewPoint(box, e.clientX, e.clientY) : [e.offsetX, e.offsetY];
                return [x, y, e.pressure, e.tiltX, e.tiltY, e.twist ?? 0];
            });
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
        // Native reset/client changes clear the hidden input. Text events are
        // subscribed on its containing view, and no DOM blur is required when
        // focus moves between two native TextBoxes using that same input.
        for (let target: Element | null = inputElement; target; target = target.parentElement) {
            const reset = this.textInputResets.get(target);
            if (reset) {
                reset();
                break;
            }
        }
        this.surroundingProjections.delete(inputElement);
        inputElement.value = "";
    }

    // Not in the original: the textarea of a connection client beside the input element.
    public static getTextInputElement(inputElement: HTMLElement): TextInputElement | null {
        return inputElement.parentElement?.querySelector<TextInputElement>(`:scope > .${textInputElementClass}`) ?? null;
    }

    public static clearTextInput(input: TextInputElement, revision: number) {
        writeTextInputState(input, "", 0, 0, revision);
    }

    public static setTextInputState(input: TextInputElement, text: string, anchor: number, active: number, revision: number) {
        if (!input) {
            return;
        }

        writeTextInputState(input, text, anchor, active, revision);
        input.style.width = "20px";
        input.style.width = `${input.scrollWidth}px`;
    }

    public static setInputOptions(input: TextInputElement, inputMode: string, enterKeyHint: string,
        capitalization: string, suggestions: boolean, sensitive: boolean, language: string): void {
        applyTextInputOptions(input, inputMode, enterKeyHint, capitalization, suggestions, sensitive, language);
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

        // A single-line input strips line breaks from its value. Preserve
        // one proxy character for every UTF-16 unit in a line delimiter.
        const projected = text.replace(/[\r\n]/g, " ");
        if (inputElement.value !== projected) inputElement.value = projected;
        writeTextSelection(inputElement, start, end);
        this.surroundingProjections.set(inputElement, { value: projected, selection: readTextSelection(inputElement) });
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

// The persistent textarea is a proxy for the framework's text document. Its
// native input events describe edits; the framework remains authoritative.
// https://www.w3.org/TR/input-events-2/#interface-InputEvent
export type TextInputElement = HTMLTextAreaElement;

export interface TextInputCallbacks {
    beforeInput: (revision: number, inputType: string, value: string, anchor: number, active: number) => number;
    input: (revision: number, token: number, inputType: string, value: string, anchor: number, active: number) => void;
    compositionStart: (revision: number, value: string, anchor: number, active: number) => number;
    compositionUpdate: (token: number, data: string) => void;
    compositionEnd: (token: number, data: string, value: string, anchor: number, active: number) => void;
    ownsPaste: () => boolean;
}

interface TextState {
    revision: number;
    value: string;
    anchor: number;
    active: number;
    pending?: { revision: number; token: number };
    composition?: { token: number };
}

const states = new WeakMap<TextInputElement, TextState>();

function stateFor(input: TextInputElement): TextState {
    let state = states.get(input);
    if (!state) {
        state = { revision: 0, value: "", anchor: 0, active: 0 };
        states.set(input, state);
    }
    return state;
}

function selection(input: TextInputElement): [number, number] {
    return input.selectionDirection === "backward"
        ? [input.selectionEnd, input.selectionStart]
        : [input.selectionStart, input.selectionEnd];
}

function writeText(input: TextInputElement, value: string, anchor: number, active: number): void {
    // Rust supplies the LF projection and matching DOM UTF-16 offsets; it
    // retains the exact model line endings and translates edits back there.
    // Normalize defensively before comparing the textarea's API value.
    value = value.replace(/\r\n?/g, "\n");
    if (input.value !== value) input.value = value;
    const start = Math.max(0, Math.min(anchor, active, input.value.length));
    const end = Math.max(0, Math.min(Math.max(anchor, active), input.value.length));
    const direction = anchor === active ? "none" : anchor > active ? "backward" : "forward";
    if (input.selectionStart !== start || input.selectionEnd !== end ||
        (start !== end && input.selectionDirection !== direction)) {
        input.setSelectionRange(start, end, direction);
    }
}

function restoreText(input: TextInputElement, state: TextState): void {
    writeText(input, state.value, state.anchor, state.active);
}

/** Publishes one LF-projected framework snapshot without replacing the element. */
export function setTextInputState(input: TextInputElement, text: string,
    anchor: number, active: number, revision: number): void {
    const state = stateFor(input);
    // A mirror between beforeinput and input invalidates that pending edit.
    // Retain a rejected marker: clearing it would let the old input acquire
    // this newer revision through the no-beforeinput fallback.
    if (state.pending) state.pending = { revision: 0, token: 0 };
    writeText(input, text, anchor, active);
    state.value = input.value;
    [state.anchor, state.active] = selection(input);
    state.revision = revision;
    // Keep the native composition's captured token until its end event. A
    // focus/reset update invalidates that token in Rust; a late end event must
    // not acquire the new client's ownership merely because this node survives.
}

function isCompositionInput(args: InputEvent): boolean {
    return args.isComposing || args.inputType === "insertCompositionText" ||
        args.inputType === "deleteCompositionText" || args.inputType === "deleteByComposition" ||
        args.inputType === "insertFromComposition";
}

/** Subscribes only to this proxy, leaving hosted native controls independent. */
export function subscribeTextInput(input: TextInputElement, callbacks: TextInputCallbacks): () => void {
    const state = stateFor(input);
    const beforeInput = (args: InputEvent): void => {
        if (args.target !== input) return;
        if (args.defaultPrevented) {
            state.pending = { revision: 0, token: 0 };
            return;
        }
        if (state.composition !== undefined || isCompositionInput(args)) {
            state.pending = undefined;
            return;
        }
        const revision = state.revision;
        const [anchor, active] = selection(input);
        const token = callbacks.beforeInput(revision, args.inputType ?? "", input.value, anchor, active);
        const accepted = token !== 0 && state.revision === revision;
        // Revision zero records a rejected beforeinput even if it cannot be
        // canceled. It must not become the no-beforeinput fallback on input.
        state.pending = { revision: accepted ? revision : 0, token: accepted ? token : 0 };
        if (!accepted && args.cancelable) args.preventDefault();
    };
    const onInput = (event: Event): void => {
        const args = event as InputEvent;
        if (args.target !== input) return;
        if (state.composition !== undefined || isCompositionInput(args)) {
            state.pending = undefined;
            // Some browsers send their final composition input after the end
            // event. The end callback has already committed or rejected it.
            if (!state.composition) restoreText(input, state);
            return;
        }
        const pending = state.pending;
        state.pending = undefined;
        const revision = state.revision;
        const [anchor, active] = selection(input);
        callbacks.input(pending?.revision ?? revision, pending?.token ?? 0,
            args.inputType ?? "", input.value, anchor, active);
        // A missing runtime, rejected callback or no-op still leaves the DOM
        // matching the last authoritative snapshot. Successful edits publish a
        // new revision synchronously; never overwrite that newer result.
        if (state.revision === revision && !state.composition) restoreText(input, state);
    };
    const compositionStart = (args: CompositionEvent): void => {
        if (args.target !== input) return;
        state.pending = undefined;
        const composition = { token: 0 };
        state.composition = composition;
        const [anchor, active] = selection(input);
        const token = callbacks.compositionStart(state.revision, input.value, anchor, active);
        if (state.composition === composition) composition.token = token;
    };
    const compositionUpdate = (args: CompositionEvent): void => {
        if (args.target !== input) return;
        const token = state.composition?.token;
        if (token) callbacks.compositionUpdate(token, args.data);
    };
    const compositionEnd = (args: CompositionEvent): void => {
        if (args.target !== input) return;
        const token = state.composition?.token;
        state.composition = undefined;
        state.pending = undefined;
        const [anchor, active] = selection(input);
        if (token) callbacks.compositionEnd(token, args.data, input.value, anchor, active);
        if (!state.composition) restoreText(input, state);
    };
    const paste = (args: ClipboardEvent): void => {
        if (args.target === input && callbacks.ownsPaste()) args.preventDefault();
    };

    input.addEventListener("beforeinput", beforeInput);
    input.addEventListener("input", onInput);
    input.addEventListener("compositionstart", compositionStart);
    input.addEventListener("compositionupdate", compositionUpdate);
    input.addEventListener("compositionend", compositionEnd);
    input.addEventListener("paste", paste);
    return () => {
        input.removeEventListener("beforeinput", beforeInput);
        input.removeEventListener("input", onInput);
        input.removeEventListener("compositionstart", compositionStart);
        input.removeEventListener("compositionupdate", compositionUpdate);
        input.removeEventListener("compositionend", compositionEnd);
        input.removeEventListener("paste", paste);
        state.pending = undefined;
        state.composition = undefined;
    };
}

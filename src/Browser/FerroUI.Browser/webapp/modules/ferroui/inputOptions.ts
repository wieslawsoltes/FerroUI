// InputScope attributes advise the IME on the same persistent textarea, keeping
// its UTF-16 selection and native composition intact.
export function applyTextInputOptions(
    input: HTMLElement,
    inputMode: string,
    enterKeyHint: string,
    capitalization: string,
    suggestions: boolean,
    sensitive: boolean,
    language: string
): void {
    const set = (name: string, value: string | null): void => {
        if (input.getAttribute(name) === value) return;
        if (value === null) input.removeAttribute(name);
        else input.setAttribute(name, value);
    };
    set("inputmode", inputMode);
    set("enterkeyhint", enterKeyHint);
    set("autocapitalize", capitalization);
    set("spellcheck", suggestions && !sensitive ? "true" : "false");
    set("autocorrect", suggestions && !sensitive ? "on" : "off");
    set("writingsuggestions", suggestions && !sensitive ? "true" : "false");
    // This element proxies framework content; browser form autofill is not a
    // document edit and must not inject saved credentials into the hidden input.
    set("autocomplete", "off");
    set("lang", language || null);
}

// The hidden input of a masked field is a password input, which browsers keep out of spelling,
// suggestion and autofill services. Changing the type keeps the element, its focus and its value.
export function applyMaskedInput(input: HTMLInputElement, masked: boolean): void {
    const set = (name: string, value: string | null): void => {
        if (input.getAttribute(name) === value) return;
        if (value === null) input.removeAttribute(name);
        else input.setAttribute(name, value);
    };
    set("spellcheck", "false");
    set("autocorrect", masked ? "off" : null);
    set("autocapitalize", masked ? "off" : "none");
    set("autocomplete", masked ? "new-password" : null);
    input.type = masked ? "password" : "text";
}

// The input handler forwards `insertText` and then mirrors the mask again. Any other insertion
// (a paste, a drop, a replacement) is not forwarded, so in a masked field it would stay in the
// element as real text.
export function isUnforwardedMaskedInsertion(elementType: string, inputType: string): boolean {
    return elementType === "password" &&
        inputType.startsWith("insert") &&
        inputType !== "insertText" &&
        !inputType.includes("Composition");
}

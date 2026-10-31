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

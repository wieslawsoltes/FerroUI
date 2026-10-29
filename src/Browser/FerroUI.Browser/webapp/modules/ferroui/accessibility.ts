import { FerroExports } from "./ferroExports";

// Not in the original, whose browser backend draws to its canvas only: a screen reader finds nothing
// in the page, and the element picker of the developer tools finds the canvas. The framework mirrors
// the automation tree of a view (its automation peers, the tree UI Automation, AT-SPI and the
// Android access helper read on the other platforms) into the accessibility host of the view, an
// element of no size over the canvas (docs/porting/browser-accessibility.md):
//
// - one element per peer, nested as the peers are, absolutely positioned at the bounds of its peer
//   (relative to its parent element), with the ARIA role, name, states and value of the peer, and
//   the class and automation id of its control as `data-ferroui-type` and `data-ferroui-name`;
// - the elements are transparent and take no pointer events: the canvas keeps every pointer and
//   key event, while a screen reader reads the elements and the element picker finds them;
// - the container of the view, which keeps the keyboard focus, names the element of the focused
//   peer with `aria-activedescendant`;
// - a click on an element (what a screen reader does to activate it) is sent to the framework, which
//   invokes, toggles, selects, expands or focuses its peer.
//
// The framework sends what changed once per animation frame (`requestUpdate`, then `apply` with a
// batch): the elements are created once and only the changed attributes are written.
//
// By default the mirror is built on demand, as Flutter and Uno Platform do on the web: a page cannot
// tell whether a screen reader runs, so the host holds a visually hidden "Enable accessibility" button
// that a screen reader finds first; activating it (or `FerroView::set_accessibility_enabled`, or a
// click on the button from the console of the developer tools) builds the mirror.

/** A change of one element: its id, then only the fields that changed. */
interface NodeChange {
    i: number;
    // Removed, with its descendants.
    d?: 1;
    // The parent element (0: the host) and the index among the elements of the parent.
    p?: number;
    x?: number;
    // ARIA role ("" for none), name, description, text content (the value or the text of a text
    // element), placeholder.
    r?: string;
    n?: string;
    h?: string;
    t?: string;
    ph?: string;
    // The bounds, relative to the parent element, in CSS pixels.
    b?: [number, number, number, number];
    // The ARIA states: each a string, or "" to remove the attribute.
    s?: Record<string, string>;
    // The class name and the automation id of the control.
    ct?: string;
    ci?: string;
}

interface Batch {
    // The changes in tree order: a parent comes before its children.
    c: NodeChange[];
    // The focused element (0 for none), when it changed.
    f?: number;
}

class AccessibilityState {
    readonly elements = new Map<number, HTMLElement>();
    frame = 0;

    constructor(
        readonly host: HTMLElement,
        readonly container: HTMLElement,
        readonly topLevelId: number,
        readonly prefix: string
    ) {}
}

const STATE = Symbol("ferroui-accessibility");
type HostWithState = HTMLElement & { [STATE]?: AccessibilityState };

const ELEMENT_STYLE =
    "position:absolute;box-sizing:border-box;margin:0;padding:0;border:0;overflow:hidden;" +
    "color:transparent;background:transparent;pointer-events:none;white-space:pre;" +
    "font-size:1px;line-height:1px;outline:none";

export class FerroAccessibility {
    // Makes `host` the accessibility host of the view in `container`, whose top-level is
    // `topLevelId`: clicks on its elements go to the framework.
    public static attach(host: HTMLElement, container: HTMLElement, topLevelId: number): void {
        const prefix = `ferroui-a11y-${container.getAttribute("data-containerId") ?? topLevelId}-`;
        const state = new AccessibilityState(host, container, topLevelId, prefix);
        (host as HostWithState)[STATE] = state;
        host.addEventListener("click", (event) => {
            const target = event.target;
            if (!(target instanceof HTMLElement)) {
                return;
            }
            if (target.classList.contains("ferroui-enable-accessibility")) {
                event.preventDefault();
                event.stopPropagation();
                FerroExports.AccessibilityHelper?.OnAction(state.topLevelId, 0, "enable");
                return;
            }
            const id = Number(target.dataset.ferrouiId);
            if (id > 0) {
                event.preventDefault();
                event.stopPropagation();
                FerroExports.AccessibilityHelper?.OnAction(state.topLevelId, id, "click");
            }
        });
    }

    // Asks for one call of the framework at the next animation frame, when it has changes to send.
    public static requestUpdate(host: HTMLElement): void {
        const state = (host as HostWithState)[STATE];
        if (!state || state.frame !== 0) {
            return;
        }
        state.frame = requestAnimationFrame(() => {
            state.frame = 0;
            FerroExports.AccessibilityHelper?.OnFrame(state.topLevelId);
        });
    }

    // Applies a batch of changes (JSON, see `Batch`).
    public static apply(host: HTMLElement, json: string): void {
        const state = (host as HostWithState)[STATE];
        if (!state) {
            return;
        }
        const batch = JSON.parse(json) as Batch;
        // The container keeps the keyboard focus and names the focused element: a widget that
        // handles its own keys, for which `aria-activedescendant` is read.
        if (state.container.getAttribute("role") !== "application") {
            state.container.setAttribute("role", "application");
        }
        for (const change of batch.c) {
            FerroAccessibility.applyChange(state, change);
        }
        if (batch.f !== undefined) {
            if (batch.f > 0 && state.elements.has(batch.f)) {
                state.container.setAttribute("aria-activedescendant", state.prefix + String(batch.f));
            } else {
                state.container.removeAttribute("aria-activedescendant");
            }
        }
    }

    // Shows or removes the visually hidden button that turns the mirror on (built on demand).
    public static setEnableButton(host: HTMLElement, shown: boolean): void {
        const state = (host as HostWithState)[STATE];
        if (!state) {
            return;
        }
        const existing = host.querySelector(":scope > .ferroui-enable-accessibility");
        if (!shown) {
            existing?.remove();
            return;
        }
        if (existing) {
            return;
        }
        const button = document.createElement("button");
        button.type = "button";
        button.id = `${state.prefix}enable`;
        button.className = "ferroui-enable-accessibility";
        button.tabIndex = -1;
        button.textContent = "Enable accessibility";
        // Visually hidden and out of the Tab order (Tab stays the view's): a screen reader reaches it
        // with its reading cursor, which finds it first, and activates it.
        button.style.cssText =
            "position:absolute;left:0;top:0;width:1px;height:1px;margin:-1px;padding:0;border:0;" +
            "overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap";
        host.prepend(button);
    }

    // Removes every element and the focus reference: the mirror is turned off.
    public static clear(host: HTMLElement): void {
        const state = (host as HostWithState)[STATE];
        if (!state) {
            return;
        }
        if (state.frame !== 0) {
            cancelAnimationFrame(state.frame);
            state.frame = 0;
        }
        state.elements.clear();
        for (const child of Array.from(host.children)) {
            if (!child.classList.contains("ferroui-enable-accessibility")) {
                child.remove();
            }
        }
        state.container.removeAttribute("aria-activedescendant");
        state.container.removeAttribute("role");
    }

    private static applyChange(state: AccessibilityState, change: NodeChange): void {
        if (change.d === 1) {
            const element = state.elements.get(change.i);
            if (element) {
                element.remove();
                FerroAccessibility.forget(state, element);
            }
            return;
        }

        let element = state.elements.get(change.i);
        if (!element) {
            element = document.createElement("div");
            element.id = state.prefix + String(change.i);
            element.dataset.ferrouiId = String(change.i);
            element.style.cssText = ELEMENT_STYLE;
            // The text of the element: always its first node, before the elements of its children.
            element.appendChild(document.createTextNode(""));
            state.elements.set(change.i, element);
        }

        if (change.p !== undefined && change.x !== undefined) {
            const parent = change.p === 0 ? state.host : state.elements.get(change.p);
            if (parent) {
                // The enable button, while shown, is the first element of the host.
                const offset = change.p === 0 && state.host.firstElementChild?.classList.contains("ferroui-enable-accessibility") === true ? 1 : 0;
                const before = parent.children[change.x + offset] ?? null;
                if (element.parentElement !== parent || before !== element) {
                    parent.insertBefore(element, before);
                }
            }
        }
        if (change.r !== undefined) {
            FerroAccessibility.setAttribute(element, "role", change.r);
        }
        if (change.n !== undefined) {
            FerroAccessibility.setAttribute(element, "aria-label", change.n);
        }
        if (change.h !== undefined) {
            FerroAccessibility.setAttribute(element, "aria-description", change.h);
        }
        if (change.ph !== undefined) {
            FerroAccessibility.setAttribute(element, "aria-placeholder", change.ph);
        }
        if (change.t !== undefined) {
            const text = element.firstChild;
            if (text && text.nodeType === Node.TEXT_NODE) {
                text.nodeValue = change.t;
            }
        }
        if (change.b !== undefined) {
            const style = element.style;
            style.left = `${change.b[0]}px`;
            style.top = `${change.b[1]}px`;
            style.width = `${change.b[2]}px`;
            style.height = `${change.b[3]}px`;
        }
        if (change.s !== undefined) {
            for (const [name, value] of Object.entries(change.s)) {
                FerroAccessibility.setAttribute(element, name, value);
            }
        }
        if (change.ct !== undefined) {
            FerroAccessibility.setAttribute(element, "data-ferroui-type", change.ct);
        }
        if (change.ci !== undefined) {
            FerroAccessibility.setAttribute(element, "data-ferroui-name", change.ci);
        }
    }

    private static forget(state: AccessibilityState, element: HTMLElement): void {
        state.elements.delete(Number(element.dataset.ferrouiId));
        for (const child of Array.from(element.children)) {
            FerroAccessibility.forget(state, child as HTMLElement);
        }
    }

    private static setAttribute(element: HTMLElement, name: string, value: string): void {
        if (value === "") {
            element.removeAttribute(name);
        } else {
            element.setAttribute(name, value);
        }
    }
}

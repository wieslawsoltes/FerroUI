// Not in the original. The position of a pointer in the view, for every pointer, wheel and drag
// event and for the coalesced points of a move (docs/porting/browser-platform.md, section 8).
//
// The original passes `offsetX` and `offsetY` of the event and takes them for logical units of the
// view. That holds only when three things agree: the target of the event has its padding corner at
// the corner of the picture (the target is the canvas when the pointer is not captured and the
// element of the view while it is), the canvas has no border or padding, and the device pixel ratio
// the framework renders at is the ratio of the canvas's backing size to its CSS size. Here the
// position is measured from where the picture is: the client coordinates of the event, less the
// corner of the canvas's content box, converted with the scale of what is drawn there.

// What the position is computed from, measured once per event.
export interface ViewBox {
    // The corner of the canvas's border box in client coordinates.
    left: number;
    top: number;
    // How many client pixels one CSS pixel of the canvas covers (a transform or `zoom` of an
    // ancestor makes it differ from 1).
    scaleX: number;
    scaleY: number;
    // Border plus padding of the canvas, left and top, in its own CSS pixels.
    insetLeft: number;
    insetTop: number;
    // Logical units of the view per CSS pixel of the content box.
    unitsX: number;
    unitsY: number;
}

// The computed style properties the box needs (a live `CSSStyleDeclaration` in the page).
export interface BoxStyle {
    boxSizing: string;
    width: string;
    height: string;
    borderLeftWidth: string;
    borderRightWidth: string;
    borderTopWidth: string;
    borderBottomWidth: string;
    paddingLeft: string;
    paddingRight: string;
    paddingTop: string;
    paddingBottom: string;
}

export interface ClientRect {
    left: number;
    top: number;
    width: number;
    height: number;
}

// The size the framework renders the view at: the backing size of the canvas in device pixels and
// the scaling (device pixels per logical unit); the client size of the view is their quotient.
export interface RenderSize {
    width: number;
    height: number;
    scaling: number;
}

const px = (value: string): number => {
    const parsed = parseFloat(value);
    return Number.isFinite(parsed) ? parsed : 0;
};

const ratio = (numerator: number, denominator: number): number =>
    numerator > 0 && denominator > 0 ? numerator / denominator : 1;

// The box of a canvas with bounding rectangle `rect`, computed style `style`, rendered at `render`.
// Pure: the page passes `getBoundingClientRect()` and `getComputedStyle()` of the canvas.
export function measureViewBox(rect: ClientRect, style: BoxStyle, render: RenderSize): ViewBox {
    const borderX = px(style.borderLeftWidth) + px(style.borderRightWidth);
    const borderY = px(style.borderTopWidth) + px(style.borderBottomWidth);
    const paddingX = px(style.paddingLeft) + px(style.paddingRight);
    const paddingY = px(style.paddingTop) + px(style.paddingBottom);
    let contentWidth = px(style.width);
    let contentHeight = px(style.height);
    if (style.boxSizing === "border-box") {
        contentWidth -= borderX + paddingX;
        contentHeight -= borderY + paddingY;
    }
    contentWidth = Math.max(0, contentWidth);
    contentHeight = Math.max(0, contentHeight);
    const scaling = render.scaling > 0 ? render.scaling : 1;
    return {
        left: rect.left,
        top: rect.top,
        scaleX: ratio(rect.width, contentWidth + borderX + paddingX),
        scaleY: ratio(rect.height, contentHeight + borderY + paddingY),
        insetLeft: px(style.borderLeftWidth) + px(style.paddingLeft),
        insetTop: px(style.borderTopWidth) + px(style.paddingTop),
        // The backing store is stretched over the content box, so a CSS pixel there shows
        // (backing size / CSS size) device pixels, which are (1 / scaling) logical units each.
        unitsX: ratio(render.width / scaling, contentWidth),
        unitsY: ratio(render.height / scaling, contentHeight)
    };
}

// The position in logical units of the view of the client point (`clientX`, `clientY`).
export function viewPoint(box: ViewBox, clientX: number, clientY: number): [number, number] {
    return [
        ((clientX - box.left) / box.scaleX - box.insetLeft) * box.unitsX,
        ((clientY - box.top) / box.scaleY - box.insetTop) * box.unitsY
    ];
}

// The canvas of each view element and the size it is rendered at, registered by the canvas surface.
export interface ViewCanvas {
    readonly canvas: HTMLCanvasElement;
    readonly width: number;
    readonly height: number;
    readonly scaling: number;
}

const viewCanvases = new WeakMap<Element, { view: ViewCanvas; style: CSSStyleDeclaration }>();

export function setViewCanvas(container: Element, view: ViewCanvas): void {
    // The computed style is live: it is read once per event, not looked up again.
    viewCanvases.set(container, { view, style: getComputedStyle(view.canvas) });
}

export function clearViewCanvas(container: Element, view: ViewCanvas): void {
    if (viewCanvases.get(container)?.view === view) {
        viewCanvases.delete(container);
    }
}

// The box of the view of `container` as it is now. A view without a canvas (none created yet)
// takes the box of its element, at one logical unit per CSS pixel.
export function viewBoxOf(container: Element): ViewBox {
    const entry = viewCanvases.get(container);
    if (entry == null) {
        const rect = container.getBoundingClientRect();
        return { left: rect.left, top: rect.top, scaleX: 1, scaleY: 1, insetLeft: 0, insetTop: 0, unitsX: 1, unitsY: 1 };
    }
    const { view, style } = entry;
    return measureViewBox(view.canvas.getBoundingClientRect(), style, view);
}

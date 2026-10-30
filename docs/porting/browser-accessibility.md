# Accessibility tree of the browser backend

The browser backend draws a view into a canvas. Without more, a screen reader finds nothing in the page, the element picker of the browser's developer tools finds only the canvas, and browser automation cannot find a control by its role or name. The upstream browser backend has no accessibility tree; this is an addition of FerroUI.

FerroUI mirrors the automation tree of a view into the page. That is the tree of automation peers that UI Automation (Windows), AT-SPI (Linux), the native bridge (macOS) and the access helper (Android) read on the other platforms. Canvas-rendered frameworks on the web do the same: Flutter's semantics tree, Uno Platform's semantic DOM (`#uno-semantics-root`) and Qt for WebAssembly build hidden HTML elements with ARIA roles from their accessibility trees.

## What is in the page

Each view has an accessibility host: the element `div.ferroui-accessibility-host` in the container of the view. Like the native control host it has no size of its own; its elements are positioned from its corner, which is the corner of the view. In paint order it lies over the canvas and under the native controls.

The host holds one `div` per automation peer, nested as the peers are. Each element has:

| Of the peer | In the element |
|---|---|
| The automation control type, or the landmark type of a container | `role` (see the mapping in `browser_accessibility.rs`); a text has no role and is read by its text |
| The name (`AutomationProperties.Name`, or the text of the content) | `aria-label`; for a text, its text content |
| The value of a text box or combo box | its text content |
| The help text, the placeholder | `aria-description`, `aria-placeholder` |
| Toggle state, expand/collapse state, selection, read-only, the range value, the heading level, the live setting, enabled | `aria-checked` (`aria-pressed` for a toggle button), `aria-expanded`, `aria-selected`, `aria-readonly`, `aria-valuemin`/`max`/`now`, `aria-level`, `aria-live`, `aria-disabled` |
| The class name and the automation id of the control | `data-ferroui-type`, `data-ferroui-name` |
| The bounding rectangle | `left`, `top`, `width`, `height` relative to the parent element |

The elements are transparent and take no pointer events. The canvas keeps every pointer and key event, and nothing about input changes. The element picker of the developer tools finds the elements anyway: it hit-tests through `pointer-events: none`. It highlights the control under the pointer, and the Elements panel shows the tree of controls with their types and names.

The container of the view keeps the keyboard focus, as before. With the tree on, it has `role="application"`, and `aria-activedescendant` names the element of the focused peer. A screen reader announces the focused control as the focus moves.

A screen reader activates an element by clicking it. The framework then invokes the peer (`IInvokeProvider`), toggles it (`IToggleProvider`), selects it (`ISelectionItemProvider`), expands or collapses it (`IExpandCollapseProvider`), or focuses it, in that order of preference.

## On demand, always or never

A page cannot tell whether a screen reader runs. By default the tree is built on demand, as Flutter and Uno Platform do on the web. The host holds a visually hidden "Enable accessibility" button (`button.ferroui-enable-accessibility`). It is out of the Tab order, so Tab stays the view's, and it is the first element of the view, so a screen reader's reading cursor finds it first. Activating it builds the tree; until then nothing is followed and nothing costs time.

Applications choose with `BrowserPlatformOptions::accessibility`:

```rust
use ferroui_browser::{BrowserAccessibilityMode, BrowserPlatformOptions};

let options = BrowserPlatformOptions {
    accessibility: BrowserAccessibilityMode::Enabled, // or OnDemand (the default), or Disabled
    ..Default::default()
};
```

At any time, for one view:

```rust
view.set_accessibility_enabled(true);       // FerroView
// or, from the top-level of a control, without the view:
if let Some(tree) = ferroui_browser::BrowserAccessibility::of(&top_level) {
    tree.set_enabled(false);
}
```

From the console of the developer tools, the tree of a view built on demand is turned on with:

```js
document.querySelector(".ferroui-enable-accessibility").click()
```

## Cost

The tree is kept up to date by what changed, once per animation frame:

- A peer whose children changed (`children_changed`) has its subtree walked again. The rest of the tree is not walked.
- A peer whose properties changed (`property_changed`) has its attributes read again.
- A layout pass of the top-level reads the bounding rectangles of the known peers, and nothing else.
- The framework diffs each element against what the page has and sends only the changed fields, in one JSON batch per frame. The page creates each element once and writes only the changed attributes.
- Peers that leave the tree are released with their subscriptions; nothing is kept alive by the mirror.

With the tree off (`OnDemand` before activation, or `Disabled`) nothing is subscribed, and the only cost is one hidden button.

## Tests

- `browser_accessibility_tests.rs`: the role mapping, the attributes of real control peers (button, check box, text box, text block) and the diff of the batch (only changed fields, removed states, JSON escaping).
- `scripts/browser/tests/themed_view.test.mjs`, "the accessibility tree is built on demand and mirrors the controls of the view", in headless Chrome:
  - Before activation only the enable button is in the page, and it is out of the Tab order.
  - After activation, the roles, names, states and values of the button, check box, text box, slider, progress bar, list and text are there, and the elements lie over their controls.
  - The elements take no pointer events: a click on the canvas still reaches the button.
  - A click on an element invokes the button and toggles the check box, and the state follows.
  - The container names the focused text box.
  - Turned off, the elements and the focus reference are removed and the button returns.

## Limits

- Text editing goes through the hidden input element of the view as before; the text box element mirrors the value but is not itself editable.
- The DOM focus stays on the container (`aria-activedescendant`). A mode that moves the DOM focus to the elements, as Flutter does, would suit screen readers that handle `aria-activedescendant` poorly.
- Bounds follow layout passes. A control moved by a render transform alone, with no layout pass, keeps the bounds of the last pass.

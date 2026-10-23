# Read-only rich-text context menus

`RichTextBlock` projects a configured context-menu request into a synchronous,
typed `ContextMenuOpening` event and then uses FerroUI's existing menu services.
`RichTextBlockOverflow` resolves its own flyout first and otherwise uses the
root master's flyout, while retaining the requesting overflow as the placement
target. This checkpoint covers those configured invocations. Native default
command-bar construction, automatic selection flyouts and the legacy Copy menu
remain separate unfinished contracts.

## Pinned source evidence

All source below is from WinUI commit
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).
The table identifies the particular behavior reviewed; it does not claim that
every member in these files is implemented.

| Source | Reviewed contract |
|---|---|
| [`RichTextBlock.cpp`, `FireContextMenuOpeningEventSynchronously`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlock.cpp#L3607-L3625) | Transform the supplied local point to the root, express it in DIPs, raise the peer event synchronously and return the shared handled state. |
| [`TextControlFlyoutHelper.cpp`, `ShowAt`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/dxaml/lib/TextControlFlyoutHelper.cpp#L151-L185) | Raise the opening event for TextBoxBase, TextBlock or RichTextBlock before showing the configured flyout; stop when handled. There is no RichTextBlockOverflow event branch. |
| [`TextControlHelper.h`, `OnContextMenuOpeningHandler`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/dxaml/lib/TextControlHelper.h#L11-L43) | Set coordinate fields, initialize Handled, invoke the typed event source and read Handled back from the same argument object. |
| [`uielement.cpp`, `OnContextRequestedCore`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/core/elements/uielement.cpp#L12781-L12885) | Use the requesting element's effective flyout. Pointer requests supply a point; read-only keyboard requests retain local `(0, 0)`. The selected-edge fallback belongs to editable controls. Default text flyouts require text selection to be enabled. |
| [`RichTextBlockOverflow.cpp`, `GetContextFlyout`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L1574-L1588) | Prefer a non-null local flyout, then the root master's flyout. An intermediate overflow's local flyout is not the master's flyout. |
| [`TextCore.cpp`, `IsTextControl` and `IsTextSelectionEnabled`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/common/TextCore.cpp#L200-L232) | Include overflow views in text-control dispatch; resolve an overflow's selection-enabled policy from its master. |
| [`DependencyProperty.cpp`, default ContextFlyout value](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/components/DependencyObject/DependencyProperty.cpp#L960-L979) | Supply the built-in default context flyout to TextBlock and RichTextBlock. The switch does not supply it to RichTextBlockOverflow. Default command construction is not implemented by this checkpoint. |
| [`PropertySystem.cpp`, `IsPropertyDefault`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/components/DependencyObject/PropertySystem.cpp#L2827-L2850) | For sparse properties, default status depends on local-assignment provenance. An explicit null and clearing a property are observably different. |
| [`TextSelectionManager.cpp`, `NotifyContextMenuOpening`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/common/TextSelectionManager.cpp#L1722-L1764) | The separate legacy path targets the text container owner, only without an owner context flyout, and enables Copy only for nonempty selection. This does not justify forwarding the modern overflow event to the master. |
| [`TextControlFlyoutHelper.cpp`, `CloseIfOpen` and `ShowAt`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/dxaml/lib/TextControlFlyoutHelper.cpp#L359-L377) | Native repeated invocation closes and reopens to refresh commands. FerroUI's retained-popup placement and actual context-key toggle remain explicit framework translations. |

## Public event and framework integration

[`rich_text_context_menu.rs`](../../../src/FerroUI.Controls/rich_text_context_menu.rs)
uses the existing `ContextMenuEventArgs`: `CursorLeft` and `CursorTop` are
read-only root DIP values, and inherited `Handled` is shared by every handler.
The `RichTextBlock.ContextMenuOpening` event has its own typed registration,
distinct from `RichEditBox.ContextMenuOpening`. It is exposed through the
existing object metadata and runtime and compiled XAML event handling.

FerroUI bubble routing and weak code-behind subscriptions are framework
translations. The native helper invokes its peer event source directly. The
port retains the useful observable contract: handlers run synchronously before
the associated configured menu opens, and setting Handled cancels that
invocation. A handled incoming `ContextRequested` is not projected again.

The editable presenter derives from `RichTextBlock` to share text layout.
It explicitly defers this event to its `RichEditBox` owner, so the inheritance
choice does not add a second public event or replace the editor's selected-edge
keyboard anchor. An explicitly configured presenter-local flyout can consume
the ordinary Control request without either typed event; after clearing that
override, routing reaches the editor and raises only its event. Likewise, a
nested rich reader owns its request before an enclosing reader can project
another event.

An invocation with neither an eligible configured flyout nor a framework
`ContextMenu` does not manufacture a typed event. It still retires an older
in-flight invocation of the same view. Automatic menus will need their own
command, selection and lifetime contracts before they can create invocations.

### Coordinates and placement

Pointer coordinates already use FerroUI root DIPs. A pointerless read-only
request uses the requesting view's local origin transformed to that root,
independently of selection direction or selected text. Native rasterization
scale division is not repeated because FerroUI's visual coordinates are
already DIPs.

The accepted root point is carried as a zero-size placement rectangle through
the existing `ContextRequestedEventArgs` weak placement-owner mechanism.
`ContextMenu` and `PopupFlyoutBase` keep their existing positioning, RTL gravity,
screen constraints, popup host and feedback services. The control does not
create a second popup implementation or change the configured flyout's
`Placement` property to encode a one-off request.

Coordinate conversion is a callback boundary: an application-supplied
`ITransform::value` can synchronously issue another request or detach a view.
Request ownership must therefore be checked after coordinate capture as well
as after typed event handlers. A stale request must not publish an additional
typed event or overwrite a newer accepted popup through later anchor
conversion. The regression record identifies the concrete guarded boundaries;
it is not a claim that arbitrary application transforms are pure.

### Choosing the accepted menu

The framework `ContextMenu` property takes precedence over `ContextFlyout`.
Typed opening handlers may replace or remove either property. Dispatch reads
the accepted choice after those handlers, then invokes that one shared menu
path. If `ContextMenu.Opening` cancels and removes its property, the request is
consumed; it cannot fall through and open the flyout instead. A canceled
flyout opening is likewise not retried by an already snapshotted property
handler or an ancestor's menu handler.

This `ContextMenu` alternative is a FerroUI integration policy. It is not a
claim that the WinUI source has the same property or precedence rule.

## Overflow lookup and selection policy

The overflow chain already stores weak continuation links. Effective flyout
lookup walks those links to the primary `RichTextBlock` and reads that master's
flyout. It never treats an intermediate continuation as a new master.

The native master is a layout-resolved cache: link changes reset it through the
continuation chain, and linked block-layout setup repopulates it. This bridge's
root lookup currently follows the live FerroUI chain. Exact native availability
before layout and reset/setup timing remain partial. The public overflow
`ContentSource` getter also needs a separate correction from its existing
immediate-predecessor behavior; this bridge does not infer its public parity
from the private root lookup.

| Requesting overflow state | Effective configured flyout and eligibility |
|---|---|
| Non-null flyout assigned locally | Use the local flyout. Its explicit assignment does not require the master's selection-enabled gate. |
| No local assignment, master has a flyout and selection is enabled | Use the master's flyout. |
| No local assignment, master has a flyout and selection is disabled | Do not open the inherited flyout. The overflow's own selection flag does not override the master's policy. |
| Explicit local null, master has a flyout | Lookup still falls back to the master, but the local property is no longer default; the default-flyout selection gate does not apply. |
| Local value cleared after an explicit assignment | Restore default-property gating and master fallback. |
| No local flyout and no resolved master | No inherited flyout is available. |
| Local framework ContextMenu | Use that menu according to framework precedence, independently of the default-flyout selection gate. |

The eligibility gate applies when opening. Cancellation and dismissal continue
to resolve an already-open inherited flyout even if text selection was disabled
after it opened. They check that the requesting overflow still owns the flyout
target, so a canceled request from an old continuation cannot close another
view's current popup.

For the configured modern path, overflow requests do not raise a typed event
on the overflow or forward one to the root master. They still open at the
overflow, retain that placement target and use its holding feedback policy.
The native legacy selection manager's owner event is a different, still
unfinished path.

## Repeated requests and actual keyboard input

Each programmatic request, including a pointerless repeat while a popup is
open, passes through the typed opening event. An accepted repeat can reposition
the existing owned FerroUI popup. Native WinUI instead closes and reopens it;
exact native popup event order and command refresh behavior remain pending.

An actual configured context-menu key, such as Apps or Shift+F10, retains the
existing FerroUI toggle behavior. It first raises the typed event, then uses
the shared hide path for the owned popup. Typed cancellation prevents Closing;
canceling Closing keeps the current popup and consumes the key. If the typed
handler replaces the menu, the old popup must close successfully before the
replacement dispatches.

A private synchronous scope identifies this exact routed request. It is not a
public event field and does not classify every pointerless request as keyboard
input. Nested programmatic requests do not inherit the toggle. A nested key
request supersedes the older identity, including when it occurs during tunnel
routing; dropping the newer scope never restores an already retired outer
request.

## Shared popup ownership

The bridge reuses `PopupFlyoutBase`, which can invoke application callbacks in
Opening, Opened, Closing, Closed, property notification, measurement, focus and
host changes. A callback may synchronously retarget or reopen the same flyout.
The shared service must retain the latest accepted operation through those
boundaries.

The regression work records three concrete old-code defects: an Opening or
Opened callback can show the flyout on target B before the older target A call
stores its subscriptions; a Closed callback can reopen on B before the older
hide clears the target; and a popup host-change callback can reopen before the
older popup close writes `IsOpen = false`. Removing A or finishing the old
close can then affect B's newer session.

The shared fix uses operation identity and actual subscription ownership,
with lower-level popup host setup and close fences. Temporary opening
subscriptions observe detach/reattach even when the view returns to the same
visual root. An owned close temporarily holds the old target and transient
registrations outside the active session. If the raw popup cancels Closing, or
an `open()` during that Closing retains the same session, the flyout restores
those exact registrations. Once close succeeds, their cleanup removes handlers
from the owner that installed them. A newer accepted show keeps its own
registrations throughout this cleanup.

The real popup closes before the flyout publishes its false open state. This
prevents a property callback from reusing an old host and its old target
subscription. Internal open-state publication suppresses only the expected
value once, so an application's nested opposite-value request still runs.
Popup host setup and transform subscription helpers also check their own state
identity after virtual callbacks; an outer caller's later check cannot undo a
stale write inside one of those helpers.

A stale operation stops without clearing a newer target, publishing an obsolete
open state or installing stale target handlers. After dispatch has begun, a
superseded request is still consumed so the same routed request cannot open an
ancestor's menu. A hide during Opening retires the pending show even though the
popup is not yet open. Canceled hide retains the active session while
superseding an older in-flight intention. Ordinary redundant `Popup.open()`
does not interrupt that popup's initial completion notifications; only an open
inside its currently owned Closing retires the close.

Popup parent assignment has its own operation identity. Removing a logical
parent can legitimately close the popup and advance its open/close generation,
so that generation alone cannot identify ownership of the remaining parent
writes. The helper checks the assignment ticket, actual parent and placement
target after each callback-bearing write. A nested show followed by hide
supersedes the older assignment even when both parent and placement target have
returned to null. A nested assignment to the same parent must still complete
the current templated-parent association even when its parent write is a no-op;
the superseded outer assignment cannot be relied on to finish that work.

These rules are safe synchronous FerroUI service behavior; they are not native
WinUI event-order measurements. Red and green executions, exact source hashes,
commands and results belong in [TESTS.md](TESTS.md) and
[continuation-validation.json](continuation-validation.json). A test definition
or source trace alone is not recorded as a completed runtime result.

## Remaining work

The exact public event remains `partial` in the API map. The checkpoint does not
implement the built-in `TextControlCommandBarContextFlyout`, automatic
`SelectionFlyout` visibility, touch selection grippers, the legacy asynchronous
command-gated Copy menu, or all native helper lifetime and command-refresh
behavior. It does not establish pixel placement equivalence, packaged WinUI
execution, or every platform's native keyboard and holding sequence. Existing
editor event and input-method behavior remain covered by their own tests and
source records.

The source mapping and deviation `RT-I37` describe this bounded read-only
contract. General UIElement and dependency-property source files are included
only for their reviewed text dispatch/default-value seams; no whole-file
completion is inferred from reuse of FerroUI's existing services.

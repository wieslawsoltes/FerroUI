# Linux accessibility: AT-SPI

Stage 3 of the Linux platform (`x11-platform.md`, section 14): the automation peers of an application as objects of the accessibility bus of a FreeDesktop session, which is how a screen reader (Orca) and other assistive technologies read and drive an application on Linux. Upstream: `src/Avalonia.FreeDesktop.AtSpi` (27 source files, 3559 lines, and 17 interface descriptions in `DBusXml/`) with `X11AtSpiAccessibility.cs` of the X11 project (176 lines), at the tracked commit (`TRACKING.md`).

Marks as in `x11-platform.md`: **[V]** verified from sources, **[M]** measured on the development machine (a Mac: builds and tests without a bus), **[VM]** measured in the Ubuntu 24.04 ARM64 virtual machine, **[CI]** shown by the job `x11`.

## 1. Where it lives

A module tree of `ferroui-freedesktop`: `src/FerroUI.FreeDesktop/at_spi/`. Not a crate of its own: upstream's separate project exists to carry its own D-Bus library, the port has none to carry, the module depends on what the crate already depends on (`ferroui-base`, `ferroui-controls`, `zbus`), and its only user is the X11 crate (later Wayland), which depends on the crate already (`CONTINUATION.md`, the hand-over of the X11 project, has the decision). The public surface is two types: `AtSpiServer` (start, add a window, remove a window, dispose) and `AtSpiAccessibilityWatcher`. Everything else is private to the module.

In the X11 crate: `x11_at_spi_accessibility.rs` (the port of `X11AtSpiAccessibility.cs`), `FerroX11Platform::track_window`, `untrack_window` and `at_spi_server`, and the two places of `X11Window` (`show`, `cleanup`) that add and remove the automation root of a window.

## 2. How an application becomes accessible

1. **The platform starts** (`FerroX11Platform::initialize`): `X11AtSpiAccessibility::initialize` starts a task of the UI dispatcher. It waits until the dispatcher reached context-idle priority, at most 100 ms, and then tries to start the server whatever the session says about accessibility ("path A" upstream, the way GTK 4 does it). Only when that fails it reads `org.a11y.Status` (`IsEnabled`, `ScreenReaderEnabled`) through `AtSpiAccessibilityWatcher` and starts the server when one of them is or becomes true; a server started this second way is disposed when both become false.
2. **The accessibility bus is found**: a connection to the session bus asks `org.a11y.Bus` at `/org/a11y/bus` for `GetAddress` (`AtSpiServer::start_async`). On a desktop this name is activated by D-Bus and starts `at-spi-bus-launcher`, which runs a second bus daemon for accessibility. No answer or an empty address means no accessibility: the start fails with a message and is logged.
3. **The connection** to that address is made (blocking, as the other connections of the crate) without an object server (section 4). Its unique name is the name every object reference of the application carries.
4. **The root object and the cache are registered** (section 3), and the tracker of the registry's listeners starts (section 6).
5. **A window is shown**: `X11Window::show` creates the automation peer of its content, takes the root of its automation tree and gives it to `AtSpiServer::add_window`. Windows that were shown before the server ran are remembered by the platform (`track_window`) and added when it starts.
6. **The application is embedded** with the first window: `org.a11y.atspi.Socket.Embed((unique name, /org/a11y/atspi/accessible/root))` on `org.a11y.atspi.Registry` at `/org/a11y/atspi/accessible/root`. From then on the registry lists the application among its children, which is how clients find it. A failed embed is logged and tried again with the next window. Later windows are announced with a `ChildrenChanged` signal of the root.
7. **A client walks the tree.** Nodes are created when they are asked for: a node attaches its children the first time a client asks for them (`ensure_children`), and from then on follows the peer (`ChildrenChanged`, `PropertyChanged`).
8. **A window is closed**: `X11Window::cleanup` removes its root; every node below is released, children before parents, each with a `defunct` state change when somebody listens.

## 3. Objects, paths and interfaces

| Path | Object | Interfaces |
|---|---|---|
| `/org/a11y/atspi/accessible/root` | the application (`ApplicationAtSpiNode`): role application, its children are the windows | `Accessible`, `Application`, `Event.Object` |
| `/org/a11y/atspi/cache` | the cache | `Cache` (`GetItems` answers with no items, so that clients ask the objects) |
| `/org/ferroui/a11y/<n>` | a node (`AtSpiNode`) of an automation peer; `<n>` counts from 1 per server | below |
| `/org/a11y/atspi/null` | no object: the path of a null reference, with an empty connection name | none |

The first, second and fourth are the paths of the AT-SPI specification. The third is the port's own: upstream's prefix carries the name of its project, the port's is `/org/ferroui/a11y` (`DEVIATIONS.md`). Clients take paths from references and never build them, so the prefix is not part of any contract.

The interfaces of a node follow the providers of its peer, as upstream's `BuildAndRegisterHandlers`:

| Interface (`org.a11y.atspi.`) | When | What it answers from |
|---|---|---|
| `Accessible` | always | name (the peer's name, else its class name), description and help text, parent, children, index in parent, role and role name, the state set, attributes (`toolkit`, `explicit-name`, `accelerator-key`, `access-key`, `placeholder-text`), the `LABELLED_BY` relation, the interfaces, locale, automation id |
| `Application` | the peer has `IRootProvider` (a window), and the root | toolkit name (`FerroUI`) and version (the version of the crate), AT-SPI version `2.1`, the identifier the registry sets (`Id`, the one writable property), locale |
| `Component` | always | extents, position and size on the screen, in the window or in the parent; containment; layer (window 7, widget 3); `GrabFocus`; `SetPosition` moves a window; resizing and scrolling answer false |
| `Action` | `IInvokeProvider`, `IToggleProvider`, `IExpandCollapseProvider`, `IScrollProvider` or `ISelectionItemProvider` | the actions `click`, `toggle`, `expand or collapse`, `scroll up`/`down`/`left`/`right`, `select` (for an item that cannot be invoked), in that order |
| `Value` | `IRangeValueProvider` | minimum, maximum, small change, the value (writable, clamped to the range) |
| `Selection` | `ISelectionProvider` | the selected children; select, deselect, select all, clear, through the selection items below the peer |
| `Text` | `IValueProvider` without `IRangeValueProvider` | the value as text: ranges, the text at, before and after an offset by character and word (`StringUtils` of the controls crate), character count. Caret, selections, attributes and extents are constants, as upstream |
| `EditableText` | the same, when the value is not read-only | set, insert, delete through `IValueProvider::set_value`; the clipboard members answer false |
| `Image` | the control type is `Image` | description, extents |
| `Event.Object` | always | signals only (section 6) |
| `Event.Window` | a window | signals only |
| `Collection` | never | Upstream has the handler (`AtSpiCollectionHandler.cs`) and creates it for no node at the tracked commit; the port has the handler and its tests and creates none either |

Every path with an object also answers `org.freedesktop.DBus.Properties` (`Get`, `GetAll`, `Set`) and `org.freedesktop.DBus.Introspectable`; a path without an object answers `Introspect` when registered paths lie below it (so `gdbus introspect --recurse` and `busctl tree` work), and everything else with `UnknownObject`. An interface a path does not have is `UnknownInterface`, a member `UnknownMethod`, a property `UnknownProperty`, arguments of the wrong types `InvalidArgs`, and a provider that refuses (its element is not enabled) `Failed` with its message. A call without an interface is not answered, as upstream.

Roles (`at_spi_node_role_mapping.rs`) and states (`at_spi_node_state_mapping.rs`) are upstream's tables: 43 control types to roles (a button with a toggle provider is a toggle button), and the state set from enabled, offscreen, focusable, focused, the toggle state, the expand/collapse state, selection, the read-only flags, "required for form", and the control type (a window is always `active`, an edit is `single-line`). Offsets of the text interfaces are UTF-16 code units, as upstream's (the indices of its strings); the specification counts characters, which differs for text outside the basic plane.

## 4. Threading: no object server, one message loop

Upstream serves every object from handlers that its D-Bus library calls on the synchronization context of the UI thread. `zbus` calls the methods of exported objects on the thread of the connection and wants them `Send + Sync`; the other services of the crate bridge that with `ui_thread_object.rs`. Here that would mean one exported object per node, added and removed as the tree changes, and the object server has no handler for a whole subtree.

So the accessibility connection has **no object server** (`zbus` then answers no method call by itself; `connection/mod.rs` of 5.19.0: the task that answers is started by the first use of the object server) and `AtSpiConnection` (`at_spi/dbus/connection.rs`) does what upstream's connection class does:

- A `MessageStream` of the connection, created with it, is read by one task of the UI dispatcher (`signal_watch::watch_stream`). Method calls are dispatched by path and interface to the handler registered there (`ObjectTable`); everything else on the stream (replies to the module's own calls, signals of the registry) is left to the proxies.
- Handlers are objects of the UI thread (`Rc`, no locks) and answer synchronously: a handler returns the reply body or an error, and the connection builds the message.
- Replies and signals go through one queue that a task of the dispatcher sends in order, so a state change emitted inside a call arrives after the calls that were answered before it, in the order it happened.
- Registration is synchronous (upstream's `RegisterObjects` is awaited): a node is reachable from the moment it is attached.

The UI thread therefore answers every call of an assistive technology between two of its jobs. A UI thread that is blocked does not answer; the client's call then waits, as with upstream.

## 5. What is ported of upstream's D-Bus library

Upstream's AT-SPI project compiles the sources of `external/Avalonia.DBus` (a submodule; commit `cada3ecfc153a9efbc8d64f530136d737c1a7eec` at the tracked commit) into itself and generates proxies and handlers from `DBusXml/` with a source generator. The port keeps one D-Bus library (`zbus`: wire format, authentication, transports, the connection, proxies, signal streams, match rules, name-owner tracking). Of upstream's library only what the AT-SPI project needs beyond that is ported, as `at_spi/dbus/`:

| Upstream (`external/Avalonia.DBus/src`) | Port (`at_spi/dbus/`) | What |
|---|---|---|
| `DBusConnection.cs`, `DBusConnection.Worker.cs`: `RegisterObjects`, `DispatchMethodCallAsync`, `ReplyMissingHandler`, `HandleVirtualIntrospectionAsync`, `ResolveIntrospectionData`, `SendMessageAsync` | `connection.rs` (`ObjectTable`, `AtSpiConnection`) | objects at paths; the dispatch of a call; the answers for what is not registered; introspection of paths without objects; messages sent in order |
| `BuiltInPropertiesHandler.cs` | `built_in_properties_handler.rs` | `Get`, `GetAll`, `Set` over the handlers of a path, with upstream's error names |
| `BuiltInIntrospectionHandler.cs` | `built_in_introspection_handler.rs` | the introspection document |
| `IDBusInterfaceCallDispatcher`, `DBusException`, and the handler code the generator writes per interface (`DBusSourceGenerator.Handler.cs`: reading arguments, `TryGetProperty`, `GetAllProperties`, `TrySetProperty`, `WriteIntrospectionXml`) | `interface.rs` (`DBusInterface`, `DBusError`, `reply`, `args`), and the `impl DBusInterface` of every handler | A handler's `call` is the generated dispatch written by hand from the same descriptions |
| The generated types of `Types.xml` | `types.rs` | references, rectangles, actions, relations, attribute sets, text ranges, cache items, match rules, as tuples of their wire signature |
| The descriptions `DBusXml/*.xml` as introspection data | `descriptions.rs` | members, signatures, properties and signals of the twelve interfaces served |
| The generated proxies that are called | `proxies.rs` (`#[zbus::proxy]`) | `org.a11y.Bus` (`GetAddress`), `org.a11y.atspi.Socket` (`Embed`), `org.a11y.atspi.Registry` (`GetRegisteredEvents` and its two signals); `org.a11y.Status` through the properties proxy of `zbus` |

Not ported, because `zbus` is that part: the wire reader and writer, the message type, signature inference, the transports, the `libdbus` binding, the worker threads, the proxy generator. The descriptions `DeviceEventController.xml`, `Registry.xml` (beyond the three members), `Socket.xml` (beyond `Embed`) and the event classes of `Event.xml` other than objects and windows describe interfaces upstream neither serves nor calls.

## 6. Events

A node subscribes to its peer when it is attached and translates:

| Peer event | Signal of `org.a11y.atspi.Event.Object` (`siiva{sv}`) |
|---|---|
| children changed | `ChildrenChanged` `add`, 0, a reference to the node itself (upstream's form: clients re-read the children) |
| the name property | `PropertyChange` `accessible-name` with the name |
| the help text property | `PropertyChange` `accessible-description` |
| the value property of the value pattern | `PropertyChange` `accessible-value` with the text |
| the toggle state | `StateChanged` `checked` (1 or 0), then `indeterminate` |
| the expand/collapse state | `StateChanged` `expanded`, then `collapsed` |
| the selection property | `SelectionChanged` |
| the bounding rectangle | `BoundsChanged` |

The server adds: `ChildrenChanged` `add`/`remove` of the root for windows; `StateChanged` `defunct` for a node that is released; `StateChanged` `focused` for the node of the peer the root provider says has the focus; for a window `StateChanged` `active` and `Activate` or `Deactivate` of `org.a11y.atspi.Event.Window` when the platform window is activated or deactivated.

**The filter** (`AtSpiRegistryEventTracker`): nothing is emitted while no client listens. The tracker asks the registry for `GetRegisteredEvents`, follows `EventListenerRegistered` and `EventListenerDeregistered`, and reports listeners when any registered event is `*` or starts with `object:`, `window:` or `focus:` (without regard to case). Until the first answer, and whenever the registry cannot be asked, the server emits everything.

## 7. The input: automation peers and providers

Checked member by member against `src/FerroUI.Controls/automation` (2026-10-10). Everything the module reads exists there; nothing had to be added to the controls crate.

| Read by | Members |
|---|---|
| `AtSpiNode`, `AtSpiServer` | `AutomationPeer`: `get_provider`, `get_children`, `children_changed`, `property_changed`, `get_name`, `get_class_name`, `get_automation_control_type`; `AutomationPropertyChangedEventArgs`: `property`, `new_value`; the identifiers `AutomationElementIdentifiers` (`name_property`, `help_text_property`, `bounding_rectangle_property`), `TogglePatternIdentifiers::toggle_state_property`, `ExpandCollapsePatternIdentifiers::expand_collapse_state_property`, `ValuePatternIdentifiers::value_property`, `SelectionPatternIdentifiers::selection_property` |
| State mapping | `is_enabled`, `is_offscreen`, `is_keyboard_focusable`, `has_keyboard_focus`; `IToggleProvider::toggle_state`; `IExpandCollapseProvider::expand_collapse_state`; `ISelectionItemProvider::is_selected`; `ISelectionProvider::can_select_multiple`; `IValueProvider::is_read_only`; `IRangeValueProvider::is_read_only`; `ControlAutomationPeer::owner` with `AutomationProperties::get_is_required_for_form` |
| `Accessible` | `get_help_text`, `get_automation_id`, `get_labeled_by`, `get_accelerator_key`, `get_access_key`, `get_placeholder_text` |
| `Component`, `Image`, the coordinate helper | `get_bounding_rectangle`, `get_visual_root`, `get_parent`, `set_focus`; `IRootProvider::platform_impl`; `ITopLevelImpl::point_to_screen`, `point_to_client`, `as_window_base_impl`, `as_window_impl`; `IWindowImpl::move_` |
| `Action` | `IInvokeProvider::invoke`; `IToggleProvider::toggle`; `IExpandCollapseProvider::expand`, `collapse`; `IScrollProvider::scroll`, `vertically_scrollable`, `horizontally_scrollable` with `ScrollAmount`; `ISelectionItemProvider::select` |
| `Value` | `IRangeValueProvider::minimum`, `maximum`, `small_change`, `value`, `set_value` |
| `Selection` | `ISelectionProvider::get_selection`; `ISelectionItemProvider::add_to_selection`, `remove_from_selection`, `is_selected` |
| `Text`, `EditableText` | `IValueProvider::value`, `set_value`, `is_read_only`; `StringUtils::previous_word`, `next_word`, `is_start_of_word`, `is_end_of_word` |
| The window node | `IRootProvider::focus_changed`, `get_focus`; `IWindowBaseImpl::activated`, `set_activated`, `deactivated`, `set_deactivated` |
| The X11 crate | `ControlAutomationPeer::create_peer_for_element`, `from_element` (upstream's internal `GetAutomationPeer`), `AutomationPeer::get_automation_root`; `IInputRoot::focus_root` |

## 8. File table

| Upstream file (`src/Avalonia.FreeDesktop.AtSpi`) | Lines | Rust file (`src/FerroUI.FreeDesktop/at_spi`) | Notes |
|---|---:|---|---|
| `ApplicationAccessibleHandler.cs` | 72 | `application_accessible_handler.rs` | |
| `ApplicationAtSpiNode.cs` | 28 | `application_at_spi_node.rs` | the name: the application's, else the file stem of the executable |
| `ApplicationNodeApplicationHandler.cs` | 32 | `application_node_application_handler.rs` | |
| `AtSpiAccessibilityWatcher.cs` | 74 | `at_spi_accessibility_watcher.rs` | the properties proxy of `zbus` |
| `AtSpiCacheHandler.cs` | 20 | `at_spi_cache_handler.rs` | |
| `AtSpiConstants.cs` | 89 | `at_spi_constants.rs` | the node path prefix and the toolkit name are the port's |
| `AtSpiCoordType.cs` | 7 | `at_spi_coord_type.rs` | |
| `AtSpiNode.RoleMapping.cs` | 109 | `at_spi_node_role_mapping.rs` | |
| `AtSpiNode.StateMapping.cs` | 111 | `at_spi_node_state_mapping.rs` | the mapping as a function over the answers of the peer |
| `AtSpiNode.cs` | 383 | `at_spi_node.rs` | registration is synchronous |
| `AtSpiRegistryEventTracker.cs` | 159 | `at_spi_registry_event_tracker.rs` | the owner of the registry's name is followed by the proxy |
| `AtSpiRole.cs` | 126 | `at_spi_role.rs` | |
| `AtSpiServer.cs` | 469 | `at_spi_server.rs` | |
| `AtSpiState.cs` | 51 | `at_spi_state.rs` | |
| `RootAtSpiNode.cs` | 88 | `root_at_spi_node.rs` | a part of the node, not a subclass |
| `Handlers/AtSpiAccessibleHandler.cs` | 172 | `handlers/at_spi_accessible_handler.rs` | |
| `Handlers/AtSpiActionHandler.cs` | 172 | `handlers/at_spi_action_handler.rs` | the action list as a function over what the peer provides |
| `Handlers/AtSpiCollectionHandler.cs` | 272 | `handlers/at_spi_collection_handler.rs` | no node has it, as upstream |
| `Handlers/AtSpiComponentHandler.cs` | 133 | `handlers/at_spi_component_handler.rs` | |
| `Handlers/AtSpiCoordinateHelper.cs` | 80 | `handlers/at_spi_coordinate_helper.rs` | |
| `Handlers/AtSpiEditableTextHandler.cs` | 72 | `handlers/at_spi_editable_text_handler.rs` | |
| `Handlers/AtSpiEventObjectHandler.cs` | 64 | `handlers/at_spi_event_object_handler.rs` | |
| `Handlers/AtSpiEventWindowHandler.cs` | 47 | `handlers/at_spi_event_window_handler.rs` | |
| `Handlers/AtSpiImageHandler.cs` | 48 | `handlers/at_spi_image_handler.rs` | |
| `Handlers/AtSpiSelectionHandler.cs` | 139 | `handlers/at_spi_selection_handler.rs` | |
| `Handlers/AtSpiTextHandler.cs` | 296 | `handlers/at_spi_text_handler.rs` | the text functions over UTF-16 units |
| `Handlers/AtSpiValueHandler.cs` | 46 | `handlers/at_spi_value_handler.rs` | |

All 27 are built. Files of the port without an upstream file of that project: `at_spi/dbus/*` (section 5) and `at_spi/tests.rs`.

## 9. Verification

Three levels, as for the rest of the platform.

1. **Tests without a bus** (`cargo test -p ferroui-freedesktop`, 102 tests of which 37 are of this module) **[M]**: the role table and role names, the state mapping and the state set words, the node paths, the event filter, the status properties, the action list, the text functions (ranges, character, word, before and after), insertion and deletion, value clamping, relative rectangles, and the match rules of the collection interface. Five tests run the server against a peer over a socket pair (`at_spi/tests.rs`), with a window of the mock platform of the controls crate that holds a button, a text box and a check box, and a double of the registry on the other end:
   - the root before any window: role, state, interfaces, properties, `Set` of `Id`, the cache, the four error names, introspection of the root and of a path without an object;
   - the tree of a window: the window is embedded once, is a frame named by its title with the root as parent; the button, the entry and the check box are found by role, with names, states, interfaces and attributes; parent, index and child at an index agree; after the window is removed its paths answer `UnknownObject`;
   - actions and text: `DoAction` clicks the button once and toggles the check box; the text is read, inserted into, deleted from and replaced; extents lie in the window;
   - events: none without a listener; `StateChanged` `checked` and `indeterminate` after a listener registers; `ChildrenChanged` `remove` and `defunct` for every node of a removed window, children first; none after the listener deregisters; a listener registered before the start is heard, and the embedding announces the window.
2. **The same on a private session bus** (`FERROUI_FREEDESKTOP_TEST_BUS=session` under `dbus-run-session`, the tests named `dbus`): the calls and signals go through a bus daemon, with real unique names and a name owner for the registry.
3. **A real X server** (the job `x11`, and the virtual machine): the smoke mode of `examples/x11_window.rs` with `--a11y`. The example is `org.a11y.Bus` and `org.a11y.atspi.Registry` on a private session bus (the accessibility bus is the session bus itself); the application of the same process finds the bus, registers and embeds; a client on a second connection, on a thread of its own, asks the registry for its children, finds the application by its toolkit name and walks the window. Checks: the address was asked for; one root was embedded; the registry lists the application under a unique name; the root is an application of the toolkit with one child; the window is a frame with the title; every object is at `/org/ferroui/a11y/<n>`; the button (name, enabled, focusable, visible, showing) and its action (the click handler ran once); extents (the button inside the window, the window where the platform says it is on the screen); the text box (role, editable, text, character count); `GrabFocus` (the state and the framework agree); the check box before and after its action; the `StateChanged` event at the listener. With `--a11y=real` the services are those of `at-spi2-core` (the bus launcher started by D-Bus activation, the registry daemon by the embed call) and the checks of the double are left out.

`scripts/x11/atspi-tree.sh` reads the tree of an application with `gdbus` alone (address from `org.a11y.Bus`, applications from the registry, then `GetChildren`, `GetRoleName` and `Name` per object): what an assistive technology sees, without this port's code on the reading side.

### Measured

RESULTS

### Not verified

- **A screen reader.** Orca was not run, nor Accerciser or `pyatspi`: nothing here shows what a user hears, how Orca's own heuristics (which roles it announces, what it does with a tree that has no cache items, how it follows focus) take this tree, or whether keyboard navigation announces what it should. The reading side of every check is a D-Bus client that makes the same calls, which shows the protocol and the content of the answers, not the experience.
- Events of text (`TextChanged`, `TextCaretMoved`, `TextSelectionChanged`): upstream emits none at the tracked commit, and the caret offset is always 0. A screen reader cannot follow typing through this interface.
- The key event interfaces (`DeviceEventController`): not served upstream.
- The status watcher against a session that toggles accessibility at run time, and a registry that restarts.

## 10. Deviations

In `DEVIATIONS.md`, "Accessibility over AT-SPI": the path prefix and the toolkit name; the D-Bus library; synchronous registration; the window callbacks; the name-owner watch; disposal.

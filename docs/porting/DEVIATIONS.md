# Deviations from upstream

The port is exact: the same architecture, contracts, members and behaviour as upstream, file by file. This page is the register of every place where the Rust code nonetheless differs from the upstream code it ports. Each entry says what differs, why, and whether a user of the framework can observe it. A deviation that is not here is a bug to fix or to record.

Other pages hold the entries of their own area, and this page does not repeat them:

- `browser-platform.md` section 14: the browser backend and the browser hosts of the samples (host-level differences, upstream quirks not copied).
- `xaml.md`: the markup pipeline and the ahead-of-time compiler.
- `PORTING-GUIDE.md`: the systematic mappings that apply everywhere (class model, nullability, events, generics, naming). An application of those rules is not a deviation and is not listed here.

## How to record a deviation

- Add a row to the table of the area, in the same pull request as the code. The pull request lists the deviation as well, as `CLOUD-WORKERS.md` asks.
- Put a comment at the site in the code, naming the upstream member and the difference, for example: `// Deviation (DEVIATIONS.md, Layout): upstream uses Stopwatch; ...`.
- Classify the entry:
  - **Representation**: Rust needs a different form (ownership, no `unsafe`, no array pools, unsigned lengths) and nothing observable changes.
  - **Behaviour**: something a user of the framework can observe differs: a value, an order, an exception, a log message or a timing.
  - **Missing**: an upstream step the port leaves out. It is either waiting for a member that is not ported yet (a seam) or deliberately left out.
  - **Test**: only a test or its support code differs.
- When a later change removes the difference, move the row to "Corrected divergences" with the pull request that removed it.

## Register

### Layout (`src/FerroUI.Base/layout/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `LayoutManager.GetTimestamp` uses `Stopwatch.GetTimestamp()` (sub-millisecond ticks). | `get_timestamp` in `layout_manager.rs` reads `Dispatcher::current_dispatcher().now()`, which is in milliseconds. | Behaviour | The port times layout with the dispatcher's clock. Durations below 1 ms read as 0 in `LayoutPassTimed` and the layout time graph. | #25 |
| `LayoutManager` calls `Dispatcher.UIThread.VerifyAccess()` in its public entry points. | Not called. | Missing | No reason was recorded. `Dispatcher::verify_access` exists (`threading/dispatcher.rs`), so this can be ported; until then a call from a wrong thread is not caught here. | before #25 |

### Render data (`src/FerroUI.Base/rendering/composition/drawing/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `RenderDataWriter` blits unmanaged payload structs into the stream. | `render_data_writer.rs` and `render_data_reader.rs` encode payloads with `BatchValue`, the little-endian encoding of the batch transport. | Representation | A raw struct copy needs `unsafe`; the stream's format, opcodes and order are upstream's. | #26 |
| `RenderDataWriter` rents its buffer from `ArrayPool<byte>` and uses `int` lengths. | A `Vec<u8>` (initial capacity 256, amortised doubling) and `usize` lengths. | Representation | Rust's standard library has no shared array pool. | #26 |
| `RenderDataStream.Visit` has no default case: an invalid opcode loops forever. | `Visit` panics on `RenderDataOpcode::Invalid`. | Behaviour | A panic stops at the fault instead of hanging. Only reachable with a corrupt stream. | #26 |
| Each payload has a static `Opcode` field. | The associated const `IRenderDataPayload::OPCODE`. | Representation | The Rust form of a per-type constant. The tracking scanner does not match it, so `RenderDataPayloads.cs` shows 35/50 members. | #26 |
| The batch stream is made of 64-byte pooled segments. | No pooled segments. | Representation | The port's batch stream never had them, so `Round_Trip_Spanning_Multiple_Stream_Segments` cannot cross a segment boundary (see the test's header). | #26 |

### Visual tree (`src/FerroUI.Base/visual_tree/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `VisualLocator.Track(Visual, int ancestorLevel, Type?)`: a negative level makes `ElementAtOrDefault` return null. | `VisualLocator::track` takes `ancestor_level: usize`. | Representation | An unsigned level, as `VisualAncestorElementNode` already took; a negative level cannot be passed. | #27 |

### Logical tree (`src/FerroUI.Base/logical_tree/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `LogicalExtensions.GetLogicalChildren` returns the `LogicalChildren` collection itself; `GetLogicalSiblings` enumerates the parent's collection lazily. | `get_logical_children` and `get_logical_siblings` return a snapshot (`Rc<Vec<..>>`) of the collection. | Behaviour | As `get_visual_children` does. A change to the collection after the call is not seen through the result. | #32 |
| `LogicalExtensions.IsLogicalAncestorOf(this ILogical? logical, ILogical? target)` accepts a null receiver and returns false. | `StyledElement::is_logical_ancestor_of(&self, target: Option<&StyledElement>)`. | Representation | The receiver of a Rust method cannot be null; the target stays optional. | #32 |
| `ControlLocator.Track(ILogical, int ancestorLevel, Type?)`: a negative level makes `ElementAtOrDefault` return null. | `ControlLocator::track` takes `ancestor_level: usize`. | Representation | As `VisualLocator::track`; a negative level cannot be passed. | #36 |

### Property system (`src/FerroUI.Base/property_store/`, `src/FerroUI.Base/ferro_object.rs`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `ValueStore` propagates inherited values over `GetInheritanceChildren()` by index up to the count at the start; an index past the end throws. | `FerroObject::for_each_inheritance_child` stops at an index past the end, and skips a child that was dropped without being detached. | Behaviour | The port's list holds weak references, and `inheritance_children` prunes dropped children, which can shorten the list during the loop; upstream's list of strong references cannot shrink that way. | #29 |

### ControlCatalog sample (`samples/ControlCatalog/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `CompositionPage.ButtonThreadSleep` calls `Thread.Sleep(5000)`. | `button_thread_sleep` in `Pages/composition_page.rs` calls `std::thread::sleep` for 5 s. | Behaviour | Ported as upstream has it: the button demonstrates a blocked UI thread. Recorded because the port otherwise rules out blocking waits on the UI thread. | #34 |
| The messages of `CompositionPage.CustomVisualHandler` are four `static readonly object` instances compared by reference. | Four thread-local `Rc<dyn Any>` objects compared with `Rc::ptr_eq`. | Representation | `Rc` is not `Sync`, so it cannot be a `static`; one instance per thread compares the same way on the UI thread. | #34 |

### Tests and test support

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| Upstream's test setup supplies a font manager to `constraint_and_negative_margin`. | The `MockPlatformRenderInterface` of `ferroui-base` has none; the test adds `TextTestScope` to supply one. | Test | Not recorded in #25. | #25 |
| The clipboard tests of the text controls flush only posted sync-context callbacks. | They run every dispatcher job, including the queued layout pass, and their log recorder ignores `Layout`-area messages. | Test | Running the layout pass logs the new layout timing messages, which the recorder would otherwise count. | #25 |
| The `TestRoot` of `Avalonia.UnitTests` has a settable `StylingParent` (the GlobalStyles and application-resource tests of `StyledElementTests` set it). | The `TestRoot` of `ferroui-controls` has none; `StylingRoot` in `styled_element_tests.rs`, a test root with a settable styling parent, takes its place. | Test | No reason recorded for the missing setter; the tests need only the setter. | #37 |
| `StyledElementTests.Resources_Owner_Is_Set` verifies `AddOwner` on a `Mock<IResourceDictionary>`. | `Resources` takes a concrete `ResourceDictionary`, so the test uses `RecordingResourceDictionary`, a derived dictionary that records its owners from the `on_add_owner` hook. | Test | A mock of the interface cannot be assigned to the property. | #37 |

## Corrected divergences

Places where the port did work that upstream does not do, or did it differently, and a later change made it match upstream again. They are kept here so a regression can be recognised. Extra allocations and copies on hot paths belong here too: `PORTING-GUIDE.md` rules out allocations on hot paths that upstream avoids.

| Upstream | What the port did | Corrected by |
|---|---|---|
| `RenderDataStream` records opcodes and payloads into a byte stream. | Recorded a `Vec<RenderDataOp>` of an enum. | #26 |
| `VisualAncestorElementNode` subscribes to `VisualLocator.Track`. | Followed the attachment events of the element itself, and panicked with "Cannot find a Visual to get a visual ancestor." where upstream throws "Cannot find an ILogical to get a visual ancestor.". | #27 |
| `GetLayoutRoot` and `GetLayoutManager` are extension methods of `Visual`. | Declared them on `Layoutable` only. | #27 |
| `ServerCompositionVisual.ServerTreeWalker.Walk` reads `Children.List.Count` and allocates nothing. | Allocated an empty `Rc<Vec>` for every visual without a children collection, every frame (`walker.rs`). | #29 |
| `CompositionTarget.HitTestChildren` and `HitTestFirstCore` index the children from `Children.Count - 1` down. | Copied the children of every visual entered into a new `Vec` (`composition_target.rs`). | #29 |
| `CompositionContainerVisual.OnRootChangedCore` enumerates `Children` in place. | Copied the children of every visual of an attached or detached subtree (`visual.rs`). | #29 |
| `ValueStore` (`OnInheritanceAncestorChanged`, `OnInheritedEffectiveValueChanged`, `OnAncestorInheritedValueChanged` and the other inherited-value loops) indexes `GetInheritanceChildren()` in place. | Copied the inheritance children of every object a change passed through (`value_store.rs`). | #29 |
| `ValueStore.ReevaluateEffectiveValue` and `ReevaluateEffectiveValues` index `_frames` from the last frame. | Copied the frame list for every reevaluation (`value_store.rs`). | #29 |
| `IPseudoClasses.Remove` compares the name in place. | `Classes::remove_pseudo` allocated a `String` of the name for every call (`classes.rs`). | #29 |
| `LogicalAncestorElementNode` subscribes to `ControlLocator.Track`, which finds the ancestor when it is subscribed whether or not the element is attached. | Followed the attachment events of the element itself and reported no ancestor while the element was not attached to a logical tree. | #36 |
| `Classes.AddRange` adds every name of the argument that the collection does not hold yet, duplicates within the argument included. | `Classes::add_range` also dropped a name that appeared earlier in the same argument (`classes.rs`). | #38 |

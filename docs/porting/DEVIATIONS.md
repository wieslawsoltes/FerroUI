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

### Tests and test support

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| Upstream's test setup supplies a font manager to `constraint_and_negative_margin`. | The `MockPlatformRenderInterface` of `ferroui-base` has none; the test adds `TextTestScope` to supply one. | Test | Not recorded in #25. | #25 |
| The clipboard tests of the text controls flush only posted sync-context callbacks. | They run every dispatcher job, including the queued layout pass, and their log recorder ignores `Layout`-area messages. | Test | Running the layout pass logs the new layout timing messages, which the recorder would otherwise count. | #25 |

## Corrected divergences

Places where the port did work that upstream does not do, or did it differently, and a later change made it match upstream again. They are kept here so a regression can be recognised. Extra allocations and copies on hot paths belong here too: `PORTING-GUIDE.md` rules out allocations on hot paths that upstream avoids.

| Upstream | What the port did | Corrected by |
|---|---|---|
| `RenderDataStream` records opcodes and payloads into a byte stream. | Recorded a `Vec<RenderDataOp>` of an enum. | #26 |

# The design of the bindings

FerroUI is consumed from other languages through one C ABI (docs/c-abi.md) and bindings
generated from one language-neutral API model (bindings/README.md). This document is the
design that the model and the ABI follow so that each language can get the fastest and the
most idiomatic binding its FFI allows. The first backend, Osprey (docs/osprey.md), implements
part of it; section 7 says what is done and what is next.

## 1. Two stages

- **One model** (`bindings/model/ferroui-api.json`), built from the metadata of the framework
  (`scripts/ferroui_metadata.py`, shared with the declarative Rust builders) and from the
  generated C header. It holds facts only: classes, properties (value kind, Rust type,
  default, read-only, attached, documentation), enumerations (members and values), routed
  events (argument type, value property, dispatch styles), the ABI functions, the byte layout
  of frames and the stable IDs with their fingerprint.
- **One backend per language** that reads only the model. Everything a language decides
  (casing, reserved words, how a property or an event looks, how errors are reported) is in
  its backend and its override file, never in the model.

## 2. Performance

1. **No lookup and no marshalling per call.** Types, properties and events have stable
   numeric IDs in the model; the library interns the same names in the same order at start
   (`api_ids.rs`), so generated code passes constants. `ferro_api_fingerprint()` lets a
   binding refuse a library built from another model. Values cross as scalars (`double`,
   `int64_t`, `bool`), strings as a pointer and a length (a negative length for a
   NUL-terminated string), objects as 64-bit handles. Each backend uses the fastest FFI path
   of its language:

   | Language | Path |
   | --- | --- |
   | C#, F# | `LibraryImport` with blittable signatures, `SuppressGCTransition` on leaf calls (setters, getters) |
   | Dart | `@Native` with `isLeaf: true` on leaf calls |
   | Java, Kotlin | Panama downcall handles (`Linker.nativeLinker()`), critical for leaf calls |
   | C++, Swift, Zig, Rust | direct calls to the C functions, inlined wrappers |
   | Go | cgo with the UI thread locked (`runtime.LockOSThread`); batching frames matters most here |
   | Python | `ctypes`/`cffi`, or a C extension for the hot paths |
   | Osprey | `extern fn` of scalars, NUL-terminated strings and named functions (#950) |

2. **Frames written straight into memory.** The frame layer (section 4) has a byte layout in
   the model. A language that can write memory fills a buffer and makes one
   `ferro_frame_commit_bytes(root, data, length)` call per update, with no call per element;
   a language that cannot calls the scalar writer (`ferro_frame_open`, `ferro_frame_str`, ...),
   which writes the same records into a buffer of the library, then `ferro_frame_commit`. One
   decoder and one reconcile serve both.
3. **UTF-16 next to UTF-8.** `ferro_set_str16` and the frame record `text16`
   (`ferro_frame_str16`) take UTF-16 code units, so C#, Java and JavaScript pass their strings
   without transcoding them first.
4. **Wrappers are values.** An object wrapper is the handle and nothing else: a `readonly
   struct` in C#, a `struct` in Swift, a `value class` in Kotlin, an inline class in C++. No
   allocation per wrapper; the handle table of the library counts references
   (`ferro_retain`, `ferro_release`), and the framework owns its tree.
5. **One callback per binding, no trampoline.** A binding gives the library one function and
   keeps its handlers in a table on its side: the token (or the frame tag) is the index. No
   native thunk per handler. A binding whose events all go to one dispatcher (Osprey, an
   Elm-like binding) subscribes with `ferro_subscribe_dispatch(object, event, tag)`, which
   calls the dispatcher of `ferro_frame_dispatcher` directly: one indirect call per event and
   no function of the binding in between; a language with closures uses `ferro_subscribe`
   with one function and the token as the index of its table. Nothing is allocated per
   event (the event text of a frame event lives in a buffer the library keeps).
6. **Selection trims both sides.** A manifest of what an application uses (`ferroui.json`:
   classes, whose bases come with them, and layers) selects the part of the model a backend
   generates; the Osprey backend does this for every sample. Trimming the exports and tables of
   the library with the same manifest is phase 2 of #636.
7. **A benchmark per backend.** Each backend ships the same operations as the Rust benchmark
   of the ABI (`src/FerroUI.CApi/benches/call_overhead.rs`): a property write, a read, an
   event dispatch, frame commits. Section 6 of docs/osprey.md has the numbers of Osprey.
8. **Frames cost what changed.** A commit compares the attribute bytes of every element with
   the ones the retained tree keeps from the last frame and matches siblings by position
   while their keys stay, so an unchanged element costs one short comparison
   (docs/c-abi.md, section 13). A view layer that knows what a subtree depends on makes it a
   memo element (`ferro_frame_memo(version, tag_base)` after its open record) and asks
   `ferro_frame_reuse(root, key, version, tag_base)` before building it: when the last frame
   under the root has it, one reuse record replaces the subtree and the binding does not build
   or write it (the idea of `Html.Lazy` and `React.memo`, with the version as the
   comparison). The version is the binding's promise: a new version whenever anything in the
   subtree changes.
9. **Getters return plain values.** A getter is one call that returns the value, or the
   fallback of its kind when the read fails (`ferro_get_f64_or`, `ferro_get_cstr_or`, ...);
   the failure is recorded, and the binding reports it the way it reports a failed setter
   (Osprey's `failure()`, an exception thrown from the code of the last error in C#), never
   by allocating a result object per read. `ferro_clear_error` and the error buffer allocate
   nothing.
10. **The UI thread is cheap to be on.** The state of the ABI that belongs to the UI thread
   (handles, resolved IDs, frames) is in statics checked with one comparison of the thread
   (docs/c-abi.md, section 13), so a binding gains nothing by caching handles or IDs on its
   side.

11. **Signals in shared memory** (docs/c-abi.md, section 14). A binding whose language can
   read and write memory maps the signal table (`ferro_signal_table`): it writes a bound value
   with a store and a dirty bit, reads a watched value with a load, and reads the versions or
   the changed bits to know what changed, with no call. A binding that cannot (Osprey) calls
   `ferro_signal_set_*` and `ferro_signal_get_*`, one small call each. Reads of values the user
   changes (the text of a text box, the value of a slider, a selection) go through watched
   slots; values of a view that change on their own go through `bind` records of frames, so
   one change is one property write and not a frame. A signal is not a faster setter: a write
   through the table costs about what a setter call costs; it batches, it can come from any
   thread, and it updates frames without describing them again.

### 2.1 Which path to use

The C ABI has three ways to reach the framework: frames, the signal table in shared memory and
direct calls. They are not alternatives to choose between once. Each is the fastest for one kind
of work, and a binding (and the code it generates) uses the fastest one for each kind. The rule
is: **frames for structure, shared memory for values that change, direct calls for the rest.**
The numbers behind it are in docs/c-abi.md, sections 13 and 14.

| Work | Fastest path | For a language that cannot read or write memory |
| --- | --- | --- |
| Build a view, or change its structure (elements added, removed, moved, many attributes at once) | A frame written into the binding's own buffer, one `ferro_frame_commit_bytes` per update | The scalar writer (`ferro_frame_open`, `ferro_frame_str`, ...) and one `ferro_frame_commit` |
| A subtree that did not change | A memo element and `ferro_frame_reuse`: the binding neither builds nor writes it | The same |
| A value of a view that changes on its own (a counter, a progress value, a colour, a text) | A slot bound with a frame `bind` record (or `ferro_signal_bind`), written with a store and a dirty bit | `ferro_signal_set_*`, one small call |
| Many values that change together | Stores and dirty bits for all of them, applied once (at the next commit, the end of the dispatch, or `ferro_signals_apply`) | `ferro_signal_set_*` for each, one apply |
| A value computed on another thread | A store of a numeric slot from that thread (no `ferro_post` per value) | `ferro_signal_set_*` from that thread |
| A value the user changes (the text of a text box, a slider, a selection, a check state) | A watched slot (`ferro_signal_watch`), read with a load; the version or the `changed` bit says whether it moved | `ferro_signal_get_*` and `ferro_signal_version` |
| A text value | A text slot: read it (`ferro_signal_get_cstr`) only when its version moved | The same |
| Events | `ferro_subscribe_dispatch` into the one dispatcher, or `ferro_subscribe` with one function and the token as the index of the binding's table | The same |
| Setup: create objects, add children, show a window, subscribe | Direct calls | The same |
| A single write that is not on a hot path, or a property frames do not model | A direct setter (one call is cheaper than a store and an apply) | The same |
| A single read that is not repeated | A direct getter (`ferro_get_*_or`) | The same |

Do not:

- **Call per element when a frame can carry the work.** A view built or updated with one call
  per element and property pays the call for each; a frame pays one call per update.
- **Write a frame again to change one value.** Bind the value to a slot: one property write
  instead of the description of the view (58 times less for 100 elements).
- **Poll getters.** A value read every frame or in a loop is a watched slot read with a load
  (a tenth to a twentieth of a getter call), and the version tells when to look at all.
- **Expect a slot write to be a faster setter.** Use slots for what they are good at (bound
  values, batches, writes from other threads, values both sides share); a one-off write is a
  setter.
- **Cache IDs or handles on the binding's side, or marshal objects.** IDs are constants of the
  model, the UI-thread check is one comparison, and a wrapper is its handle.

What the generated code does, in every backend:

1. **The default API of a binding is frames, signals and the fluent object layer.** The direct
   calls stay available in the module that mirrors the ABI (`abi.osp` for Osprey) for setup
   and for what frames do not model, but an application written against the generated API
   gets the fastest path without choosing it.
2. **A view value that the program changes is a signal.** The view layer turns a value that
   depends on program state into a bound slot, so a change updates the one property; it falls
   back to writing the frame again only for structure.
3. **A property the program reads back is watched.** The generated reader of a value the user
   changes reads its slot, not the getter.
4. **A backend maps the table when its language can.** A language with pointers or typed
   memory views (C, C++, Rust, Zig, Swift, C# spans, Dart `asTypedList`, Java and Kotlin memory
   segments, JavaScript typed arrays over the module's memory) reads and writes the arrays of
   `FerroSignalTable` directly, with the orders of docs/c-abi.md, section 14.2. Only a language
   without memory access (Osprey 0.29) uses the `ferro_signal_set_*` and `ferro_signal_get_*`
   calls, and its backend switches to the mapped table when the language gains it.
5. **Each backend's benchmark covers the three paths:** a setter and a getter, a slot store and
   apply and a slot load, a bound value in a frame of 100 elements, and a frame commit (item 7
   above), so a backend shows the same ordering as the C numbers, or the reason it does not.

## 3. Idiom

1. **Names are words** in the model (`["font", "size"]`). A backend applies its casing:
   camelCase members (Osprey, JavaScript, TypeScript, Java, Kotlin, Dart, Swift), PascalCase
   (C#, exported Go), snake_case (Python, C, Zig, Rust); enumeration members by the convention
   of the language. A table of reserved words per backend, and an override file per language
   for the exceptions.
2. **Properties the language's way**: C#, Swift, Kotlin and Python properties; Go `X()` and
   `SetX()`; C++ `x()` and `setX()`; Osprey functions that take the handle and return it, for
   the pipe.
3. **Events the language's way**: `event` and `+=` in C#, closures in Swift and Kotlin,
   callbacks or streams in Dart, func values in Go, callables in Python, handlers in
   JavaScript, message tags in Osprey.
4. **Errors the language's way**: exceptions in C#, Java and Python, `throws` in Swift, a
   `(value, error)` pair in Go, `Result` in Osprey and Rust. Status codes stay inside the
   generated layer; `ferro_last_error()` gives the message and `ferro_clear_error()` lets a
   binding tell a failure of a getter with a fallback from its value.
5. **Nullability and enumerations**: the optionals of the language for `nullable` values, its
   native enumerations for `enums` (an Osprey union, a C# `enum`, a Swift `enum`).
6. **Building interfaces the language's way**: object initializers in C#, DSL builders in
   Kotlin, result builders in Swift, named-argument constructors in Dart, a TSX-friendly API
   in TypeScript, pipelines and view functions in Osprey. The frame layer gives any of them an
   Elm-like or React-like model without a reconciler of their own.
7. **Documentation from the model** in the doc-comment format of the language.

## 4. Frames

A frame describes an interface again on every update, and the library reconciles it into the
retained tree of a root. The layout (also `frame` in the model):

- Records of 8-byte words, little-endian. Each starts with a `u32` operation and a `u32` size
  of the whole record in bytes (a multiple of 8).
- `open` (1): type ID, key. `close` (2). `text` (3): property, length, UTF-8 bytes padded to 8.
  `number` (4): property, the bits of an `f64`. `integer` (5): property, value. `bool` (6):
  property, 0 or 1. `on` (7): event, tag, value property. `text16` (8): property, code units,
  UTF-16 padded to 8. `key_text` (9): a text key for the open element. `memo` (10): version,
  tag base of the open element. `reuse` (11): key, version, tag base, in place of an element.
  `bind` (12): property, signal slot: the property follows the slot with no new frame.

The commit: an element keeps its object while its class and its key stay (without a key, its
position among the siblings without one); a value is written when it changed since the last
frame, and a text not even then when the object already shows it (what the user typed into a
text box is never written back); a value the frame no longer sets is cleared; event bindings
stay and take their new tags in place; elements that went away are taken out and released.
Writes go through the setters of the ABI, so a frame converts and validates as they do (the
text of a value is checked for UTF-8 when it is written: an unchanged value is not looked at
again). With the root `0` the elements at the top are windows, shown when they come and
closed when they go. A memo element whose key and version stay is not looked into; a new tag
base only moves the tags of its events. A reuse record stands for the memo element of its
key and version in the last frame under the root, also when it moved to another parent (the
library keeps the bytes of every memo element with an integer key).

Events of a frame call one dispatcher (`ferro_frame_dispatcher`) with the tag, the sender and
the value of the value property (`ferro_frame_event_text`, `ferro_frame_event_f64`). What a
commit causes is not dispatched: events raised while it writes, and events whose value is
still the one the frame set (the `TextChanged` a text box posts after the commit wrote its
text), so two text boxes that follow each other do not echo.

This is a keyed reconcile of its own in `ferroui-capi`, not the reconciler of `ferroui-view`:
that one compares typed Rust elements (`ElementClass`, `&'static FerroProperty`), a frame
names everything by ID and writes through the ABI.

## 5. Threads and lifetime

As for the ABI: UI objects belong to the UI thread; the ID lookups, the fingerprint, the
frame writer (a buffer of the calling thread) and the dispatcher registration work on any
thread. A frame commit, the dispatch and the retained tree are on the UI thread. The retained
tree holds one reference to each of its objects and one to its root until
`ferro_frame_clear`.

## 6. What a backend generates

At least: the declarations of the ABI functions its FFI can call; the fingerprint check; the
enumerations; per class a constructor (object layer) and an element (frame layer) where the
class can be created; per property a setter and a getter (object layer) and an attribute
(frame layer); per event a subscription (object layer) and an attribute (frame layer); and
documentation from the model.

## 7. Status

Done in the Osprey MVP: the model and its tests, stable IDs compiled into the library and the
fingerprint, the frame layout with the scalar writer and the buffer commit, UTF-16 entry
points, getters with a fallback, both layers of Osprey for every class, selections from a
manifest (each sample builds in one to two seconds), five samples and a benchmark. Then the
performance work of docs/c-abi.md, section 13: frames proportional to what changed, memo
elements and reuse records (the `memo(version)` attribute of Osprey), plain getters, events
subscribed to the dispatcher, and the cheaper calls; then signals shared through memory
(section 14 of docs/c-abi.md), with `signal*` helpers and `bound(attr, signal)` in Osprey.

Next:

- The library trimmed by the same manifest as the bindings (phase 2 of #636).
- The default paths of section 2.1 in the generated view layer of Osprey: values that depend
  on program state bound to slots by the generator (today the application writes
  `bound(attr, signal)` itself), and generated readers of user-changed values through watched
  slots.
- The mapped signal table (loads and stores, no call) in every backend whose language can
  reach memory; Osprey keeps the calls until it can.
- Object values in frames (an element as the value of a property, not only as a child), and
  typed event arguments (pointer and key events) as values of a frame event.
- Typed handle wrappers per class where the language has value types (section 2.4).
- The backends of C++, C#, Go, Dart, Python, Swift and Zig, each with its benchmark.

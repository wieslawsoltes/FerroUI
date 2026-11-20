# Using FerroUI from your language

FerroUI is a Rust framework with one C ABI (`ferroui-capi`, the header
`src/FerroUI.CApi/include/ferroui.h`) that every language with a foreign function interface
can call. This guide is for application and binding authors: how to call it correctly from
your language, and how to take the fastest path for each kind of work. The reference of
every function is docs/c-abi.md; the rules generated bindings follow are
docs/bindings-design.md.

## 1. Start from a generated binding

If your language has a generated binding (Osprey today, `bindings/osprey`, docs/osprey.md),
use it: it already passes constant IDs, picks the fastest path for each operation and maps
errors and callbacks to your language. Write against the header directly only for a
language without a binding, or to write one. Either way the rules below hold.

## 2. The fastest path

**Frames for structure, shared memory for values that change, direct calls for the rest.**

| You want to | Use | Avoid |
| --- | --- | --- |
| Build a view, or change its structure | A frame: fill a buffer and call `ferro_frame_commit_bytes` once per update (or the scalar writer `ferro_frame_open`, `ferro_frame_str`, ... and `ferro_frame_commit` when your language cannot write memory) | One call per element and property |
| Skip a subtree that did not change | A memo element (`ferro_frame_memo`) and `ferro_frame_reuse` with a version you raise whenever the subtree changes | Building and sending it again |
| Change a value of a view (a counter, a progress, a colour, a text) | A signal slot bound to the property (the frame `bind` record or `ferro_signal_bind`): a store and a dirty bit | Sending the frame again for one value (58 times slower for 100 elements) |
| Change many values at once | Stores and dirty bits for all of them; one apply at the next commit, the end of the dispatch, or `ferro_signals_apply` | A setter per value from a loop that runs every frame |
| Produce values on a worker thread | A store of a numeric slot from that thread | `ferro_post` per value |
| Read what the user changed (text, slider, selection, check state) | A watched slot (`ferro_signal_watch`): a load, and the version or `ferro_signal_take_changed` to know whether it moved | Calling a getter every frame or in a loop |
| Read a text value | A text slot, read with `ferro_signal_get_cstr` only when its version moved | Reading the text on every frame |
| Handle events | `ferro_subscribe_dispatch` into one dispatcher (`ferro_frame_dispatcher`), or `ferro_subscribe` with one function and the token as an index into your table of handlers | A native thunk or closure per handler |
| Set up: create, add children, show, subscribe | Direct calls | |
| Write a property once, outside a hot path | A direct setter (`ferro_set_f64`, `ferro_set_str`, ...): one call is cheaper than a store and an apply | A slot for a value written once |
| Read a property once | A direct getter with a fallback (`ferro_get_f64_or`, ...) | |

A signal write is not a faster setter: it pays where it batches, comes from another thread,
or updates a bound value of a frame. A signal read is always cheaper than a getter (a load
against a call: 0.3 to 0.6 ns against 7.3 ns in C and Rust). The measurements are in
docs/c-abi.md, sections 13 and 14.

## 3. Calling the ABI from your language

Use the fastest FFI path your language has, and declare the functions with plain scalar
signatures (`int64_t`, `uint64_t` handles, `double`, `bool`, pointer and length for text):

| Language | How to call | Notes |
| --- | --- | --- |
| C, C++, Zig | include `ferroui.h`, link the library | Optionally link-time optimisation across languages (docs/c-abi.md, 13.4) |
| Rust | `extern "C"` declarations from the header, or `ferroui-capi` as a Rust library | A Rust application uses the framework directly, not the ABI |
| C#, F# | `[LibraryImport]` with blittable signatures; `[SuppressGCTransition]` on setters, getters and signal calls | Pass `string` through `ferro_set_str16` (UTF-16, no transcoding) |
| Java, Kotlin | Foreign function API (Panama): `Linker.nativeLinker().downcallHandle(...)`, with `Linker.Option.critical(false)` on leaf calls | Keep the method handles in `static final` fields |
| Swift | Import the header through a module map or a bridging header | Strings: `withCString` and a negative length, or the UTF-8 view and its count |
| Dart | `@Native` declarations, `isLeaf: true` on leaf calls | Run the UI on the isolate whose thread called `ferro_init` |
| Go | cgo; call `runtime.LockOSThread()` in `init` of the package that runs the UI | Each cgo call costs more than in other languages: frames and signals matter most here |
| Python | `cffi` in API mode for anything called often; `ctypes` is fine for setup | Keep per-frame work in frames and signals, not in Python loops of calls |
| JavaScript (browser) | The WebAssembly exports of the library | Text as UTF-16 (`ferro_set_str16`) from JS strings |
| Osprey | The generated binding (`extern fn` of scalars, named functions) | docs/osprey.md |

Rules every caller follows:

- **Check the version and the model once at start**: `ferro_abi_version()` against
  `FERRO_ABI_VERSION`, and `ferro_api_fingerprint()` against the fingerprint of the model your
  IDs come from.
- **Use constant IDs.** Generated bindings pass the stable IDs of the model
  (`bindings/model/ferroui-api.json`). Hand-written code looks each name up once
  (`ferro_type_id`, `ferro_property_id`, `ferro_event_id`) and keeps the integer; never look a
  name up per call.
- **Register the classes you use** before `ferro_app_run`: call the entry of each class
  (`ferro_type_window()`, `ferro_type_button()`; generated bindings call them in
  `registerTypes()`). An entry returns the type ID. A library built without the tables of types
  knows only the classes registered this way, and a program that links it statically keeps only
  those and the theme they need (docs/c-abi.md, section 16).
- **The UI thread.** UI calls run on the thread that called `ferro_init` or `ferro_app_run`
  (the main thread on macOS). From any other thread, post the work (`ferro_post`,
  `ferro_post_data`) or store numeric signal slots.
- **Handles are integers with a reference count.** Release what you created once the tree
  holds it (`ferro_release`); the framework owns its tree. Retain a handle you keep beyond a
  callback (`ferro_retain`). A stale handle is an error, never a crash.
- **Text** is UTF-8 with a length in bytes (a negative length for a NUL-terminated string),
  borrowed for the call, or UTF-16 through `ferro_set_str16` and `ferro_frame_str16`.
- **Errors are return values.** Every call returns a status, an ID, a handle or a value with a
  fallback; read `ferro_last_error_code()` and `ferro_last_error()` after a failure and turn
  them into your language's error. Never ignore a negative status.
- **No structs by value, no varargs.** Everything crosses as scalars or a pointer and a length,
  which every FFI handles (Go's cgo and Python's `ctypes` handle structs by value poorly).
- **Do not cache what is already cheap.** The thread check is one comparison and IDs are
  constants; a wrapper object is its handle and nothing else (a value type where your language
  has one: a `readonly struct` in C#, a `struct` in Swift, a `value class` in Kotlin).

## 4. Callbacks into your language

Events, posted work and (with #974) the members of controls you write call back into your
code. To work in every language and stay fast:

- **A plain function pointer and an integer or a data pointer, no closures.** Give the library
  one function and keep your handlers in a table on your side; the token is the index
  (`ferro_subscribe`), or the `void *` is your object (`ferro_subscribe_data`, with a release
  function the library calls when the handler is gone).
- **Keep the callback and its data alive.** In a garbage-collected runtime the data pointer is
  a pinned handle (`GCHandle` in .NET, a global reference in Java, `Unmanaged` in Swift,
  `cgo.Handle` in Go, a kept `ffi.new_handle` in Python), freed in the release function.
- **Callbacks run synchronously on the UI thread**, and may call the ABI again (set a property,
  add a child, read a value): the library holds none of its own state while it calls out.
- **Never let an exception cross the boundary.** Catch everything in the callback and return a
  result (`0` or a status); the binding reports the failure in your language.
- **Keep callbacks short.** Layout and rendering call back only for elements that changed, and
  rendering is retained (the GPU scene replays what was drawn until the element is
  invalidated), so the cost of a callback matters only for work that runs often.

| Language | Callback mechanism | Typical cost of one callback |
| --- | --- | --- |
| C, C++, Rust, Zig | function pointer | 1 to 2 ns |
| C#, F# | `[UnmanagedCallersOnly]` function pointer (`delegate* unmanaged`) | a few ns |
| Swift | `@convention(c)` function and a context pointer | a few ns |
| Java, Kotlin | upcall stub (`Linker.upcallStub`) | 10 to 20 ns |
| Dart | `NativeCallable.isolateLocal` | tens of ns |
| Go | exported cgo function | 50 to 100 ns or more |
| Python | `cffi` `@ffi.def_extern` (API mode) or `ctypes` callback | around 1 us |
| JavaScript (browser) | imported function of the WebAssembly module | a few to tens of ns |
| Osprey | named function | docs/osprey.md, section 6 |

These costs are typical for each FFI, not measurements of FerroUI; each binding's benchmark
measures its own (docs/bindings-design.md, section 2, item 7).

## 5. The signal table from your language

`ferro_signal_table(capacity)` returns a table whose arrays stay at their addresses until the
process ends (docs/c-abi.md, section 14.2). Map it once and read and write it directly:

| Language | View of the arrays |
| --- | --- |
| C, C++, Zig, Rust | the pointers of `FerroSignalTable`; C11 `<stdatomic.h>`, `std::atomic_ref`, Rust atomics |
| C# | `Span<long>` over the pointer, `Volatile.Read`/`Volatile.Write` and `Interlocked.Or` |
| Java, Kotlin | `MemorySegment.reinterpret(capacity * 8)` and a `VarHandle` with `getAcquire`/`setRelease`/`getAndBitwiseOrRelease` |
| Swift | `UnsafeMutablePointer<UInt64>` with `UnsafeAtomic` of the swift-atomics package |
| Dart | `Pointer<Uint64>.asTypedList(capacity)`; write from the UI isolate only |
| Go | `unsafe.Slice` and `sync/atomic` |
| Python | the `ferro_signal_set_*` and `ferro_signal_get_*` calls (no atomics on raw memory) |
| JavaScript | typed arrays over the module's memory and `Atomics` when the memory is shared |
| Osprey | the `signal*` helpers of the binding (calls; Osprey 0.29 has no memory access) |

The protocol, from any language:

1. **Write a bound value:** store the value word (relaxed), then set bit `slot % 64` of
   `dirty[slot / 64]` with a release OR. A language without atomics writes only on the UI
   thread, with plain stores, or calls `ferro_signal_set_*`.
2. **Read a watched value:** read `versions[slot]` (acquire), then the value word; compare the
   version with the last one you read, or test and clear the `changed` bit
   (`ferro_signal_take_changed`).
3. **Text slots** keep the text in the library: the value word is its length, and
   `ferro_signal_get_cstr` reads it when the version moved.
4. Slots are allocated and freed with `ferro_signal_new` and `ferro_signal_free`, bound with
   `ferro_signal_bind` or the frame `bind` record, and watched with `ferro_signal_watch`; the
   kind of a slot is fixed when it is allocated.

## 6. Controls written in your language

A control is a class registered with the library (docs/c-abi.md, section 15) on one of four
bases: `FERRO_BASE_CONTROL` (draws and lays out itself), `FERRO_BASE_PANEL` (children),
`FERRO_BASE_DECORATOR` (one child) and `FERRO_BASE_CONTENT_CONTROL` (a content and a template).

1. **Register the class once**: `ferro_class_register("Gauge", -1, FERRO_BASE_CONTROL)` returns its
   type ID, which `ferro_create`, frames and styles use as any class.
2. **Set the callbacks of the members you override**, and only those: `ferro_class_on_create`,
   `_on_measure`, `_on_arrange`, `_on_render`, `_on_property_changed`, `_on_attached`,
   `_on_detached`, `_on_apply_template`, `_on_pointer`, `_on_key`. A member without a callback
   runs the base class with no call into your language. Each callback is one exported function of
   your binding (the mechanism of section 4) that receives the data pointer of the instance.
3. **Give each instance its data**: the create callback calls `ferro_object_set_data(object,
   data, release)`. In a garbage-collected runtime the data is a pinned handle to your object
   (`GCHandle`, a global reference, `Unmanaged`, `cgo.Handle`, a kept `ffi.new_handle`), freed in
   `release`, which the library calls when the instance goes away. A callback turns the pointer
   back into your object and calls its method.
4. **Register the properties of the class**: `ferro_property_register(class_id, "Level", -1,
   FERRO_SIGNAL_F64, 0.5, NULL, FERRO_PROPERTY_AFFECTS_RENDER)` returns a property ID that the
   setters and getters, frames, signals, styles, bindings and animations use. The flags make a
   change measure, arrange or render the instance again; `FERRO_PROPERTY_ATTACHED` makes an
   attached property (`Gauge.Level` on any control).
5. **Measure and arrange with scalars**: the callback gets the available (final) size and writes
   the desired (used) size through its two `double *` (or calls `ferro_layout_result` where your
   language cannot write memory). Lay out children with `ferro_layout_measure`,
   `ferro_layout_arrange` and `ferro_layout_desired_width`/`_height`; a panel's children are
   `ferro_panel_child_count` and `ferro_panel_child`.
6. **Draw in the render callback** with the context it receives: make brushes and pens once
   (`ferro_draw_brush(0xFF2E7D32)`, `ferro_draw_pen`), then `ferro_draw_rect`, `_ellipse`,
   `_line`, `_path` (path markup), `_text`, and `_push_clip`, `_push_transform`,
   `_push_opacity` with `ferro_draw_pop`. The drawing is recorded into the retained GPU scene and
   replayed until the instance is invalidated: render is not called every frame.
7. **Find template parts** of a content control from the apply-template callback on:
   `ferro_template_part(object, "PART_Track", -1)`.
8. **Return statuses, never throw**: `0` (or `1` for a handled pointer or key event), a negative
   status on a failure, which the library records and answers with the base behaviour.

`samples/CApi/gauge.h` is a complete control in C (registration, data, measure, arrange,
render, pointer) and `samples/CApi/osprey-bindings/gauge_control` the same control in Osprey,
which keeps its state in its registered property instead of instance data. The cost of the calls
against the same control in Rust is in docs/c-abi.md, section 15.4.

## 7. Linking statically

A program that links the static library (`libferroui_capi.a`, `ferroui_capi.lib`) keeps only what
it reaches, if the linker is told to drop the rest, and if the library leaves something to drop:
build it by use (`--no-default-features --features desktop`, docs/c-abi.md, section 16) and
register the classes the program uses with their entries. The C hello world is 36.97 MB linked
statically against the full library and 20.12 MB against the library built by use (macOS,
release, stripped; docs/binary-size.md, section 3.7). A dynamic library exports every entry and so
keeps every class: the per-application library with the exports of a selection is #977.

The native libraries the static library needs (Skia's C++ runtime, the frameworks or system
libraries of the platform) are the ones rustc prints for it:

```sh
cargo rustc -p ferroui-capi --release --lib --no-default-features --features desktop \
    -- --print native-static-libs
```

The flags that drop what a program does not reach, per toolchain:

| Toolchain | Dead code | Notes |
| --- | --- | --- |
| clang or GCC on macOS (ld64) | `-Wl,-dead_strip` | then `strip` the program; `-Wl,-map,<file>` writes the map that `scripts/capi/size_by_crate.py` sums by crate |
| GCC or clang on Linux (GNU ld, lld, gold) | `-Wl,--gc-sections` | Rust objects have a section per function, so the collection reaches into the library; `-s` or `strip` for the symbols |
| MSVC (`link.exe`) | `/OPT:REF /OPT:ICF` | the default of a release link without `/DEBUG`; with `/DEBUG` state them, since the symbols go to the PDB |
| MinGW | `-Wl,--gc-sections` | as on Linux |
| SwiftPM | on by default on Apple platforms (`swift build --disable-dead-strip` turns it off) | add the library with `linkerSettings: [.linkedLibrary("ferroui_capi"), .unsafeFlags(["-L<dir>"])]` and each framework of the native libraries with `.linkedFramework` |
| Go (cgo) | `go build -ldflags='-s -w -extldflags=-Wl,-dead_strip'` (macOS) or `-extldflags=-Wl,--gc-sections` (Linux) | `#cgo LDFLAGS: -L${SRCDIR}/lib -lferroui_capi` and the native libraries; cgo links with the external linker, which does the collection |
| Rust (a Rust launcher) | none: link the `rlib` and let cargo link | the application is then a Rust application (docs/binary-size.md) |

Link-time optimisation across the C program and the library (clang with `-flto=thin` and lld of
the LLVM version of rustc) is docs/c-abi.md, section 13.4: it inlines the small functions of the
ABI into the program; it is not needed for the size, which dead stripping gives.

## 8. Checklist

- Generated binding, or constant IDs looked up once.
- Version and fingerprint checked at start.
- The entries of the classes you use called before `ferro_app_run`.
- Views as frames, memo elements for subtrees that rarely change.
- Values that change bound to signal slots; values the user changes watched and read with
  loads.
- One dispatcher or one callback function with a table of handlers.
- UI calls on the UI thread only; other threads post work or store numeric slots.
- Handles released once the tree holds them; callbacks kept alive and pinned.
- Every negative status turned into your language's error; no exception crosses the boundary.
- The benchmark of your binding (`src/FerroUI.CApi/benches/call_overhead.rs` and
  `samples/CApi/call_overhead.c` are the reference) shows the same ordering of paths.

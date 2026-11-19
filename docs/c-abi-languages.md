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

Writing new controls through the ABI (classes on `Control`, `Panel`, `Decorator` and
`ContentControl`, registered properties, drawing in render, template parts) is #974, in
progress. The rules above apply to it: one table of callbacks per class, a data pointer per
instance, scalars only, synchronous callbacks that may call the ABI again, errors as results.
Implement only the members you need: a member you do not implement runs the base class with no
call into your language. This section will describe the calls when #974 lands.

## 7. Checklist

- Generated binding, or constant IDs looked up once.
- Version and fingerprint checked at start.
- Views as frames, memo elements for subtrees that rarely change.
- Values that change bound to signal slots; values the user changes watched and read with
  loads.
- One dispatcher or one callback function with a table of handlers.
- UI calls on the UI thread only; other threads post work or store numeric slots.
- Handles released once the tree holds them; callbacks kept alive and pinned.
- Every negative status turned into your language's error; no exception crosses the boundary.
- The benchmark of your binding (`src/FerroUI.CApi/benches/call_overhead.rs` and
  `samples/CApi/call_overhead.c` are the reference) shows the same ordering of paths.

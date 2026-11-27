# Runtime registration in the macOS arm64 ControlCatalog

Sharing the invocation code for one common routed-event handler signature reduced the **default desktop Catalog by 98,784 bytes (0.15%)**, from 65,859,504 to 65,760,720. The matched macOS arm64 builds used the same private checkout at `933fd4632577`, normal Cargo `dist` profile, diagnostics dependency and native backend. The candidate changes two Base metadata source files. [Exact patch](routed-handler.patch) · [Full executable and link-map hashes, sizes and checks](catalog-pair-results.json)

## Stock registration footprint

The stock 65,876,016-byte Catalog contains **832,408 bytes of type and member descriptors** in relocated constants. Following their public method, property, field, event and constructor callbacks reaches **1,547,260 instruction bytes** at 4,742 distinct entry addresses. The sibling 65,859,504-byte stock link from the binding experiment gives identical counts. These bytes are already included in the Catalog allocation. Other code can reference the same functions. [Full allocation](../../../themes/full-opcode-spike/catalog-allocation.json) · [Registration counts](catalog.json)

The desktop package's release dependency on Diagnostics enables Base's `full-themes`, which enables `type-tables`. The F12 attachment itself is debug-only. The registration path and method macro match the reviewed revision `74ed59e571a3` on Catalog revision `933fd4632577`. The review's 12,564,928-byte feature delta was measured in HelloWorld with 16 WinUI units. [Desktop manifest](../../../../samples/ControlCatalog.Desktop/Cargo.toml) · [Diagnostics manifest](../../../../src/FerroUI.Diagnostics/Cargo.toml) · [Base features](../../../../src/FerroUI.Base/Cargo.toml) · [Desktop startup](../../../../samples/ControlCatalog.Desktop/program.rs)

| Named registered descriptor family | Distinct rows | Data bytes | Direct callback instruction bytes |
| --- | ---: | ---: | ---: |
| Type headers | 1,328 | 478,080 | — |
| Methods | 1,138 | 81,936 | 732,732 |
| Instance properties | 1,703 | 122,616 | 543,256 |
| Constructors | 482 | 19,280 | 146,304 |
| Events | 141 | 5,640 | 101,976 |
| Static fields | 168 | 8,064 | 20,156 |
| Static properties | 331 | 23,832 | 5,576 |
| Enum members | 2,324 | 92,960 | — |

The census reads named 360-byte `MARKUP` records in the shipped image, walks their slices using the Base crate's measured field offsets, and resolves non-null invoker pointers against live link-map symbols. It verifies that the binary and map section ranges match, each pointer names a live instruction entry, and all counted data and instruction ranges are disjoint after de-duplication. The data count covers headers and the listed rows. It excludes auxiliary descriptors, property type/typed-path hooks, transitive callees, alignment and anonymous type headers. An invocation body can include inlined business logic. Within each family addresses count once. The total across families removes shared entries again. [Census tool](census.py) · [Measured Rust layout](layout.txt)

## Sharing the routed-event method adapter

Of the registered instance methods, 348 on 95 type records take `(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>)` and declare no return value. Their 345 distinct direct invokers occupy **227,284 bytes** within the methods row. The census identifies this signature through the parameter-type function addresses, using the Catalog's `DrawerPageEventsPage.OnClearLog` declaration as the reference. Today's method macro prepares the receiver, converts the arguments, and invokes the typed callable in each member. The fixture preserves the public `MarkupMethod` descriptor and replaces its invoker with a forwarding function. Methods on each receiver type share one argument-checking and conversion function. A typed function pointer selects the individual handler. [Production macro](../../../../src/FerroUI.Base/metadata/markup_macros.rs) · [Reference handler](../../../../samples/ControlCatalog/Pages/DrawerPage/drawer_page_events_page.rs) · [Fixture](handlers.rs)

| 32-method linked fixture | File bytes | Machine instructions in bytes | Median ns per call |
| --- | ---: | ---: | ---: |
| Current method macro | 3,260,720 | 2,628,456 | 9.41 |
| Shared per-receiver invoker | 3,244,192 | 2,614,056 | 8.94 |

The difference is **16,528 file bytes** and **14,400 instruction bytes** on four concrete framework classes with eight fixture methods each. The fixture calls the actual declaration macro against the same prebuilt production Base library on both arms. Both executables check valid calls, null and non-null senders, errors and argument indices, registered receiver and event conversions, reference release and handler effects. The timing loop alternates eight methods on one exact receiver for one million calls after 100,000 warmup calls per process. The median paired timing ratio was 0.951 across 31 randomized pairs, with a 95% paired bootstrap interval of 0.944–0.960. These are single-host invocation timings. The full-application comparison below measures executable size separately. Desktop interaction latency was not timed in this pair. The fixture covers infallible shared-receiver declarations with direct callable leaves. The census signature can also describe other callable and receiver forms. [Samples and hashes](probe-results.json)

## Matched shipping executable

| Normal desktop Catalog | Executable bytes | Machine instructions in bytes | Relocatable constants |
| --- | ---: | ---: | ---: |
| Stock | 65,859,504 | 51,347,192 | 5,013,344 |
| With the shared handler adapter | 65,760,720 | 51,240,632 | 5,013,344 |

All **1,138 registered method identities and signatures** matched across the images, including their declaring type and declaration order. Of 348 routed-event methods, the candidate selected 340. Their 337 distinct entry bodies declined from 221,980 to 7,932 instruction bytes. The 94 shared receiver adapters occupy 72,728 instruction bytes. Typed handler functions and other linked code account for the difference between those counts and the **106,560-byte** shrinkage of the complete text section. The other eight routed methods all use a `handler!(...)` expression on CompositionPage, preserving their original eight invokers and 5,304 instruction bytes. No unrelated method entry branches to a routed adapter. [Method branch and signature check](compare_methods.py) · [CompositionPage declarations](../../../../samples/ControlCatalog/Pages/composition_page.rs)

The macro selects a literal, infallible void closure with two declared parameters. Compile-time type selection then restricts the adapter to framework `Ref<T>` receivers, `Option<BoxedValue>` senders and `Rc<dyn IRoutedEventArgs>` events, including type aliases. Other receiver or parameter types retain the original descriptor invoker. Fallible, returning, static, attributed and general callable expressions retain their declaration paths. Both arms passed checks of argument count and type errors, conversion order, receiver and event conversion, ownership, null and non-null senders, static and fallible calls, attributes and callable-construction effects. [Declaration checks](declarations.rs)

Both complete applications launched with their defaults. The standard WinUI Light framework captures of DrawerPage, TabbedPage and ListBox each matched at every pixel of the 2200×1524 frames. The checks sampled these page renderings and the declared method behavior. They did not time UI interactions or run a full Catalog test suite. The two arms use stock XAML and stock binding code. Combining this patch with either optimization requires its own link. [Pixel hashes and validation results](catalog-pair-results.json)

## Reproduction

Run from the repository root on macOS arm64. Supply the normal Catalog binary and its matching Apple map to the census. Supply the prebuilt target Base library from the Catalog's dist dependency directory to the fixture. The driver checks that its features are `full-themes`, `markup-functions` and `type-tables` and validates its Rust layout. All links and scratch files are written to the requested output directory.

```sh
review=scripts/perf/typed-binding-size/registration-review
python3 "$review/census.py" target/size-investigation/control-catalog-desktop target/size-investigation/control-catalog-desktop.map
python3 "$review/probe.py" target/dist/deps/libferroui_base-3d08cc0236938f5b.rlib target/registration-review/replay
```

To reproduce the full pair, use a fresh output directory and provide the normal Skia build prerequisites. The measured run provided `SKIA_NINJA_COMMAND` pointing at the installed Ninja and seeded a private Cargo cache with a copy-on-write clone of `target/dist`. Both arms shared that private cache. The driver also works with a new cache. Its only additional link argument requests the Apple map. It leaves the detached checkout, binaries, maps, validation executables and screenshot pairs under the output directory.

```sh
python3 scripts/perf/typed-binding-size/registration-review/catalog_pair.py /path/to/new/catalog-pair
```

# Ahead-of-time XAML execution on macOS arm64

The compiler resolves types, overloads, events and properties against the same models as the native Rust emitter. It encodes common operations into four instruction families. The runtime executes those operations directly and calls typed Rust adapters for framework members. It requires neither a runtime XML parse nor an application-wide type registry for the operations it lowers.

| Family | Encoded operations | Native support retained |
| --- | --- | --- |
| Resources | Resource keys, alias and brush rows, typed literals, declaration and value positions | Per-run string and lexical namespace imports, original deferred materialization |
| Control styles | Style hierarchy, setters, selectors, template descriptors | Property and type identity imports, typed computed values, expression and template factories |
| Object construction | Constructor, initialization, registered setter, parent stack, namescope, template binding, dynamic resource, deferred factory and resolved member call | Original class constructors, exact typed member calls, values and source locations |
| Binding paths | Ordered property and index paths, roots, ancestor or name lookup, stream operators, nested paths | Property, accessor, converter, command or method imports, original reactive binding engine |

Cargo writes instruction images next to each generated Rust module, with an `.fxc` extension. Identical streams in a module share an offset. Each generated factory references its own slices as constant arrays. Unreachable units can be discarded individually by the linker. Each construction factory shares one typed import table across its instruction regions. Typed object registers preserve object identity. Value registers retain their Rust representation, including nullable values and reference-counted collections. Deferred template factories still execute when a template instance is requested.

The backend can encode an entire class document's population body when its only external object operand is the document root. A shared runtime entry point initializes the context and namescope and executes the program. Other factories mix instruction regions with native statements for operations outside the supported set. Rust also implements application handlers and binding accessors. The executable comparison includes those costs.

The construction pass presently recognizes exact statement forms produced by the resolved Rust emitter and uses its type and call records. An unrecognized spelling ends an instruction region. A compiler intended to emit bytecode for every XAML construct would benefit from a typed instruction representation shared by both emitters, with explicit operands for target-property scopes, checked conversions and collection operations. This would make coverage independent of Rust source formatting. Among the 127 mixed class documents in the measured Catalog build, 41 contained a direct extension `ProvideValue` call, 31 contained a property write with a run-time cast and 25 contained a typed dictionary capacity check. These counts overlap.

## Cargo entry points

`Build::from_env()` reads `FERROUI_XAML_BACKEND`. The same representation can be selected explicitly with `Build::xaml_backend(XamlBackend::...)`.

| Backend | Representation |
| --- | --- |
| `native` (default) | Existing generated Rust and native resource/style tables |
| `theme-bytecode` | Typed resource and control style programs, ordinary native page and template constructors |
| `bytecode` | Theme programs plus construction programs and binding-path instructions |

Build the sample apps in separate Cargo invocations to preserve their usual dependency features. For example, run `FERROUI_XAML_BACKEND=bytecode cargo build --locked --profile dist -p hello-world` and then repeat with `-p control-catalog-desktop`. The backend selection changes the compiler's generated representation. Benchmarks use the same Rust profile on both sides.

## Instruction storage in the integrated application

The binary-file backend generated 467,965 bytes of instruction images across 233 modules on the default ControlCatalog dependency graph. Generated factories referenced 457,041 unique bytes within those images before the linker trimmed any units. The remaining image bytes came from compiler candidates subsequently replaced by a fused operation. The two Hello World application documents themselves used 18 and 64 instruction bytes.

| Package in the ControlCatalog build | Generated instruction image bytes |
| --- | ---: |
| ControlCatalog | 243,370 |
| Full WinUI theme | 168,176 |
| Color picker | 19,979 |
| Diagnostics | 18,327 |
| Inking | 8,605 |
| TreeDataGrid demo | 6,577 |
| TreeDataGrid controls | 2,333 |
| Dialogs | 598 |

These counts include numeric operands and references into native import tables. Strings held in those tables, callback bodies, native XAML operations and the runtime contribute separately to the linked executable. Recompute the inventory for a particular build with `python3 scripts/themes/full-opcode-spike/application_image.py target/dist/control-catalog-desktop.d`.

The later theme-unit build emitted 1,170 construction import descriptors across the Catalog dependency graph, with 35,868 operation entries and 8,750 string references. On arm64, Rust's `size_of` reports 32 B for `ConstructionImports`, 16 B for `ConstructionImport` and 16 B for `&str`. This gives 751,328 B of table storage before duplicate folding or reachability trimming. Distinct string values in those construction pools contain another 115,377 UTF-8 bytes across the graph. WinUI accounts for 225,120 B of table storage, with 24,985 distinct string bytes. These counts exclude style, resource and binding-path import tables and the machine code that implements an operation. Strings can also be shared with other parts of the executable.

## Linked results

The following comparisons measure final executables after the same Apple strip and ad-hoc signing steps. The checked-in original 65,876,016-byte ControlCatalog and its normalized copy differ in signing data. The native executable was rebuilt for the table-based experiments.

| Experiment | Native | Candidate | Candidate minus native |
| --- | ---: | ---: | ---: |
| ControlCatalog, only resource rows encoded | 65,498,144 B | 65,255,904 B | −242,240 B |
| ControlCatalog, resource rows and control styles | 65,498,144 B | 64,969,664 B | −528,480 B |
| ControlCatalog, additionally construction and binding paths, shared per-factory import tables (inline instruction prototype) | 65,498,144 B | 64,653,776 B | −844,368 B |
| ControlCatalog, binding and construction instructions in binary files (literal method arguments and routed-event adapters) | 65,498,144 B | 64,473,088 B | −1,025,056 B |
| ControlCatalog, shared property writes from typed registers | 65,498,144 B | 64,193,936 B | −1,304,208 B |
| ControlCatalog, additionally resolved selectors, static resources and closed constructor operands | 65,498,144 B | 63,614,800 B | −1,883,344 B |
| ControlCatalog, shared typed result extraction at native boundaries | 65,498,144 B | 63,039,408 B | −2,458,736 B |
| ControlCatalog, contiguous result registers held in the interpreter vector | 65,498,144 B | 61,922,496 B | −3,575,648 B |
| ControlCatalog, ordinary style setters and additional property literals encoded | 65,498,144 B | 61,528,256 B | −3,969,888 B |
| ControlCatalog, strings indexed separately from typed operations | 65,498,144 B | 61,221,616 B | −4,276,528 B |
| ControlCatalog, additionally ordered dynamic setter choices | 65,498,144 B | 61,536,960 B | −3,961,184 B |
| ControlCatalog, whole class documents and proven default template binding | 65,498,144 B | 61,122,896 B | −4,375,248 B |
| ControlCatalog, additionally typed markup-extension calls and target-property scopes | 65,498,144 B | 61,106,672 B | −4,391,472 B |
| ControlCatalog, additionally selected theme runs, namespaces and ownership transfers | 65,498,144 B | 61,024,192 B | −4,473,952 B |
| ControlCatalog, additionally dictionary capacity, resolved static fields and converted property writes | 65,498,144 B | 60,924,816 B | −4,573,328 B |
| ControlCatalog, shared decoding for registered-property writes | 65,498,144 B | 60,859,136 B | −4,639,008 B |
| ControlCatalog, vector geometry literals in construction bytecode | 65,498,144 B | 60,744,224 B | −4,753,920 B |
| Hello World, shared decoding for registered-property writes | 16,290,272 B | 16,267,248 B | −23,024 B |
| Hello World, selected theme runs and ownership transfers | 16,290,272 B | 16,283,888 B | −6,384 B |
| Hello World, whole-document execution and proven default template binding | 16,290,272 B | 16,283,984 B | −6,288 B |
| Hello World, shared typed result extraction | 16,290,272 B | 16,300,320 B | +10,048 B |
| Hello World, binary instruction files | 16,290,272 B | 16,300,544 B | +10,272 B |

For the 64,473,088-byte image build, the map attributes 4,864,568 B to generated XAML instructions. All nine screenshots matched in a repeat paired capture at 2,200 × 1,524 pixels. An earlier Light Buttons capture differed from its immediate native counterpart and matched another native capture exactly.

The shared-property-write build brings generated XAML instructions down to 4,498,568 B. Its compiler produced 475,177 B of instruction images. All nine checked screenshots matched native. Across 15 alternating launches the median time to the after-setup marker moved from 136.318 ms to 139.002 ms. That marker precedes window construction. Each timing comparison uses its own matched native processes.

The 63,614,800-byte Catalog contains 3,822,748 B of machine instructions attributed to generated XAML. The compiler emitted 529,669 B of instruction images across 233 modules, which include 335 selector program references. The new lowering covers ordinary selector chains, literal-key static resource assignment, exact nullable values and closed constructor operands. Nine 1,900 × 1,360 screenshots matched. In 15 alternating launches, median time to the after-setup marker was 155.081 ms native and 154.924 ms bytecode. Native typed-binding expressions still occupy 4,515,736 B in this candidate. The separate binding-controller experiment is excluded from this comparison.

Grouping consecutive internal registers in the interpreter's existing result vector eliminated repeated per-object extraction and destruction in generated Rust factories. Scope guards borrow disjoint slices and drop each slice in reverse declaration order. Live native locals retain their original position between groups. The compiler emits these groups only at boundaries that return some registers to native code. This reduced generated XAML instructions to 1,982,520 B. A destructor-order regression test and the complete Catalog library suite passed (936 passed, 20 ignored).

Nine screenshots matched for the 61,922,496-byte build. Across 15 pairs, median setup took 152.344 ms native and 144.903 ms bytecode. In seven alternating frame runs covering ListBox, TreeDataGrid and Data Validation, each executable recorded 2,268 draws. Median render times were 7.531 and 7.507 ms. The 95th percentiles were 9.434 and 9.495 ms. These process and compositor measurements exclude physical input and presentation latency.

Encoding ordinary style setters and closed property literals reduced generated XAML instructions to 1,609,676 B. The 211 compiled-markup comparisons passed and all nine screenshots matched. The [application measurements](typed-application-results.json) include linked-image hashes, allocation and instruction counts, per-package instruction storage and startup samples for each measured stage. The [full-theme measurements](typed-selector-results.json) use the earlier shared-result-extraction source snapshot for both the full and Button-trimmed WinUI and Fluent theme probes.

Strings and typed operations now occupy separate import arrays in each factory. A string no longer forces every operation entry to reserve space for a pointer and a length in addition to the tag. The 61,221,616-byte executable saved 306,640 B against the preceding all-in-one import table, with identical generated XAML instruction attribution (1,609,676 B). The generated image set was 543,364 B. The 211 compiled-markup tests and all nine paired screenshots matched; the median setup marker over 15 alternating launches moved from 131.3 to 130.0 ms.

Encoding ordered overload choices extends the bytecode to previously native setter dispatch. It reduced named generated XAML instructions by another 48,472 B, but grew the executable by 315,344 B. The link map assigns an additional 32,512 B to relocatable constants and 206,772 B to other Catalog instructions. Inlining can change which symbol owns a given instruction. This result measures the complete lowering and import strategy at that stage.

The whole-document Catalog saved 4,375,248 B (6.68%). The compiler produced 544,185 B of instruction images across the dependency graph, including 282,726 B in the Catalog package and 191,711 B in WinUI. Of 203 Catalog class population methods, 76 lowered to one descriptor. Across dependencies the count was 78 of 222 class documents. The separate theme-unit selection wrapper is excluded. The map attributes 1,476,192 B of remaining machine instructions to generated XAML symbols. Host import tables, strings, native framework code and inlined operations contribute additional bytes. The [application source snapshot](typed-application-snapshot.patch.gz) applies to revision `933fd4632`.

All nine whole-document pairs matched at 1,900 × 1,360 pixels across Buttons, ListBox and Calendar in Light, Dark and HighContrast. Median setup was 128.937 ms native and 129.719 ms bytecode over 15 pairs. Median time from spawn to first successful compositor draw was 268.405 ms and 271.707 ms over 11 pairs. The median paired first-draw change was +1.2%, with a 95% bootstrap interval of −2.9% to +2.1%. The application and theme benchmarks ran on the same development machine as other builds.

Seven paired navigation runs exercised ListBox, TreeDataGrid and Data Validation. Each arm recorded 2,268 draws. The median of the per-process median render times was 7.572 ms native and 7.555 ms bytecode. The corresponding 95th percentiles were 7.741 and 7.715 ms. The corresponding Catalog library run passed 936 tests with 20 ignored, including the compiled-markup comparisons. The runtime passed 278 tests, the loader passed 470 with one ignored, and the build-task and build-scanner suites passed 60 and 56 respectively. [Application measurements](typed-application-results.json)

The extension-target build resolves a typed adapter for eligible `ProvideValue` calls and encodes the scope setter and clear. It uses the original target property, the same service provider and the original source coordinates on invocation errors. A failed extension leaves the target set, matching generated native code. This stage saved 16,224 B over the whole-document executable. The linker attributed 1,415,768 B to generated XAML instructions, down by 60,424 B. All nine screenshot pairs matched. Median first successful compositor draw over 11 pairs was 315.223 ms native and 319.081 ms bytecode.

Both Hello World class documents lower to one program each, with 82 instruction bytes in total. They still use native control constructors and typed imports. The complete executable saved 6,288 B, including the runtime and reachable theme content. Across 15 process pairs its median setup marker was 88.110 ms native and 88.117 ms bytecode. Across 11 separate pairs its median first successful compositor draw was 175.516 and 186.743 ms. The median within-pair change was −2.3% with a broad bootstrap interval (−7.9% to +18.0%). These samples leave the effect on Hello's first draw unresolved.

In the [final standalone theme runs](README.md#final-full-theme-comparison), the compiler produced 191,711 instruction bytes for all of WinUI and 80,445 for Fluent. The complete full-theme executables saved 456,560 and 61,968 B respectively. In 32 alternating pairs, the median paired change per warm Button build was +144 ns for full WinUI (95% bootstrap interval +127 to +183 ns) and +8 ns for full Fluent (−28 to +28 ns). The WinUI and Fluent Button-only builds retained 16/166 and 2/75 theme units respectively. The trimmed Fluent executable grew by 49,088 B. [Raw theme measurements](typed-final-results.json)

For the 64,653,776-byte ControlCatalog, linked machine instructions attributed to generated XAML fell from 7,997,836 to 5,369,316 bytes. The complete file saved less because the program encoding, native adapters, import tables, and runtime execution also occupy space. Attribution follows the symbol holding each range and can change when code is inlined.

Fifteen alternating process pairs for that Catalog prototype reached the after-setup Light-theme message at median 184.842 ms native and 191.713 ms bytecode. This is a process milestone before the first rendered frame, measured while other builds were running. All nine framework screenshots matched across Buttons, ListBox and Calendar in Light, Dark and HighContrast, at 1,900 × 1,360 pixels. Both Hello executables opened and closed their window through the sample's timer. Fifteen fresh Hello process pairs reached its after-setup timer message at median 122.765 ms native and 123.693 ms bytecode.

The integrated binary-file build also passed all 211 compiled-markup tests, including generated document comparisons against the runtime XAML loader. The broader ControlCatalog library suite passed 936 tests, with 20 ignored. The runtime and emitter suites passed 269 and 463 tests respectively, with one ignored emitter test.

For a diagnostic ceiling on XAML-only savings, [omitting the population bodies](catalog-omission.md) removed 11,486,416 bytes and left an empty UI. For the full WinUI, Fluent, selected theme units and template instantiation measurements, see [the full-theme experiments](README.md). The [independent binding investigation](../../perf/typed-binding-size/README.md) measures a separate source change with native compiled XAML.

The matched four-build experiment combines both mechanisms on an earlier XAML snapshot. It uses the Cargo-produced executables from one checkout. Stock measured 65,859,488 B, shared bindings alone 63,839,040 B, XAML bytecode alone 61,562,624 B and both together 59,542,176 B. The combined saving was 6,317,312 B (9.59%). The marginal binding saving measured 2,020,448 B with either XAML backend. All 12 framework screenshot comparisons against stock were pixel-identical. [Combined experiment and source versions](../../perf/typed-binding-size/combined-xaml/README.md)

## What the latest Catalog executable retains

Resolved property conversions, static fields and dictionary capacity increased the number of complete class-document programs from 83 to 113 of 222 across dependencies, including 106 of the Catalog package's 203 documents. The remaining documents combine bytecode and native operations. The compiler produced 566,785 bytes of instructions, including 293,435 for Catalog and 199,024 for WinUI. Across the dependency graph the 1,170 construction tables contain 36,325 operation entries and 8,750 string references. Their arm64 layouts total 758,640 bytes before folding and trimming. The observations below use the stock 65,876,016-byte link map and the 60,924,816-byte candidate's own map. The matched normalized native executable used to measure the 4,573,328-byte saving was 65,498,144 bytes.

| Linked machine instructions identified by symbol | Archived native Catalog | Resolved-writes Catalog |
| --- | ---: | ---: |
| Functions named directly in generated XAML modules | 7,997,836 B | 1,185,788 B |
| Trait implementations whose receiver belongs to a generated module | 1,516 B | 1,771,088 B |
| Other symbols whose names contain a generated module path | 358,196 B | 296,860 B |
| Sum of these disjoint ranges | 8,357,548 B | 3,253,736 B |

The last column contains 1,953,532 B of functions whose symbol names specifically reference construction import tables. This subtotal spans all three rows. Decoder functions, constants, strings, framework implementations and inlined code can have other names. A map cannot tell how much would disappear if two imports shared an implementation. A matched link is needed to measure that saving. [Stock map census](catalog-allocation.json) · [Candidate measurements and source hashes](typed-application-results.json)

The candidate passed 936 Catalog library tests with 20 ignored, including all 211 compiled-markup comparisons. The runtime passed 281 tests and the loader passed 471 with one ignored. An initial screenshot run differed on seven of nine pages and recorded a window activation change. In a fresh three-way capture, the same nine 2,200 × 1,524 images matched exactly between native, the preceding bytecode build and the new binary. Across 11 alternating launches, median first compositor draw was 272.0 ms native and 273.5 ms bytecode. The median paired change was −0.5%, with a 95% bootstrap interval from −0.7% to +5.8%. Across 15 other pairs the after-setup medians were 128.3 and 125.2 ms. These measurements establish no clear startup difference. [Raw samples and both capture runs](typed-application-results.json) · [Compiler and runtime snapshot](typed-resolved-writes-snapshot.patch.gz)

### Sharing property-write operand decoding

One instruction now reads the target object, source register and source coordinates for 1,735 property-write import entries across the Catalog dependency graph. The native adapter receives those operands and applies the original resolved conversion and property write. Across otherwise matched links this saved another 65,680 B. The instruction images shrank by 3,043 B to 563,742 B. Machine code in symbols naming the construction imports fell by 62,868 B, from 1,953,532 to 1,890,664 B. The 113 whole class documents, including 106 Catalog pages, remained whole. Full WinUI contributed 198,490 instruction bytes. Nine screenshots matched and all 936 enabled Catalog tests passed, with 20 ignored.

Median first compositor draw on the Catalog was 336.0 ms native and 320.6 ms typed across 11 pairs. The 95% bootstrap interval for the paired change was −13.3% to +1.5%. The corresponding Hello World medians were 222.4 and 213.1 ms, with an interval from −9.7% to +3.6%. Other builds were active on this host. Both Hello documents compile to one program each. [Application results](typed-application-results.json) · [Matched compiler snapshot](typed-shared-setter-snapshot.patch.gz)

Catalog frame timing showed a host effect. In seven paired navigation runs the median of the process medians was 7.786 ms native and 0.481 ms typed. Running the identical native executable in both positions reproduced that split (7.7165 and 0.408 ms). A repeat native/typed run gave 7.963 and 7.947 ms. These runs provide no evidence of a sustained rendering speedup. The frame report includes every process median and the native control. Two navigation marker lines in the initial run shared a line with compositor output. Re-reading the intact markers recovered the missing navigation phases without changing any recorded frame sample. [Measurements](typed-application-results.json)

## Comparison with the November 17 binary-size review

The independent review measured revision `74ed59e571a3` using macOS Quartz Hello World. Enabling broad framework type registration with the same 16 WinUI units grew its executable from 11,437,792 to 24,002,720 bytes. A separate copied-app comparison with broad registration disabled grew by 8,548,400 bytes when retaining all 166 units. These experiments overlap in the code they make reachable. The review did not run UI or timing tests on those links.

The corresponding release ControlCatalog builds here use Skia and all 166 WinUI units. Their target dependency graph reaches `ferroui-base/full-themes` through `ferroui-diagnostics`, which also enables `type-tables`. The Catalog attaches its F12 tool window under debug assertions. Broad registration still applies across the release dependency graph. A behavior-preserving experiment can share the repeated member invocation logic while keeping the discoverable types, members and their behavior. The 12,564,928-byte Quartz figure cannot be added to the measured Catalog savings. [Desktop dependency](../../../samples/ControlCatalog.Desktop/Cargo.toml) · [Diagnostics dependency](../../../src/FerroUI.Diagnostics/Cargo.toml) · [F12 attachment](../../../samples/ControlCatalog.Desktop/program.rs)

A physical census of the archived Catalog found 832,408 B in type and member descriptors and 1,547,260 B in the directly referenced callback functions. These overlap the existing constant and instruction categories. Among them, 348 registered methods take a sender and routed event and return void. Their 345 distinct invokers occupy 227,284 B. A matched 32-method fixture shared argument handling, retained registered conversion and error behavior, and saved 16,528 file bytes. [Registration census and invocation measurements](../../perf/typed-binding-size/registration-review/README.md)

In the separate default native-XAML desktop Catalog link, sharing this logic for 340 routed methods took the executable from 65,859,504 to 65,760,720 B, saving 98,784 B. The candidate retained all 1,138 registered method identities and signatures and the original diagnostic settings. DrawerPage, TabbedPage and ListBox rendered identically. This change was tested in isolation from the XAML backend. The 32-method invocation timings apply only to the fixture. [Full Catalog comparison and exact patch](../../perf/typed-binding-size/registration-review/README.md)

The review also found an ordinary Rust row table made its isolated Material palette executable 65,872 bytes larger even as machine code shrank. A packed record with a shared key pool saved 16,688 file bytes in the same test. Material is optional in ControlCatalog. This evidence supports measuring the cost of an import together with the code and data it keeps. The full and Button-only theme links provide that comparison for this backend. [Full-theme and trimming results](README.md#theme-unit-instructions-and-imports)

## Binary geometry literals

Previously, a path such as an icon's `Data` attribute emitted a constant Rust `PathCommand` array and a native call to build the geometry. The typed backend now records the framework parser's commands at build time and emits them as the operands of an object-literal instruction. Each command uses a tag byte with inline flags. Small whole coordinates use one byte, and other coordinates retain the original binary64 bits. The importer streams the commands into the same geometry builder, creating a fresh object on each execution. The import belongs to the factory containing the literal. The native backend still uses the command array.

This change saved another **114,912 B** in the real desktop Catalog. The linked file fell from 60,859,136 B to 60,744,224 B. The instruction images grew from 563,742 to 625,785 B as native constants moved into them. The link map attributed 66,496 B less to other read-only constants and 34,028 B less to directly named generated XAML instructions. Complete class documents increased from 113 to 124 of 222 across dependencies, including 117 of 203 in the Catalog package. The full WinUI image was 208,846 B. All 211 native/loader compiled-markup comparisons passed, alongside 279 markup-library, 282 runtime and 471 loader tests, with one loader test ignored. [Source snapshot](path-geometry/snapshot.patch.gz) · [Linked allocation, component counts and tests](path-geometry/results.json)

The complete Catalog suite passed 936 tests, with 20 ignored. Eighteen desktop screenshots matched native pixel-for-pixel across six pages in Light, Dark and HighContrast, including IconElement, DrawerPage and TabbedPage. Hello World went from 16,290,272 B native to 16,250,928 B typed. Across 11 process pairs, median first compositor draw was 209.9 ms native and 200.0 ms typed for Hello World and 432.2 ms and 407.6 ms for the Catalog. Build load affected these measurements.

| Theme workload | Linked units in both arms | Native executable | Geometry-bytecode executable | File change | Paired change per repeated Button build (95% bootstrap interval) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full WinUI | 166 / 166 | 18,578,224 B | 18,055,312 B | −522,912 B | +143.5 ns (+115 to +171) |
| WinUI Button | 16 / 166 | 10,261,696 B | 10,222,288 B | −39,408 B | +139.5 ns (+98 to +176.5) |
| Full Fluent | 75 / 75 | 15,567,888 B | 15,456,272 B | −111,616 B | −26 ns (−55 to +7) |
| Fluent ambient + Button | 2 / 75 | 9,973,872 B | 10,022,896 B | +49,024 B | +27.5 ns (+2 to +44.5) |

The full Fluent instruction image occupied 92,693 B. Both full inventories matched native, and the trimmed probes verified selected resources and controls. The Button timings use 32 alternating process pairs with 128 constructions per process.

The geometry decoder added work when a path was constructed. A separate probe exercised the three literal DrawerPage icons in source order using the same headless builder in both arms. Each process built its selected icon 1,024 times after 64 warm-ups. Across 31 alternating pairs, the 13-, 13- and 46-command icons cost a paired median of **75, 87 and 384 ns more per construction** respectively. Their native command arrays occupied 728, 728 and 2,576 B, and their instruction payloads occupied 37, 87 and 1,466 B. Every decoded command matched the original parser result. The rendering comparison used the real desktop application. [Raw application, theme and icon results](path-geometry/results.json)

To reproduce the icon timings, run `prepare.py` in this directory to create the standalone probe template, then run `python3 scripts/themes/full-opcode-spike/path-geometry/measure.py --build-in target/path-probe target/path-probe/timings.json` from the repository root.

## Converters, checked writes and list operands in ordinary pages

The interpreter now passes the active document to image converters, performs the resolved runtime cast and preserves its XAML error position. A subsequent checked property write retains the original member name, argument index and location. The compiler can also encode typed runtime lists, their ordered additions, parent-stack pushes of value objects and resolved type descriptors. Deferred bindings and templates receive their original parent context.

| Matched macOS arm64 ControlCatalog build | Executable | Change from previous row | Compiler instruction images | Whole Catalog pages out of 203 |
| --- | ---: | ---: | ---: | ---: |
| Geometry bytecode | 60,744,224 B | — | 625,785 B | 117 |
| Document-aware converters and casts | 60,711,392 B | −32,832 B | 626,470 B | 117 |
| Checked property writes | 60,645,600 B | −65,792 B | 627,677 B | 128 |
| Typed lists, parents and descriptors | 60,595,952 B | −49,648 B | 632,129 B | 144 |

The last build saved **4,902,192 B (7.48%)** against the 65,498,144 B normalized native Catalog, with 153 of 222 class documents fully encoded across the dependency graph. The compiler image column includes every generated module in that graph. It excludes the native adapters, their import tables and strings, and can include unlinked programs. All 18 screenshot pairs matched across Buttons, ListBox, Calendar, Image, ComboBox and Viewbox in Light, Dark and HighContrast. Median first successful compositor draw across 11 pairs was 345.0 ms native and 345.5 ms bytecode. Hello World measured 16,290,272 B native and 16,250,912 B bytecode, with median first draw of 184.1 and 195.6 ms.

| Theme case | Linked units in both arms | Executable change | Paired change per repeated Button construction, median (95% bootstrap interval) |
| --- | ---: | ---: | ---: |
| Full WinUI | 166 / 166 | −522,992 B | +183 ns (+145 to +249.5) |
| WinUI Button | 16 / 166 | −39,408 B | +70 ns (+8 to +126) |
| Full Fluent | 75 / 75 | −111,696 B | +9.5 ns (−16 to +42) |
| Fluent ambient + Button | 2 / 75 | +49,024 B | +21.5 ns (−37.5 to +74.5) |

Full-theme resource inventories matched byte for byte. Each template measurement used 32 alternating process pairs and 128 Button constructions per process. The retained trimmed selections matter: a small Fluent application can grow even while a full-theme application shrinks. [Candidate sizes, image counts, raw timings and screenshot results](typed-page-operands-results.json) · [Exact source snapshot](typed-page-operands-snapshot.patch.gz)

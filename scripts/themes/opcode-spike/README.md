# Binary XAML execution spike

From the repository root run `python3 scripts/themes/opcode-spike/run.py` after a macOS arm64 dist build. Set `SPIKE_DIST` to use another dist directory. All generated artifacts go in `target/opcode-spike`. Python reads the checked-in WinUI Button XAML and emits the complete body of DefaultButtonStyle's ControlTemplate twice. One lowering is an embedded binary document and one is straight-line Rust. Both use the same real FerroUI operations. The comparison measures the cost of changing the instruction representation, including any differences caused by native constant folding. The native reference is the spike’s straight-line emitter.

The document starts with FXOP and an ABI version byte. Single-byte instructions construct and initialize a presenter, set and register its name, install template bindings by numeric target/source property IDs, set a template-priority boolean, construct a collection and transition, set its target and signed ticks, append it and attach the collection, then end initialization. This slice uses implicit typed slots for the presenter, collection and transition, with their constructors specialized by opcode. Property IDs refer to direct typed host accessors. Only the binary build contains the interpreter. Black-boxing the byte slice keeps LLVM from specializing the interpreter for the embedded document.

The first timer covers the first template-body build of a fresh process. Repeated timing includes building and dropping a fresh object and name scope each iteration. Both exclude dynamic loader launch, style lookup, template-provider setup, attaching the parent, layout and painting. Separate checks exercise lookup by registered name, identity, access-key value, transition identity by property, duration and two padding changes after installing the templated parent. Each mode runs nine times, alternating order, with 256 repeated builds per process. Native and binary builds use the same rustc flags and precompiled dependencies.

## Proposed document and execution model

Compile the existing XAML front end’s resolved operations into independently addressable documents. Resolve names, types, overloads and binding paths at build time. A document describes its own control tree, styles, selectors, templates and resource factories entirely as data. A shared native host supplies operations such as constructing a Button, setting a typed property, or connecting a code-behind handler.

| Section | Purpose |
| --- | --- |
| Header and entry-point directory | Record the format version, host ABI and byte ranges of the root, templates and deferred resources. |
| Per-unit imports | Link only the constructors, typed property accessors, collection operations, converters, handlers and binding accessors used by that unit. An app’s existing theme-unit selection can still discard unused imports. |
| Constants and indexes | Intern resource keys and literals. Give each resource variant, selector chain and style a range in the document. |
| Instructions | Create objects in typed slots, set properties with their original priority, add children, register names and create bindings. Dedicated instructions can handle frequent runs of template bindings. |
| Source map | Store instruction-offset to document/line/column mappings separately. Consult them when an operation reports an error. |

Map the compiled bytes and enter the requested byte range directly. Register deferred resource and template entry points as offsets with their declaration context. On first lookup or template application, the interpreter calls the same native property and binding APIs used by compiled XAML today. Each build has its own object slots, initialization sequence and namescope. Theme changes and binding updates continue through the framework’s normal notifications. Keep cold resource entries in the mapped format to avoid maintaining a second, expanded copy in heap memory.

The prototype uses trusted, compiler-produced input and two narrow host implementations. A general backend needs a generated import ABI and validation of operand types, stack or slot usage, and entry-point bounds. The next integration target is a complete catalog page and its transitive resources, measured in the shipping executable. Compare the object tree, resource errors and positions, binding updates, and screenshots across light, dark and high contrast. Time first navigation, repeated template realization and updates separately.

## Catalog tree comparison

Run `python3 scripts/themes/opcode-spike/run.py catalog` for a second, independently linked workload. Its input is the complete WrapPanel inside the ToggleButton sample section of `samples/ControlCatalog/Pages/ButtonsPage.xaml`. It builds that panel, applies both spacing properties, constructs three real ToggleButtons, applies their string and boolean properties, registers the named button and appends all three children. Validation checks the panel, child order and type, name identity, text, checked, three-state and enabled values. The measurement covers this subtree and its own property assignments. It uses a second, independently linked host implementation. The document-count sweep below measures interpreter reuse.

## Measurements on arm64 macOS

Template-only executables used 7,207,184 bytes for straight-line Rust and 7,207,216 bytes for the 80-byte document, interpreter and shared host. The Apple linker map attributes 3,416 and 4,064 bytes respectively to each build function, including inlined helpers. The 32-byte file delta reflects segment alignment, dead stripping, symbol layout and signing. Comparing the 80 bytes directly with all 3,416 native bytes would omit the VM cost. Median first build and subsequent build/drop were 271.959 µs and 1.766 µs for native, 278.750 µs and 1.885 µs for bytecode. A preceding timing round showed 254.833/1.761 µs and 272.084/2.068 µs respectively. Concurrent builds and process startup variation limit the precision of small timing deltas.

The catalog subtree used an 80-byte payload. Native and bytecode executables occupied 7,168,256 and 7,168,304 bytes respectively. Their live Mach-O instruction sections occupied 5,457,016 and 5,458,296 bytes. Median first/repeat times were 268.167/2.927 µs for native and 268.000/3.089 µs for bytecode. The binary subtree built and dropped in 0.162 µs more on repeat, approximately 5.5%. Neither single-subtree binary paid back its interpreter footprint in these executables.

## Size across multiple bodies

Run `python3 -B scripts/themes/opcode-spike/sweep.py` to compare linked executables with 1, 8, 32 and 128 **synthetic** documents built by the same catalog emitter. Each document has a WrapPanel and 2–5 ToggleButtons. Names and text are unique, spacing differs, and the presence and ordering of setters vary. The native executable dispatches to separately emitted straight-line functions. The bytecode executable contains one catalog interpreter and a concatenated byte stream with a u32 offset per entry plus a final end offset. Both executables create and verify every document’s typed controls, child order, names, content, properties and spacing. The independently computed expected values use identical code in both builds. Section measurements come from Apple's `size -m`. The sweep reports the instruction section separately from the stripped, ad-hoc-signed executable including all sections. Its synthetic document family cannot predict the distribution of shapes, resource types or host imports in an entire application.

| Documents | Binary payload B | Native file B | VM file B | VM − native file B | VM − native instructions B | VM − native file-backed section B |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 70 | 7,151,168 | 7,167,648 | +16,480 | +1,408 | +2,490 |
| 8 | 762 | 7,167,632 | 7,167,648 | +16 | −704 | +698 |
| 32 | 3,162 | 7,167,712 | 7,167,648 | −64 | −5,824 | −3,214 |
| 128 | 12,932 | 7,184,368 | 7,167,648 | −16,720 | −23,808 | −16,270 |

File-backed section totals sum named Mach-O sections, excluding zero-fill sections, segment padding and link-edit data. Payload bytes exclude the offset index, which occupies 516 bytes at 128 documents. Both are included in the linked VM file. At 128 documents the corpus constructed 448 ToggleButtons across 128 WrapPanels and validated them all in each executable. VM instructions stayed at 5,454,544 bytes from 8 through 128 documents. The native instruction section grew from 5,455,248 to 5,478,352 bytes over that range. The summed file-backed sections crossed over between the measured 8- and 32-document points.

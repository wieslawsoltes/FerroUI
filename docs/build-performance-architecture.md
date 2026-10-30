# Build compiler architecture

**Proposal; not implemented or benchmarked.** This review uses the source at
`98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`. It complements the
[performance guide](build-performance.md), [CI audit](build-performance-ci-audit.md)
and [research review](build-performance-research.md).

The next architectural improvement is to remove the UI runtime from the
model-based XAML compiler's host dependencies. Extract shared parsing and
assembly metadata into a small reusable crate, while retaining the existing
framework value types, public paths and generated member functions. Stock Cargo
can then build a smaller host graph. This is a separate change from the measured
optimizations; no numerical speedup is forecast here.

## The dependency boundary

The [build task manifest](../src/FerroUI.Build.Tasks/Cargo.toml) selects the
loader's `compiler` feature. The [loader manifest](../src/Markup/FerroUI.Markup.Xaml.Loader/Cargo.toml)
already makes its runtime backend and XAML runtime library optional, but retains
two unconditional runtime-crate dependencies:

| Edge in the model-based build tool | Why it exists today | Proposed replacement |
|---|---|---|
| `ferroui-build` → `ferroui-base` | `MarkupAssembly` and metadata attribute names | Shared assembly metadata |
| `ferroui-build` → loader `[compiler]` → `ferroui-base` | Value parsing, URI operations, binding grammar, query operator and metadata names | Shared parsing and syntax |
| Loader → `ferroui-markup` → `ferroui-base` | Selector, container-query and property-reference grammars | Shared grammars; remove the loader's direct Markup dependency |

The [Markup manifest](../src/Markup/FerroUI.Markup/Cargo.toml) depends on Base for
its runtime bindings and parser adapters. Those runtime parts remain there.
The loader has no other production `ferroui_markup::` references: its three
grammar imports are the complete direct dependency to replace.

Removing these edges takes the Base and Markup host libraries out of the
model-based compiler's prerequisites. Build scripts that generate XAML would no
longer wait for those libraries solely to obtain parsers. A Base runtime-only
implementation edit would also cease to invalidate this host compiler through
that dependency. Any duplicate host metadata-export work exists only if the
selected Cargo graph currently needs a distinct host unit; verify this from
artifacts rather than assuming every host/target use is duplicated.

Target Base, Controls, required metadata exports, generated Rust compilation and
linking remain. Other selected tools may still independently require Base on the
host. Parallel overlap determines how much critical-path time disappears, so
individual crate durations cannot simply be added into a wall-time saving.

## Actual compiler imports

The following are compiled host dependencies, rather than strings naming types
in generated target code. Outside runtime/test modules, the loader's actual Base
and Markup references are confined to the eleven files listed below.

| Consumer | Values or APIs used on the host |
|---|---|
| Build [`lib.rs`](../src/FerroUI.Build.Tasks/lib.rs) | `MarkupAssembly`, assembly name and defaults; public `Build::assembly`, `compile_xaml` and `embed_assets` APIs |
| Build [`model_type_system.rs`](../src/FerroUI.Build.Tasks/type_system/model_type_system.rs) | Metadata attribute name constants |
| Loader [`core_table.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/core_table.rs) | Metadata attribute name constants |
| [`ferro_xaml_il_well_known_types.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/transformers/ferro_xaml_il_well_known_types.rs) | Framework XML namespace constant |
| [`ferro_xaml_il_language_parse_intrinsics.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/ferro_xaml_il_language_parse_intrinsics.rs) | Geometry, relative point, color, TimeSpan, URI and numeric parsers |
| [`xaml_include_group_transformer.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/group_transformers/xaml_include_group_transformer.rs), [`transform.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/transform.rs), [`compiled.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/compiled.rs) | URI validation and relative include/group resolution |
| [`ferro_xaml_il_binding_path_parser.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/transformers/ferro_xaml_il_binding_path_parser.rs), [`xaml_il_binding_path_helper.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/xaml_il_binding_path_helper.rs) | `BindingExpressionGrammar`, `Node` |
| [`ferro_xaml_il_selector_transformer.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/transformers/ferro_xaml_il_selector_transformer.rs) | `SelectorGrammar`, `SelectorSyntax` |
| [`ferro_xaml_il_query_transformer.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/transformers/ferro_xaml_il_query_transformer.rs) | `ContainerQueryGrammar`, `ContainerQuerySyntax`, `StyleQueryComparisonOperator` |
| Loader [`parsers/mod.rs`](../src/Markup/FerroUI.Markup.Xaml.Loader/parsers/mod.rs) | `PropertyParser`, `PropertyReference` |

The Base paths in the emitter's generated strings, the core handle table and
Build's [`emit_types.rs`](../src/FerroUI.Build.Tasks/type_system/emit_types.rs)
remain unchanged. Emitting `::ferroui_base::Ref`, `TypeInfo`, `FerroProperty`,
`MarkupTyped` or `IServiceProvider` does not itself require the generator to link
those types. The runtime backend and emitter runtime host do use their concrete
types and retain the corresponding dependencies.

## Smallest practical extraction

Use one leaf crate, provisionally `ferroui-markup-core`, with no dependency on
Base, Controls, Markup, the loader or the scanner. Its name and public packaging
are implementation decisions to review. The existing
[`ferroui-system`](../src/FerroUI.System/lib.rs) extraction demonstrates the
project's convention of preserving old public paths through re-exports; putting
UI grammar into that crate is not required for this proposal.

### Stage 1: shared metadata and syntax

Move [`metadata/markup_assembly.rs`](../src/FerroUI.Base/metadata/markup_assembly.rs)
and the `attributes` constants from
[`metadata/markup_type.rs`](../src/FerroUI.Base/metadata/markup_type.rs).
`MarkupAssembly`, `XmlnsDefinition`, `XmlnsPrefix`, namespace/key constants and
the assembly registry need only the standard library. Re-export them at the
existing Base paths. Retain one actual type and one actual registry, so runtime
registration and build APIs observe the same data.

Move these syntax implementations and keep their existing modules as re-exports:

- Base [`character_reader.rs`](../src/FerroUI.Base/utilities/character_reader.rs),
  [`span_helpers.rs`](../src/FerroUI.Base/utilities/span_helpers.rs),
  [`span_string_tokenizer.rs`](../src/FerroUI.Base/utilities/span_string_tokenizer.rs)
  including `FormatError`, and
  [`expression_parse_exception.rs`](../src/FerroUI.Base/data/core/expression_parse_exception.rs).
- Base [`binding_expression_grammar.rs`](../src/FerroUI.Base/data/core/parsers/binding_expression_grammar.rs),
  including its `Node` and `SourceMode` types.
- Markup [`selector_grammar.rs`](../src/Markup/FerroUI.Markup/markup/parsers/selector_grammar.rs),
  [`container_query_grammar.rs`](../src/Markup/FerroUI.Markup/markup/parsers/container_query_grammar.rs)
  and [`property_parser.rs`](../src/Markup/FerroUI.Markup/markup/parsers/property_parser.rs).

Leave `expression_node_factory.rs`, `selector_parser.rs` and
`container_query_parser.rs` in the runtime crates: they construct runtime objects
and resolve framework properties. Moving nearby property-path or argument-list
grammars is optional and does not remove another compiler dependency.

The shared container grammar and public loader AST also require
[`StyleQueryComparisonOperator`](../src/FerroUI.Base/styling/style_query_comparison_operator.rs).
Move its single definition and re-export it from Base, subject to the source-tool
work below. Duplicating the enum would give compiler and runtime callers
incompatible types.

### Stage 2: shared parsing kernels, unchanged framework value types

Keep the public value types in Base and delegate their current methods to shared
functions. The compiler calls those same functions to obtain primitive values.

| Existing Base implementation | Shared result |
|---|---|
| [`Thickness`](../src/FerroUI.Base/thickness.rs), [`CornerRadius`](../src/FerroUI.Base/corner_radius.rs) | Four components, with each type's existing ordering and diagnostics |
| [`Point`](../src/FerroUI.Base/point.rs), [`Vector`](../src/FerroUI.Base/vector.rs), [`Size`](../src/FerroUI.Base/size.rs) | Two components |
| [`Matrix`](../src/FerroUI.Base/matrix.rs) | All matrix components; preserve current six/nine-component parsing and edge behavior |
| [`RelativePoint`](../src/FerroUI.Base/relative_point.rs) | Coordinates and relative/absolute unit data |
| [`TimeSpan`](../src/FerroUI.Base/animation/time_span.rs) | Ticks, preserving range checks and error text |
| [`Color`](../src/FerroUI.Base/media/color.rs) | ARGB value, using shared HSL/HSV conversion and named-color lookup |
| [`Uri`](../src/FerroUI.Base/utilities/uri.rs) | Shared internal URI representation and operations, wrapped by the existing Base type |

Color is the largest part of this stage. Its parser also uses
[`hsl_color.rs`](../src/FerroUI.Base/media/hsl_color.rs),
[`hsv_color.rs`](../src/FerroUI.Base/media/hsv_color.rs) and
[`known_colors.rs`](../src/FerroUI.Base/media/known_colors.rs). Move the parsing,
conversion and lookup algorithms once; retain delegates for all existing callers.
Keep culture-aware public formatting APIs in Base. A second compiler-only color
parser would risk different accepted inputs or rounding.

URI operations need the same treatment. The compiler's uses above are internal,
so it can consume the shared URI representation while Base retains its public
`Uri` type, methods and markup-generated functions. Preserve `UriKind` and error
compatibility at the existing public boundary; their adaptation must not change
accepted strings, normalization, equality, relative resolution or diagnostics.

### Stage 3: remove the host edges

After all actual imports have replacements:

- Make Base optional in `ferroui-build`; `runtime-host` enables it. Keep
  `Build::assembly`, the free helper functions and `TypeSystem::Runtime`
  available as before, including the existing disabled-runtime-host error.
- Make Base optional in the loader; `runtime` enables it and `emitter` continues
  to enable runtime plus Base's `compiler-metadata` support.
- Remove the loader's direct Markup dependency. Add the leaf dependency to Base,
  Markup, the loader and the build task as required.
- Retain explicit development dependencies for tests that intentionally use
  real framework types. Test feature combinations separately so those
  dependencies cannot conceal an accidental production host edge.

The resulting model compiler keeps XamlX, the scanner, XML/JSON dependencies and
the leaf crate. The runtime loader and runtime emitter host retain their full
features. Cargo's [optional dependencies and feature unification](https://doc.rust-lang.org/cargo/reference/features.html)
and [resolver behavior](https://doc.rust-lang.org/cargo/reference/resolver.html#feature-resolver-version-2)
make explicit graph checks necessary: disabling a feature on one edge does not
override an independent request that enables it.

## Compatibility work required before accepting the split

### Inherent markup functions require locally defined value types

[`ferro_markup_type!`](../src/FerroUI.Base/metadata/markup_macros.rs) generates
inherent `impl Type` blocks containing `__MARKUP_VALUE` and `__markup_*` members
when the relevant features are enabled. Rust requires an inherent implementation
to be in the crate defining its type; see the
[Rust Reference](https://doc.rust-lang.org/reference/items/implementations.html#inherent-implementations).
Moving Color, geometry, TimeSpan or Uri wholesale and re-exporting them would
therefore break these implementations. Retaining their definitions and extracting
parsing functions preserves this source convention and the typed generated calls.

### Re-exported enum identity must survive source scanning and reflection

The source tools currently have a narrower view than rustc:

- [`scanner/modules.rs`](../src/FerroUI.Build.Scan/scanner/modules.rs),
  `public_paths()`, ignores external items when collecting shortest public paths.
- [`scripts/rust_paths.py`](../scripts/rust_paths.py) explicitly excludes types
  defined by other crates from its generated public-path table.
- Base declares `StyleQueryComparisonOperator`'s markup metadata in
  [`markup_types/enums.rs`](../src/FerroUI.Base/markup_types/enums.rs).

Support this re-exported enum's canonical identity and public Base path in both
tools, and verify its discriminants and metadata projection. Do not silently
accept an opaque type or drop it from reflection. The plain-enum macro implements
Base's local metadata trait; it does not have the value-type inherent-impl issue.
Its source-path and enumeration-value handling still require explicit validation.

### Assembly constants have a hard-coded external-crate assumption

[`scanner/mod.rs`](../src/FerroUI.Build.Scan/scanner/mod.rs), `text_value()`,
recognizes external `FERRO_XML_NAMESPACE` and `MarkupAssembly` key constants
specifically through `ferroui_base`. Account for resolution through the new leaf
crate while preserving existing Base paths and source declarations. Sharing the
constant definitions with the scanner is possible because the leaf has no
dependency back to it. Do not bypass unresolved metadata diagnostics.

## Correctness and performance gates

Each stage should be independently reviewable, with no change to accepted source
declaration forms or feature availability. Before accepting the final split:

1. **Graph:** prove that a standalone model compiler build has no Base, Markup or
   Controls host library; inspect both Cargo's dependency graph and compilation
   artifacts. Confirm the complete runtime graph when its features are enabled.
2. **APIs:** compile existing public paths and calls in compiler-only, runtime,
   emitter and `runtime-host` configurations, plus `markup-functions` and
   `compiler-metadata`. Check shared enum/error/assembly type identity and that
   assembly registration reaches the same registry through either public path.
3. **Parsing:** retain the existing parser suites and compare valid outputs and
   invalid-input diagnostics. Cover Unicode columns, malformed tokens, numeric
   boundaries, signed zero/non-finite values where accepted, color rounding and
   URI case/query/fragment/relative-path behavior. Runtime and compiler must call
   the same implementation.
4. **Metadata:** run the existing
   [runtime/model drift test](../tests/FerroUI.Markup.Xaml.UnitTests/type_system_drift.rs),
   verify generated public-path tables, and compare complete model outputs.
   Inventory/path changes caused by moving source files must be reviewed;
   type/member/enum/namespace semantics must be preserved.
5. **Code generation and behavior:** compare generated Rust and compiled-document
   metadata for themes, class documents and include fixtures. Compile and execute
   their tests, including runtime-loader and generated-code conformance.
6. **Performance:** use the [whole-build harness](../scripts/build-perf/compare.py)
   with the same toolchain, selected features and target settings. Its current
   unit-equality guard deliberately rejects changed graphs; add a narrowly
   reviewed expected graph-difference check for this refactor rather than
   disabling comparison. Record cold build-tool cost, warm rebuilds after Base
   edits, actual removed units, generated-output correctness and wall time.

The goal is a smaller and less coupled host compiler. Whether the resulting
critical-path improvement justifies the extraction remains a measured acceptance
decision, separate from scanner execution, cache replay and whole-CI speedups.

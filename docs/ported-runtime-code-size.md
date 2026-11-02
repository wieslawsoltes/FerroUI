# Machine-code costs of the .NET-to-Rust object model

The largest identified family in the complete editable WinUI executable is the dependency-property implementation. A property value can use one generic type parameter in the source and still generate multiple functions for each Rust value type in the linked application. The CLR can share generic machine code across reference-type instantiations. Rust ordinarily monomorphizes each reachable concrete instantiation, including distinct handle types such as `Ref<Button>` and `Ref<Border>`. Inlining and linker folding can subsequently share or eliminate some of that output. [CLR generic code sharing](https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/generics/generics-in-the-run-time) · [rustc monomorphization](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html)

## Physical machine code in the compact whole-program build

The editable FatLTO executable at `ff5ea13a` has 20,307,877 bytes of `.text`. A separate symbolized link reproduced the address, size and SHA-256 of that section. The following counts assign physical ranges once, including when several names resolve to a single address.

| Family | Machine code (bytes) |
|---|---:|
| Property registration, stores and observers | 4,857,401 |
| Callable entries and trait shims | 2,342,641 |
| Generated native XAML | 2,196,404 |
| Other XAML runtime and codecs | 1,313,297 |
| Bindings, reactive operators and events | 1,119,874 |

`StyledProperty::route_bind` accounts for 405,105 bytes across 260 physical bodies. In the largest group, 75 instances each occupy 1,605 bytes. They use the same instruction sequence with different operands for the observer and binding-entry constructors, entry vtable, metadata lookup and local-observer vtable. Those bodies also execute the store lookup and insertion. Factoring out a generic call requires retaining that behavior, especially subscription order and reentrant completion.

Following the actual property descriptor slots found 959 distinct getter/setter functions totaling 327,694 bytes across 241 owners. A shared receiver adapter saved 102,896 bytes in a fixture with many members per owner. With two properties per owner it saved 12,832 bytes but cost 2.4% in getter CPU (paired 95% interval 0.5–5.3%). The median production owner has three accessor references. The measurements favor keeping the existing accessor ABI for the current release.

[Exact symbol and unwind attribution](benchmarks/compiled-theme-whole-program-attribution-linux-x86_64.json)

## What changed in the linked executable at `71d44ae8`

The same full editable workload was linked with safe ICF, RELR and panic unwinding at both revisions. Each byte of machine code is assigned once. The unwind columns sum frame records and their eight-byte search-index entries. A frame beginning at an aliased address is attributed to a shared category when ownership is ambiguous. The separate landing-pad table can share records across several functions, so the raw report identifies storage spans separately.

| Symbol family | Previous machine code (B) | Current machine code (B) | Previous frame and index (B) | Current frame and index (B) |
|---|---:|---:|---:|---:|
| Property registration, stores and observers | 7,227,223 | 6,710,733 | 1,880,964 | 1,786,444 |
| Bindings, reactive operators and events | 1,540,735 | 1,544,111 | 320,992 | 322,072 |
| Generated native XAML functions | 2,462,821 | 2,379,871 | 257,568 | 256,420 |
| Reflection and value conversion | 998,815 | 918,429 | 365,512 | 407,828 |
| Class projection, initialization and virtual dispatch | 631,795 | 594,637 | 200,684 | 182,624 |
| Compiler-generated destruction | 1,466,900 | 1,477,858 | 657,940 | 660,376 |
| Callable entries and trait shims | 1,300,064 | 1,170,556 | 656,000 | 599,692 |

The reflection and conversion functions occupy 80,386 fewer machine-code bytes, while their attributed frame and search-index bytes increase by 42,316. Compiler-generated destruction grows by 10,958 machine-code bytes. The property family alone loses 516,490 machine-code bytes and 94,520 frame and index bytes. Categories reflect outer linked symbols, so inlining and cross-category folding can move costs between rows.

The property implementation shared disposal, state changes and notification scheduling across concrete value types. Its private dispatch table fell from 20 slots to 8. Typed value and comparison operations remain in their original type. The existing property route now creates missing entries, and coercion metadata directly owns the registered callback. In a separate full Base registry fixture, changing only the dispatch implementation saved 203,064 ordinary ELF bytes and 148,888 compact ELF bytes. The mixed read/write CPU median was nearly unchanged in this isolated test, while style installation, read and disposal took 4.1% longer. [Effective values](../src/FerroUI.Base/property_store/effective_value.rs) · [Value store](../src/FerroUI.Base/property_store/value_store.rs)

Conversion registration shares the normal conversion, assignability and borrowed-object routes whenever they refer to the same operation. Captured user conversions keep their ownership and independent replacement semantics. Nullable facts now share one record. Primitive pair conversion shares a checked operation between ordinary and nullable destinations. A 38-control-class fixture plus FerroObject registered in 38.31 µs versus 77.85 µs, with 17 allocations versus 349. Boxed lookup rose from 43.22 to 44.20 ns while borrowed lookup fell from 41.67 to 39.57 ns. All 4,872 differential numeric records matched, including exact types and floating-point bits. [Value conversion registry](../src/FerroUI.Base/data/core/value_type.rs)

Inherited virtual slots share a whole prefix when the generated override declarations prove it is unchanged. Each class still owns its table and most-derived type. Unspecified handwritten overrides take the existing builder. Repeated constant template assignments share a typed routine local to each XAML document, after resource pruning. In the full WinUI source 1,466 assignments share 88 routines. A Button-rooted generation retained 19. The 131-document compiler corpus checked generated code and position maps. [Class construction](../src/FerroUI.Base/type_system.rs) · [Template assignment lowering](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/template_values.rs)

Plain-property binding metadata now asks for typed accessors lazily. The previous link had 377 distinct hook bodies that always declined the typed projection, alongside 145 supported bodies. The builder still evaluates the name expression once, preserves observable notification and captured conversion behavior, and uses the original untyped path for unsupported properties. On 78 rejected paths the allocation count fell from 78 to zero. A separate fixture including real two-way bindings fell by 82,304 executable bytes. [Typed binding metadata](../src/FerroUI.Base/metadata/typed_path.rs)

The largest remaining property definitions include `set_and_raise_core_with_base` (292,254 bytes across 260 physical ranges), `set_and_raise_core` (252,453 across 267), and `MetadataTable::unregister` (208,530 across 330). A next experiment can share priority and notification decisions before calling typed leaves for value comparison, coercion and storage. Reentrant style disposal, validation order and typed reads are part of the comparison. An application link would measure what fraction of those bodies can be shared.

[Measurements and complete ELF section breakdown](compiled-theme-performance.md) · [Raw machine-code families, frame ownership and startup samples](benchmarks/compiled-theme-property-reflection-linux-x86_64.json)

## Earlier linked instructions at `58b32389`

The columns below are physical bytes in the Linux x86-64 `.text` section of a full, unrooted WinUI consumer. Each column partitions instructions into mutually exclusive symbol categories. The raw file also lists the other framework implementations, standard-library functions, shared aliases and unlabelled text. Read-only tables, ELF relocations, unwind records and exception data have their own accounting in the executable report. [Executable sections and startup](compiled-theme-performance.md)

| Symbol family | Previous PR, ordinary link | Current implementation, ordinary link | Current implementation, safe folding and RELR |
|---|---:|---:|---:|
| Property registration, stores and observers | 7,990,849 | 7,323,458 | 7,227,223 |
| Bindings, reactive operators and events | 1,561,626 | 1,560,907 | 1,540,735 |
| Generated native XAML functions | 2,702,121 | 2,467,205 | 2,462,821 |
| Other XAML runtime, theme and codecs | 1,348,763 | 1,290,082 | 1,277,164 |
| Callable entries and trait shims | 1,905,679 | 1,827,173 | 1,300,064 |
| Compiler-generated destruction | 1,801,437 | 1,618,012 | 1,466,900 |
| Reference counting and weak handles | 784,680 | 645,587 | 626,287 |
| Reflection and value conversion | 1,340,539 | 1,310,782 | 998,815 |
| Collection primitives | 1,228,703 | 1,198,048 | 915,620 |
| Class projection, initialization and virtual dispatch | 638,549 | 636,432 | 631,795 |
| Thread-local and once initialization | 274,278 | 275,940 | 199,570 |
| Other closure bodies | 465,973 | 467,313 | 401,423 |
| **Entire .text, including other categories** | **28,530,117** | **27,089,685** | **25,852,453** |

The modern editable executable has 186,059 sized text symbol records describing 118,504 distinct address ranges. The script counts alias ranges once. When names from different categories share an address, their bytes go in a separate bucket. The classifier uses the emitted symbol's definition. Inlined instructions belong to the surviving enclosing symbol. A callback entry can contain real control behavior, and an `Rc` destructor can run arbitrary user code. These measurements rank investigation targets and do not predict how many bytes a rewrite would remove. [Physical ranges and leading definitions](benchmarks/ported-runtime-text-linux-x86_64.json)

## Where to change the port

### Dependency properties and typed bindings

On the previous editable build, `BindingEntry<T>::set_value` contributed 300,175 bytes in 267 distinct ranges. Two `EffectiveValue<T>` routines for setting and raising contributed another 551,342 bytes with 267 ranges per routine. `TypedBindingExpression::update_source` contributed 126,436 bytes in 113 ranges. Each concrete property type specializes logic for priority, subscriptions, notifying a parent and reentrancy along with the operations that actually depend on its value type. [Binding entries](../src/FerroUI.Base/property_store/binding_entry.rs) · [Effective values](../src/FerroUI.Base/property_store/effective_value.rs) · [Typed binding expressions](../src/FerroUI.Base/data/core/typed_binding_expression.rs)

This PR moved subscription lifetime, validation state, inherited notification traversal and common entry selection outside those type parameters. The remaining opportunity is to split a property write into a shared decision about precedence and publication, a small typed value operation, and a shared notification step. Keep the stored value, comparison, clone, coercer and metadata-cache hit typed. Measure linked bytes as the same fixture grows from a single property type to many distinct types. The bytes-per-type slope distinguishes a truly shared operation from a smaller copy for every type. Release mutable borrows before user callbacks or destructors run. A reentrant handler may replace the value again, so descendants must see the post-handler value at the same point as today.

The most promising concrete subset is enums and reference handles. Matching native symbols back to the declared XAML types found 117 non-nullable enums among 267 `EffectiveValue<T>` value types. Their bodies in `EffectiveValue<T>`, `BindingEntry<T>` and `LocalValueBindingObserver<T>` occupy 859,931 bytes. The same three families for 26 nullable class handles occupy 343,249 bytes. A compiler-generated enum adapter can map legal representations into a shared scalar operation, while preserving each enum's `TypeId`, underlying width, flags, nullable state and existing behavior for unknown values. A class-handle adapter can share lifetime operations while preserving the actual dynamic class, reference identity and nullability. Custom structs keep their own clone, drop and equality semantics. Record the total code envelope before prototyping and measure actual savings after the typed wrappers are linked.

A separate 96-type experiment checked a smaller change. Outlining cache clearing, replacing the cache on invalidation and skipping a redundant clear all preserved the checksum. They increased the stripped fixture by 4,032, 18,864 and 768 bytes respectively. The PR keeps the existing invalidation operation.

An initial erased metadata cache took the public overridden-default read from 10.02 to 15.79 ns in a paired probe. The shipping version keeps a typed `Rc<M>` cache and shares the hierarchy search. It allocates an override list only when a second host supplies metadata. The ordinary styled-property table fell from 88 to 40 bytes inline, with one heap allocation of 48 bytes instead of two totaling 64 bytes in the measured fixture. The final balanced run still found small steady-state costs, including a 6.2% increase in the mixed read/write workload. The exact workload and measurements are in the executable report. [Property metadata implementation](../src/FerroUI.Base/ferro_property_metadata.rs)

### XAML templates and virtual initialization

The full native Rust build contains 2,439,558 bytes of direct generated XAML functions. In the measured native generator output, 517 deferred factories contributed about 2.14 MB. The NavigationView factory alone contributed about 58.6 KB. Its expansion contains 98 priority-setting operations and 46 name registrations. This code is retained for later template activation, even when first startup resolves only Button. Changes in this PR already share typed pushes on the XAML parent stack and the pop-and-EndInit sequence. [Object initialization lowering](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/emitter.rs) · [Compiled runtime operations](../src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/compiled.rs)

The next candidate is a template-local instruction stream for repeated construction, assignment, registration and init operations. Emit stable local operand indices and direct typed function imports for the operations used by that template. Keep custom constructors, converters and overrides callable through those imports. Preserve the exact ordering of BeginInit, parent pushes, named-object registration, EndInit, template binding and disposal. Preserve source positions at fallible instructions, unique namescopes for standalone builds, provider identity for nested builds and old-generation data for previously created deferred factories. Compare actual template activation for NavigationView, ScrollViewer and Button in addition to measuring the first theme lookup.

### Reflection, delegates and nullable conversion

The previous editable image included 444 ranges for `from_markup_value` totalling 248,855 bytes, 961 nullable-registration ranges totalling 158,565 bytes and 429 object-registration ranges totalling 96,518 bytes. Two generated markup callback families accounted for 807,560 bytes across 7,238 ranges in the callable category. Many of those entries execute the getter, setter or method as well as adapting its ABI. [Value-type registry](../src/FerroUI.Base/data/core/value_type.rs) · [Markup member macros](../src/FerroUI.Base/metadata/markup_macros.rs) · [Member invocation](../src/FerroUI.Base/metadata/markup_type.rs)

An isolated 42-type nullable-registration prototype kept the same value ABI, six registry map shapes and arbitrary captured user conversions. Static built-ins used a common function-pointer adapter around a small typed leaf. The ordinary stripped fixture fell from 1,637,296 to 1,625,184 bytes. With safe folding and RELR it fell from 1,599,240 to 1,595,240 bytes. Boxed, borrowed, wrong-type, null and unwrap cases passed for all 42 types. A stateful converter observed its changed capture. This is evidence for a small adapter saving. Whole-app throughput and size were outside that experiment.

The PR reuses `ValueType::of::<T>` directly in descriptors and shares cast, object normalization and error construction. A further change could encode arity, conversion policy, null treatment and result wrapping in compact member descriptors. Generate one typed thunk for the actual Rust member. Static built-in conversions can use function pointers with small immutable operands, while dynamically registered conversions keep owning their captured Rust closures. Validate overload selection, boxed null versus nullable handles, object identity, user-defined conversions, inheritance, exception type, diagnostics and line/column values. Keep the descriptor and its referenced operations in the owning member or resource section so registering one class does not retain the entire catalog.

### Reference ownership, destruction and class dispatch

The previous editable ELF had 241,417 relative relocations. Of these, 143,549 entries in `.data.rel.ro` and 70,316 GOT entries pointed into `.text`. Taking function addresses therefore has a substantial file-format cost under ordinary RELA encoding. These pointer counts include multiple mechanisms and do not identify a reflection table or vtable by themselves. RELR represents the same fixups compactly, while member-local tables can constrain which code becomes reachable at all.

Destruction, reference counting, closure entry points and class projection interact. Reducing one provider closure per property can remove allocation, weak-handle work, a `Drop` body, a vtable entry and unwind records. The source change to borrow observer identity on completion is one such example. Keep weak ownership edges in subscriptions, release order, synchronous completion and destructor reentry observable. Sharing state machines across `T` is a lower-risk first target than unifying distinct `Rc<T>` layouts. [Local binding observers](../src/FerroUI.Base/property_store/local_value_binding_observer.rs)

For virtual calls, the framework already reuses parent slots when it knows that a method is inherited unchanged. Further sharing requires equality of the full override mask and ABI-compatible table layout. Handwritten implementations without a declared mask still need their own dispatch path. Preserve `TYPE` identity, subclass construction and explicit base calls. On this workload, the direct class projection family is much smaller than the property and template families. [Class macros and vtable construction](../src/FerroUI.Base/type_system.rs) · [Rust trait-object dispatch](https://doc.rust-lang.org/reference/types/trait-object.html)

## Reproducing the attribution

Keep a symbolized copy from the same optimized link. The tool compares the address, length and SHA-256 of its code with the actual stripped product when `--match` is supplied. GNU `nm` and `readelf` are required.

```sh
python3 scripts/themes/bench/attribute_runtime.py ./winui-symbolized \
  --match ./winui-stripped --output ./runtime-text.json
```

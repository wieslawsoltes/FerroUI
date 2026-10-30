# Explicit Int32 enum factories for compiled XAML

Four existing isolated 0.3.0 packages now carry the same optional compiler metadata:
`enum_i32_factory` names an associated `fn(i32) -> original enum value` on the public
metadata carrier. Base stores it only with compiler metadata; Build.Scan preserves it in
format-2 exports, absent for older/closed declarations. Build's model host and the loader's
runtime host expose it to the same Rust emitter. Known members keep their existing constant
paths. Unnamed plain-enum constants use only the explicit typed factory; no constructor,
method name or flags status is inferred. Missing factories remain unsupported.

The original XamlX frontend narrows oversized compiled enum literals to Int32; both original
compiled C# XAML and the native pipeline retain that behavior. This is separate from JSON
numeric range validation. The final typed emitter checks its Int32 input and never truncates
an unnormalised AST value. Source-backed consumers cover signed endpoints, unnamed values,
nullable settings, original window options/parent identity and both narrowing boundaries.

The Core SDK declares factories over its actual open window enum types. This is not automatic
Rust source provenance or migration of the remaining plain enums. Factory declaration macro
syntax and non-Int32 enum storage are outside this change. Source/data/runtime engine paths
remain shared. Existing package licenses, original VCS identities and original file hashes
are retained; each complete patch is replayed against its exact cached published archive in
owned SSD scratch and compared byte-for-byte before updating provenance. Registry and sibling
FerroUI source are untouched; no package version or feature defaults change.

## Previous dependency changes

# FerroUI 0.3.0 conditional instance-check correction

`ferroui-markup-xaml/` is the unchanged published crates.io 0.3.0 package apart from
the declared instance-check and shared object-namescope changes. Registry bookkeeping markers
`.cargo-ok` and `.cargo-checksum.json` are excluded, as in the existing base copy.
The original license, notices, package manifest and VCS metadata are retained.
`ferroui-markup-xaml-provenance.json` records every original hash, the modified hashes,
and the exact `ferroui-markup-xaml-instance-check.patch` hash.

The original `DataTemplate.Match` equates registration of a conditional interface
cast with successful `IsInstanceOfType` for every object carried by its source handle.
For example, registering IDockable -> IDock is required for actual dock objects, but
the cast returns None for Tool. The original predicate still matches that Tool to
IDock and realizes the wrong inspector property view. The correction requires the
actual cast to succeed before accepting the assignability branch; normal metadata
fallback and native template building remain unchanged. This also applies to native
TreeDataTemplate through its shared predicate. No replacement matching engine is used.

`root_dock_debug_tests::complete_compiled_inspector_has_all_tabs_templates_tree_and_window_bindings`
checks the actual Selected tab chooses DockableDebugView for Tool, rejects DockDebugView,
and still realizes the real recursive root and floating-window trees. The full locked
workspace passes with this correction. The native desktop confirms correct Tool1
properties. Original custom/CLR metadata and lifetime scope remain explicitly partial.

Verify from the workspace root:

```sh
python3 scripts/dock/check_framework_patch.py
cargo test --locked --workspace
```

The shared namescope work additionally changes ResolveByNameExtension to query arbitrary
objects and the compiled runtime to register them. Its existing native-object helper
remains compatible. Named plain application objects now follow the same target-property
conversion and deferred registration path as native objects.

The separate published ferroui-markup-xaml-loader 0.3.0 package is retained with its
license, notices and registry VCS metadata. The declared files differ: the Rust emitter
registers Class and Exact object values via register_name_value, and the runtime AST
evaluator uses the same scope value contract (including null/unsupported error mapping).
The compiler's original AST registration transformer is unchanged: named plain objects
still declare INamed/Name. The emitter's root return/context/scope path now accepts
plain declared values and materializes constructor expressions exactly once, so each
assignment and the returned value reach the same instance. Group build records retain
the actual constructed handle. emit_types.rs and runtime_types.rs expose the declared
public value syntax for the model and runtime hosts; compiled.rs records those values
in group builds. Native root return types and styled-scope attachment remain unchanged.

The separate downstream NamedModelRoot is built, named, initialized, completed and
loaded through the ordinary compiled loader and original Load extension. The runtime
emitter independently produces the declared Rc return and one construction. A real
unchanged C# Document asset independently confirms plain-root loading/fresh identity.
Complete descriptor-carrier/public-type-path declarations, plain-model x:Class fields
and every generic/cross-package root variant remain unproven.

ferroui-markup-xaml-loader-provenance.json records all 141 original files and the five
modified hashes; ferroui-markup-xaml-loader-namescope.patch is the exact diff. The
framework integrity guard verifies all three packages. The base patch now also includes
the six shared namescope/binding/metadata changes described in NOTICE.md. The separate
compiled NamedModelConsumer fixture checks naming, Reference identity, and live compiled
binding updates from the real source Document model. No parser, registry or engine is
substituted. None of these packages depends on or modifies the sibling FerroUI checkout.

The base ValueTypes registry also carries explicit enclosing-object identity accessors.
Existing Rc<T> reference registration supplies the accessor automatically; UI-owned
metadata registers original model interface/class views without adding UI to Domain.
Scopes and the Dock reference converter use the same identity, preserving native object
projection precedence and distinguishing equal boxed scalars. Accessors run outside
registry/scope borrows and compare addresses without dereferencing them. Accessors belong to the existing type registry; there is no
per-object side registry, copied model, reflection or replacement equality engine.

## External constructor carriers and public value paths

The previous helper incorrectly substituted a metadata carrier's public path into the
constructor return type. CapabilityPolicyRoot/CapabilityOverridesRoot reproduced invalid
`DockCapabilityPolicyMetadataImpl` and trait-as-concrete output before the correction.
The model host now emits the import-resolved declared value syntax itself. The runtime
host first uses the EXISTING register_type_rust_paths/rust_path_of_type registry for
exact value types. Its fallback uses the actual Rust owner declaration, recorded by
ferro_markup_type only with compiler metadata, and replaces complete type components
only. A carrier prefix cannot rewrite a distinct model's identifier, including Unicode
identifier boundaries. Missing public value information is refused rather than guessed.

The base patch now declares twelve modified files, adding metadata/markup_type.rs and
metadata/markup_macros.rs for that compiler-only owner slot. It remains NotRecorded
without compiler metadata. The earlier resource, cast and namescope changes remain.
The compiler still declares five modified files; all exact hashes/diffs are updated.
UI-owned Dock model registration supplies public policy/override return paths through
the existing registry and adds identity accessors for their original interface views.
No public-path side registry or model replacement is introduced.

Four downstream cases exercise actual engine roots/scopes, ordinary Load/fresh objects,
real layout capability resolution/override precedence/live mutation, cross-interface
identity, both compiler hosts and rejection/explicit registration of a distinct return
type behind a prefix-named carrier. The synthetic CarrierValue is only a consumer fixture
holding original engine policy state; production policy roots are the unwrapped original
model instances. The unchanged C# harness loads both original classes and independently
asserts the same state and resolver outcomes. All concrete ReactiveUI model constructor
exports, logical assembly metadata, x:Class/names and arbitrary generic/cross-package/CLR
lifetime/boxing contracts remain further work.

# FerroUI XAML language service

`LanguageService` provides versioned incremental text synchronization, XML and schema diagnostics, completion, hover, markup-extension signatures, definitions/references, scope-aware rename, document/workspace symbols, folding, semantic tokens and source-preserving indentation. Native editor callers use UTF-8 byte offsets; `LineIndex` converts strictly between those bytes and LSP UTF-16 positions, including surrogate pairs and CRLF. Invalid drafts remain available without applying stale syntax spans to new text.

The built-in schema is generated from the existing `scripts/generate_view_builders.py` metadata reader: class inheritance, registered properties, attached properties, routed events, markup collection properties and enum values. Regenerate with `python3 src/FerroUI.Designer.Language/generate_schema.py`; `--check` validates it. `Schema::register` adds application controls, documentation and definition locations without linking their native assemblies into the server.

`index_solution` indexes unopened XAML without replacing open editor buffers. Resource navigation and rename cross indexed project documents; named references respect template namescopes. The shared `name_occurrences` grammar also supports duplication tools, including Binding ElementName/#name, x:Reference, selectors, relative panels and behavior/storyboard target attributes. Plain string properties and unresolved external references are not rewritten. Multi-document edits carry expected versions and apply atomically; stale refactorings cannot partially update the service.

Formatting changes inter-element whitespace while preserving attribute spelling, comments, mixed text content and inherited `xml:space="preserve"`. Runtime type loading and custom markup execution remain the host runtime's responsibility; schema diagnostics identify unknown registrations instead of executing application code.

## LSP process

Run `cargo run -p ferroui-designer-language --bin ferroui-xaml-lsp`. The process speaks Content-Length framed JSON-RPC on stdin/stdout and advertises UTF-16 incremental synchronization and the implemented semantic capabilities. It handles workspace discovery, open/change/save/close, refactoring edits, file-rename reference updates, custom schema configuration, shutdown and cancellation. The reader observes cancellation while semantic work runs; cancelled results are discarded. Protocol output is reserved for LSP and diagnostics go to stderr. Frames and document sizes are bounded.

The language layer never writes project files as a side effect of completion or rename. Clients apply returned versioned edits through their document/project transaction boundary.

## Semantic model and editor integration

The service caches declarations and resource edges per document, replacing those indexes as one step after each valid or invalid source update. It resolves nearest resource scopes, inline and imported merged dictionaries in reverse merge order, and application/standalone dictionary candidates. Import cycles terminate. Ambiguous references expose all candidate definitions; rename only rewrites references with one resolved declaration. Quoted resource keys and XML entity spellings retain accurate byte ranges. `name_scope` exposes the same template-scope rule used by `name_occurrences` and rename.

Multiple indexed solution roots coexist. Closing a project buffer restores its saved disk snapshot to the index; removing a workspace preserves editor buffers and snapshots still owned by another root. `resource_uri` understands file, relative, `ferres://` and `resm:` references through the project resolver. Client file rename requests combine edits for each affected document.

Binding member completion, hover and definition navigation use `x:DataType`, explicit Binding DataType, named element sources, self sources and template targets. Nested member paths follow registered property types. Hosts register view models with `Schema::register`; both `TypeInfo` and `PropertyInfo` accept optional `Location` values for code navigation. This metadata is data, so inspecting a project does not require executing its constructors or markup extensions.

```rust,ignore
let mut language = LanguageService::new(Schema::ferroui());
language.index_solution(&solution)?;
language.open(uri, source, 1)?;
let items = language.completions(uri, caret_byte)?;
let errors = language.diagnostics(uri)?;
let edits = language.rename(uri, caret_byte, "RenamedControl")?;
// Check each document version, then use ProjectWorkspace::apply_updates in the host.
language.apply_workspace_edit(&edits)?;
```

`LineIndex` supports LF, CRLF and CR without normalizing authored bytes. Surrogate splits, invalid UTF-8 offsets, stale versions, overlapping edits and mismatched LSP `rangeLength` values are errors. Formatting preserves mixed content and inherited `xml:space`; it changes layout whitespace without serializing the XML tree. The service's analysis snapshots do not retain a second undo stack alongside the designer's document history.

Byte/UTF-16 conversions use sparse Unicode checkpoints and binary searches instead of rescanning each line. Incremental changes splice the affected checkpoints and shift the unchanged suffix; ASCII text needs no Unicode entries. Refactoring batches validate all ranges and the final size before allocating one output buffer, then copy unchanged slices once. Run the repeatable long-line workload with `cargo run --release -p ferroui-designer-language --example source_engine_benchmark`; it reports indexing, coordinate queries and batch-edit timings for increasing document sizes.

File documents use canonical filesystem identity, including symlink and `localhost` aliases and not-yet-created files under an existing parent. Query/edit entry points accept the original URI spelling; returned file locations use canonical URIs. LSP file-rename notifications validate their complete mapping before changing the index, support swaps, and reject duplicate or conflicting destinations without partially moving documents.

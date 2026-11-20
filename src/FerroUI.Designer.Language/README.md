# FerroUI XAML language service

`LanguageService` provides versioned incremental text synchronization, XML and schema diagnostics, completion, hover, markup-extension signatures, definitions/references, scope-aware rename, document/workspace symbols, folding, semantic tokens and source-preserving indentation. Native editor callers use UTF-8 byte offsets; `LineIndex` converts strictly between those bytes and LSP UTF-16 positions, including surrogate pairs and CRLF. Invalid drafts remain available without applying stale syntax spans to new text.

The built-in schema is generated from the existing `scripts/generate_view_builders.py` metadata reader: class inheritance, registered properties, attached properties, routed events, markup collection properties and enum values. Regenerate with `python3 src/FerroUI.Designer.Language/generate_schema.py`; `--check` validates it. `Schema::register` adds application controls, documentation and definition locations without linking their native assemblies into the server.

`index_solution` indexes unopened XAML without replacing open editor buffers. Resource navigation and rename cross indexed project documents; named references respect template namescopes. The shared `name_occurrences` grammar also supports duplication tools, including Binding ElementName/#name, x:Reference, selectors, relative panels and behavior/storyboard target attributes. Plain string properties and unresolved external references are not rewritten. Multi-document edits carry expected versions and apply atomically; stale refactorings cannot partially update the service.

Formatting changes inter-element whitespace while preserving attribute spelling, comments, mixed text content and inherited `xml:space="preserve"`. Runtime type loading and custom markup execution remain the host runtime's responsibility; schema diagnostics identify unknown registrations instead of executing application code.

## LSP process

Run `cargo run -p ferroui-designer-language --bin ferroui-xaml-lsp`. The process speaks Content-Length framed JSON-RPC on stdin/stdout and advertises UTF-16 incremental synchronization and the implemented semantic capabilities. It handles workspace discovery, open/change/save/close, refactoring edits, file-rename reference updates, custom schema configuration, shutdown and cancellation. The reader observes cancellation while semantic work runs; cancelled results are discarded. Protocol output is reserved for LSP and diagnostics go to stderr. Frames and document sizes are bounded.

The language layer never writes project files as a side effect of completion or rename. Clients apply returned versioned edits through their document/project transaction boundary.

## Semantic model and editor integration

The service caches declarations and resource edges per document, replacing those indexes as one step after each valid or invalid source update. It resolves nearest resource scopes, inline and imported merged dictionaries in reverse merge order, and application or standalone dictionaries in the owning workspace. Import cycles terminate. Ambiguous references expose all candidate definitions. Rename checks the proposed spelling against the same dictionary graph, including unchanged references, and refuses changes that would select a different declaration. Quoted resource keys and XML entity spellings retain accurate byte ranges. Each disk snapshot has its own internal revision so a changed unopened file can invalidate a pending refactoring. `name_scope` exposes the template-scope rule used by `name_occurrences` and rename.

Multiple indexed solution roots coexist. Closing a project buffer restores its saved disk snapshot to the index; removing a workspace preserves editor buffers and snapshots still owned by another root. `resource_uri` understands file, relative, `ferres://` and `resm:` references through the project resolver. Client file rename requests combine edits for each affected document.

Binding member completion, hover and definition navigation use `x:DataType`, explicit Binding DataType, named element sources, self sources and template targets. Nested member paths follow registered property types. Quick fixes correct the casing of a typed binding member or a resource key when the service can resolve one replacement in scope. Hosts register view models with `Schema::register`. Both `TypeInfo` and `PropertyInfo` accept optional `Location` values for code navigation. The language service reads metadata without executing constructors or markup extensions.

Cargo builds publish an optional `xaml-rust-sources.json` companion to `.xamlmeta`. Its anchors identify authored registration macros by package, file, inline module and symbol. Definitions re-parse the current Rust file and return a range only when the same anchor resolves uniquely. Standard `textDocument/definition` uses the saved source. Clients with an open Rust draft can use the negotiated project-definition requests to supply that one buffer. Studio supplies its open Rust tab through the same language service.

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

## Project Library

The `ferrouiProjectLibrary` capability advertises `ferroui/projectLibrary` and `ferroui/resolveProjectLibrary`. Pass the indexed workspace `folder` URI, a `limit` between 1 and 200, and optional `query` and `offset`. The result contains `entries`, `total`, and `offset`. Each entry contains a workspace-relative `path`, its `folder`, and the definition’s kind, name, UTF-16 name range, and authored key or target when present.

Open the source buffer before resolving a result. Send its `uri` and synchronized `version` together with the entry’s fields. The language service checks the folder, current definition and buffer version. Clients can set `initializationOptions.ferrouiProjectLibrary` to receive `ferroui/projectLibraryChanged` when the underlying index changes. [Browse the Library in VS Code](../../docs/designer/vscode/authoring.md#find-reusable-xaml-definitions)

## Asset moves

The `ferrouiAssetMove` capability advertises `ferroui/planAssetMove`. Pass local `from` and `to` file URIs and a `sourceDrafts` array with the current Rust buffer versions and text. The service uses synchronized XAML buffers and indexed files to return the move, text patches, and a review identifier. Hosts replan before applying the checked edits and file rename in their editor transaction. [Move an asset in VS Code](../../docs/designer/vscode/authoring.md#move-an-asset-and-update-its-references)

## Project event handlers

`ferroui/projectMetadata` supplies the selected package’s build registry. The handler service resolves `x:Class` in that package, reads the event contract from the control’s exported type or its bases, and finds the authored `ferro_class_info!` registration through Cargo sources. The Rust and XAML snapshots use independent versions. The Rust grammar validates the original registration and reparses the proposed callback before returning the source changes.

Clients can discover the requests through `experimental.ferrouiEventHandlers`. `ferroui/eventHandlerTarget` returns the code-behind URI and signature. `ferroui/planEventHandler` accepts the current Rust text and version and returns a two-document `WorkspaceEdit` plus the new declaration range. `ferroui/eventHandlerDefinition` accepts the Rust buffer for navigation before save. A client that sets `initializationOptions.ferrouiHandlerCommands` gets a Quick Fix using its `ferroui.createEventHandler` command. The host checks both versions again before applying the patch atomically. [User guide](../../docs/designer/vscode/authoring.md#create-a-rust-handler-for-an-xaml-event)

## Typed command bindings

Discover `ferroui/commandBindings` and `ferroui/planCommandBinding` through `experimental.ferrouiCommandBindings`. Send `uri` and a `request` with the synchronized document `version` and UTF-16 `position` at the start of the authored control’s tag name. An optional `prefix` browses the registered members of a nested data-context property.

The planner also accepts `change` with the command `property` and tagged `command` and `parameter` choices. It verifies the writable command slot, the binding paths, and the current authored expression, then validates the proposed XAML before returning a versioned `WorkspaceEdit`. The calling host checks the frame and buffer again before applying the edit. Studio calls the same planner directly. [MCP source edits](../../docs/designer/vscode/reference.md#bind-a-command-with-mcp)

## Reviewed resource rename

The `ferrouiResourceRename` capability advertises `ferroui/resourceRenameTarget` and `ferroui/planResourceRename`. Send `textDocument.uri` and a `request` with the current `version` and UTF-16 `position` on the key, declaration tag, or reference operand. The target returns its name and declaration location. The plan also takes `newName`, refreshes the project index, and validates the same scope semantics as standard XAML rename.

The plan retains each file’s version, original text, exact changes, and proposed text. Qualified resource extensions, XML escapes, and object-form key operands have source spans. Hosts replan on Apply, compare the review identifier, and stage a single source transaction. Studio calls the planner directly. [User guide](../../docs/designer/vscode/authoring.md#rename-a-resource-across-xaml-files)

## Registered property rename

Clients use `experimental.ferrouiRegisteredPropertyRename` to discover two review requests. Send `textDocument.uri` and a `request` containing the current `version` and UTF-16 `position` to `ferroui/registeredPropertyTarget`. The result identifies the public name, selected range, and Rust source URI. Send the same fields, `newName`, and the relevant open Rust `sourceDrafts` to `ferroui/planRegisteredPropertyRename`. Each draft supplies `uri`, `version`, and `source`. The single `rust` draft argument remains supported for clients that supply only the registration source.

The service resolves the property through the consuming package's build schema, refreshes unopened XAML, and validates each patch against the proposed name. The review contains each affected document's original text, versioned patches, proposed text, and any potential XAML references with unmapped targets. Hosts recompute the plan and compare its review identifier before staging a single undoable transaction. Studio calls the same planner directly. [User guide](../../docs/designer/vscode/authoring.md#rename-a-registered-xaml-property)

Selector operands come from `ferroui-markup-syntax`, the grammar used by the XAML compiler and runtime. The language service carries the declared target through each selector branch and resolves nested targets and attached owners against the Cargo schema. It maps decoded XML token boundaries back to the source before planning patches. Go to Definition and the two registered rename reviews use these same operands. [Style selector workflow](../../docs/designer/vscode/authoring.md#rename-a-registered-xaml-type)

## Registered type rename

The `ferrouiRegisteredTypeRename` capability advertises `ferroui/registeredTypeTarget`, `ferroui/planRegisteredTypeRename`, and `ferroui/completeRegisteredTypeRename`. Use `textDocument.uri` and a versioned UTF-16 `request` to identify the build-registered type. The plan also takes `newName` and the relevant open Rust `sourceDrafts`. A literal alias returns a complete review.

Some type registrations inherit their public name from a Rust item. The build index records that item's physical module and source file. A plan with `rustRename` needs semantic Rust edits for that declaration. The host resolves the request through rust-analyzer and submits the `review` identifier and `resolution` to the completion request. The engine checks editable paths, versions, the Rust identifier tokens and the new XAML resolution before returning a complete review. [User guide](../../docs/designer/vscode/authoring.md#rename-a-registered-xaml-type)

An optional `assembly_source` in the Cargo source index identifies the selected `MarkupAssembly`. A review resolves explicit RSX entries through the same descriptor parser as the designer, then checks the current literal or local text constant in the indexed module. It merges metadata patches and semantic identifier edits per file. [Authoring the mapping](../../editors/vscode/README.md#map-an-application-control-to-rsx)

## Resource dictionary extraction

Clients that set `initializationOptions.ferrouiExtractionCommands` receive the `refactor.extract` action for an authored dictionary. `ferroui/dictionaryExtractionTarget` resolves the caret and version to an inline scope. `ferroui/planDictionaryExtraction` adds a sibling `fileName` and returns the replacement, the proposed file, and a review identifier. The host asks for a fresh plan and matches the identifier before committing the edits. Studio uses the same service directly. [User guide](../../docs/designer/vscode/authoring.md#extract-a-resource-dictionary)

## File moves

Clients can discover the reviewed move planner through `experimental.ferrouiFileMove`. Call `ferroui/planFileMove` with `from` and `to` file URIs. With `experimental.ferrouiFileMoveSaved`, pass open Rust, stage-set, and scenario drafts as `sourceDrafts` entries with `uri`, `version`, and `source`. The server still accepts the earlier `rustDrafts` argument. The planner refreshes project references and returns the canonical paths, a review identifier, and versioned edits and before and after source for each affected file. The host replans immediately before applying. Studio uses `LanguageService::plan_file_move_with_drafts` and its native transaction history. [User guide](../../docs/designer/vscode/authoring.md#move-an-xaml-file-and-its-references)

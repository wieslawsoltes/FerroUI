# ProTranslate Studio engine

This crate ports the portable catalog editing and translation workflow from the
ProTranslate Translation Studio sample. It contains no windowing, clipboard,
file-picker, process-global configuration, or implicit network access.

## Contract

**Inputs:** text in any `ferroui-protranslate-formats` format; a normalized catalog;
explicit source/target cultures, session identity and timestamp; edits identified
by stable entry IDs; host-provided storage paths and an optional asynchronous AI
transport. Provider credentials and model names are supplied by the host.

**Outputs:** normalized catalogs and format diagnostics; filtered entries, review
counts and measured coverage; bounded undo/redo; versioned JSON session/settings
snapshots; recovery and rotating backup records; AI suggestions with their request
baseline, model and token usage; placeholder diagnostics and format-helper output.

### Editing and catalog fidelity

The full normalized catalog, including document metadata, context, plurals,
comments, source text, format, original state and per-entry metadata, survives
editing, undo/redo and session serialization. Review state is separate from the
original file state. Entry IDs distinguish duplicate keys and cultures. Import
errors leave an existing document intact; export reports representation losses.
Text edits, notes, review changes, additions, deletion and batch acceptance are
undoable. The default history limit is 100 operations. Loading a different session
clears history and pending suggestions. Coverage is calculated from actual data.

`StudioDocument::export()` preserves original format states along with all other
catalog metadata. To export Studio review decisions, pass
`document.catalog_with_review_states()` to `CatalogGateway::export()` or
`export_to_storage()`. This explicitly projects generic `Missing`, `Review` and
`Approved` states without modifying the source catalog. Destinations may report
loss diagnostics for that projection; for example, PO represents `fuzzy` rather
than arbitrary review states. Unknown native states are never silently replaced.

### Persistence

Storage and timestamps are explicit. The native filesystem implementation writes
a sibling temporary file, flushes it, then atomically renames it; failures preserve
the previous destination. A successful session save marks only the saved revision
clean; recovery snapshots and backups never clear dirty state. Corrupt recovery
and settings are reported so a host can offer recovery or defaults. Backup names
include a collision-resistant sequence, with the session identity in the record;
creation uses an atomic create-if-absent operation. Rotation occurs only
after the new backup has been saved. A host-driven autosave scheduler replaces the
sample's hidden background timer, so there are no racing UI snapshots or threads.
Storage namespaces are validated once and remain immutable. Equality and nesting
checks use conservative Unicode case mappings for case-insensitive filesystems;
path components ending in a dot or space are rejected to prevent Windows aliases.
Valid Unicode spelling and internal spaces are preserved.

### AI translation

OpenAI Chat Completions, Anthropic Messages and Gemini generateContent request and
response adapters use an injected async transport. There is no default HTTP client
and no network call during import, editing, or configuration. APIs include provider
selection, explicit connection tests, model discovery, single translation and
bounded batches (10 entries by default). Cancellation is cooperative. Responses
must map to requested keys; malformed, duplicate, unknown or omitted results are
errors. Suggestions retain a document/entry revision and are rejected when source,
target, culture, review or notes have changed since the request. Acceptance is an
undoable edit into Review; rejection does not modify the document. The host owns
credentials and may choose whether to persist them.

`AiProviderConfig::for_provider` starts with an empty `model_id`. Enabled credentials
allow `list_models` before a model is selected; translation and connection tests
require an explicit ID selected from live discovery or supplied by the host.
Discovery never selects a model or changes configuration. Empty successful lists
remain empty, and failed or incomplete pagination returns an error. Cancellation
discards partial discovery results and drops the pending transport future. The
explicit `compatibility_models` and `compatibility_model_list` APIs expose historical
upstream IDs for migration only; those IDs may be retired and are never an automatic
fallback. Hosts must choose a model compatible with the provider's translation API.

Sampling temperature is omitted by default. Set `omit_temperature = false` only
for models that accept it. OpenAI requests use `max_completion_tokens`; the explicit
`legacy_max_tokens` option supports older compatible servers. The Claude adapter
uses the supported `2023-06-01` API version and `x-api-key` authentication; Gemini
uses `v1beta` and the `x-goog-api-key` header. These are wire protocol choices, not
claims that any historical model is still available. The adapters were checked
against the official references; tests use fake transport and require no API keys:

- [OpenAI Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)
- [Claude Messages](https://platform.claude.com/docs/en/api/messages/create),
  [model discovery](https://platform.claude.com/docs/en/api/models/list),
  [API versions](https://platform.claude.com/docs/en/api/versioning), and
  [authentication](https://platform.claude.com/docs/en/manage-claude/authentication)
- [Gemini generateContent](https://ai.google.dev/api/generate-content),
  [model discovery](https://ai.google.dev/api/models), and
  [authentication](https://ai.google.dev/api)

### Formatting assistance

Helpers recognize composite, i18next, named/ICU and printf placeholders; report
missing/extra placeholders and malformed braces; build text, number, date, plural
and select snippets; and provide autocomplete and deterministic previews. Composite
previews use the real core formatter and explicit culture and date values. Other
previews are editor illustrations, not replacements for the target format runtime.

## Validation

Focused tests cover all-format gateway dispatch, metadata-preserving sessions,
filter/review/coverage, bounded undo/redo including add/delete and batch operations,
save failures, corrupt recovery/settings, atomic filesystem replacement, backup
rotation/collisions, autosave scheduling, all three AI wire protocols with fake
transport, cancellation, malformed HTTP/JSON/provider responses, stale suggestions,
placeholder validation and formatting helpers. The in-memory engine compiles for
`wasm32-unknown-unknown`; filesystem access is a native-only optional feature.

See `NOTICE.md` for attribution and the upstream baseline.

# Native component integration

The component is a detachable Cargo workspace under `components/FerroBoard`.
The SDK root explicitly excludes it from automatic workspace membership and
the catalog declares three normal path dependencies. Thus component versions, package license,
lockfile and tests remain independently owned. Parent applications can use it
without copying its sources into FerroUI.Controls.

main was `df8b92d9438a7f1f797f9f86c2c4c602e3a4bf75` at branch creation.
It predates PR #507. Five native prerequisites are separate commits, rather
than a merge of unrelated newer work: native Option metadata, borrowed
property access, TextBox selection operations, pointer-only focus policy and masked
input privacy. These are framework integration fixes, not runtime emulation.
Their commits can remain independently if the keyboard directory is removed.

The framework at that base exposes futures through its actual dispatcher scheduler
API and its IClock contract still uses the SDK TimeSpan. These existing framework
interfaces are used only in native adapters; Core exposes Rust Duration, futures,
traits, collections and ownership. No task, stream, delegate, reflection, collection
or garbage-collection runtime is introduced by this component. Host animation
starts from std::time::Duration and converts at the SDK boundary.

The catalog page is passive XAML with a minimal native class constructor. Its
settings are passed through the public keyboard/host constructors; gestures,
focus tracking, layout, rendering and target synchronization stay in the library.

Removal seams outside this directory: root Cargo.toml exclude entry;
ControlCatalog Cargo.toml, page/module/class tables, page-list entry, type registration
and page integration test; one CI step and its ICU system dependency; README/NOTICE
and this port's documentation entry. There are no keyboard hooks in other ports.

The imported source comes from the validated r3 artifact recorded in provenance.json.
Source C# and resource bytes remain unchanged. Retained migration/audit documents
state their own evidence scopes; current branch validation belongs in VALIDATION.md.

# FerroUI designer document engine

`ferroui-designer-core` is the portable editing boundary for the FerroUI visual designer. It has no dependency on controls, rendering, the dispatcher, an MCP transport or a filesystem. Visual tools, source editors and automation share the same transactions.

## Editing a document

```rust
use ferroui_designer_core::{Document, Edit};

let mut document = Document::new(
    r#"<Grid xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Button x:Name="confirm" Width="120" Content="Confirm"/>
</Grid>"#,
)?;
let button = document.ast().find_name("confirm").unwrap();
let revision = document.revision();
let change = document.apply_at(revision, "Resize confirmation button", vec![
    Edit::SetAttribute {
        node: button,
        name: "Width".into(),
        value: "160".into(),
    },
])?;
assert_eq!(change.revision, revision + 1);
assert_eq!(document.ast().find_name("confirm"), Some(button));
document.undo()?;
document.redo()?;
# Ok::<(), ferroui_designer_core::DesignerError>(())
```

Every mutation validates a private candidate before changing the accepted source, syntax, revision or history. `apply_at` rejects a stale revision. Attribute-only batches are coalesced and parsed once, including namespace declarations and multi-selection edits. Mixed structural commands run sequentially against the candidate and commit as one history item.

`InsertChild` and `MoveNode` indices count element children, excluding comments and whitespace. A move index denotes the destination list **after** removing the moving element. Moving an element into another namespace scope adds the declarations needed to preserve its resolved element and attribute names. Ancestor cycles and removal/movement of the document root are rejected.

## Source synchronization

`source()` returns exactly what the user authored. `valid_source()` and `ast()` expose the last successfully parsed snapshot. `update_source` deliberately accepts malformed text so typing an incomplete tag never discards the user's buffer or the last good preview. It returns diagnostics and `valid: false`; visual commands are rejected until source is repaired. The runtime loader remains responsible for type, property, binding and resource resolution.

All source spans use half-open **UTF-8 byte ranges**, not UTF-16 editor offsets. A browser or C# editor must translate its indexing convention at the integration boundary. `apply_text_patches` accepts non-overlapping patches in the current source coordinate system and validates only the completed candidate. It rejects overlap, ambiguous insertions, ranges outside the document and offsets inside a multibyte character.

A `ChangeSet` contains the preceding/new revision, inserted/removed/changed syntax IDs and a minimal contiguous source replacement. Applying that replacement to the previous buffer reconstructs the accepted buffer. Runtime preview acceptance is separate from syntactic validity, allowing semantic errors to remain editable.

## Syntax and identity

The AST indexes elements, property elements, text, comments, CDATA and processing instructions. Attributes retain their qualified name, resolved namespace, decoded value, full/name/value spans and quote character. Namespace declarations are normal syntax attributes. `Attribute::markup` parses nested markup extensions into positional/named arguments, including nested extensions, quotes, escapes and the XAML `{}` literal escape. Unknown extension types are preserved and resolved by the runtime loader.

No serialization pass rewrites the document: unchanged markup retains byte-for-byte formatting, comments, entities, attribute order and quote style. Attribute edits replace only their value spans; new properties and elements splice into the source.

Node IDs are document-local monotonically allocated integers. Visual edits pin surviving syntax locations; explicit moves pin the moved subtree, including identical unnamed siblings. Source replacement reconciles unique XAML names, unchanged subtree fingerprints and sibling kind/order under reconciled parents. Namespace-prefix spelling does not determine semantic identity. IDs survive undo and redo and are never recycled. When source replaces several indistinguishable unnamed siblings, identity reconciliation is necessarily positional; collaboration transports should use their own persistent replica identifiers and map them to the document.

Namespaces are resolved during a single token traversal. Reconciliation hashes each syntax token bottom-up and uses indexed sibling buckets rather than comparing every pair of sibling subtrees. Multi-property transactions validate once. History snapshots share immutable source and AST storage with `Arc`; history is bounded to 256 entries by default and can be configured with `set_history_limit`.

## Validation

Run:

```sh
cargo test -p ferroui-designer-core
```

Tests cover exact-source retention, Unicode/entity escaping, comments and CDATA, namespace scope and prefix changes, atomic rollback, stale revisions, invalid editor buffers, undo/redo, non-reused IDs, duplicate sibling moves, namespace-preserving reparenting, source patch boundaries, nested markup extensions, DTD rejection and a 10,000-element edit/reconciliation scenario. XML parsing is bounded to one million syntax nodes and DTD processing is disabled.

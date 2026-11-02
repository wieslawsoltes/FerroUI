# Portable binary XAML documents

The optional `portable-xaml` loader feature reads and writes FXD v1, a binary encoding of the six node kinds that XamlX produces when it parses XML. A document can move between processes, machines and target architectures. Custom types, attached properties, markup extensions and member resolution use the receiving application's XAML type system. Compiler-generated FXB leaves and their native typed builders remain available for linked, ahead-of-time themes. [FXB format](fxb.md)

## Build and load

Compile a source document with the framework parser's namespace compatibility settings.

```sh
cargo run --release -p ferroui-markup-xaml-loader --no-default-features \
  --features portable-xaml --bin ferroui-fxd -- Views/Editor.xaml Editor.fxd
```

Applications with `runtime` and `portable-xaml` can pass the resulting bytes to `FerroRuntimeXamlLoader::load_stream` or to their normal runtime document group. Preserve the original base URI for relative resource and style includes. A document group's inputs can mix XML and FXD. Each FXD stream reconstructs a fresh mutable syntax graph before the existing root substitution, transforms and interpreter run. The type system, source diagnostics, namescope and deferred content semantics come from that pipeline.

`binary_document::compile` parses a string and encodes it. For an existing parsed `XamlDocument`, `encode` chooses compression when beneficial and `encode_stored` writes an uncompressed image. `decode` reconstructs the graph without executing constructors. Recompiling a document after a source edit produces a new image whose bytes identify it in the runtime group's cache.

## Replacing a mounted style

Enable `ferroui-hot-reload/portable-xaml` in a development host. Register the style with `StyleReloadTarget::contribution`. On the UI thread, pass a complete edited image to `target.replace_portable(bytes)`. The loader builds the replacement, resolves its relative includes and verifies that its root implements `IStyle` before changing the owned slot. If a revision fails, the previously installed style stays active. Other styles in the same container retain their order and identity. The compiler CLI publishes its output with a rename so a file-change handler can read a complete revision.

## FXD v1 layout

Integers use canonical unsigned 32-bit LEB128 except for the four-byte size of a compressed section. A node reference counts backwards from the previous node, starting at zero. Every referenced node has already been decoded. Optional string and scope references reserve zero for absence and otherwise add one to the index.

| Section | Encoding |
| --- | --- |
| Envelope | ASCII `FXD` and a zero byte, major version 1, storage mode |
| Stored syntax, mode 0 | The syntax section follows immediately |
| Compressed syntax, mode 1 | Expanded byte count as little-endian u32, then one RFC 1950 zlib stream. End-of-stream, consumed byte count, expanded size and checksum must agree |
| String table | Count, then length and UTF-8 bytes for each string |
| Document metadata | Optional document-name index and the root namespace alias map |
| Lexical scope table | Count, followed by prefix/URI maps of visible bindings. A map contains a count and string-index pairs sorted by prefix at emission. Every node sharing a lexical scope uses one table entry |
| Graph | Node count, followed by postorder records. The last node is the root object |

Each record starts with an opcode byte, source line, source column and optional scope reference. Lists carry counts. Flags are single bytes of value zero or one.

| Opcode | Payload in order |
| --- | --- |
| 0 XML type | Optional namespace string, local-name string, markup-extension flag, generic type references |
| 1 Named property reference | Declaring type, name string, target type |
| 2 Text | Text string, preserve-whitespace flag, type reference |
| 3 Object | Type reference, positional argument nodes, ordered child nodes |
| 4 Property assignment | Property reference, attribute-syntax flag, ordered value nodes |
| 5 XML directive | Optional namespace string, directive-name string, ordered value nodes |

The same type node can appear as an object's type and the declaring and target type of a property. The reader recreates that shared identity. Non-type nodes belong to one parent. A stored or expanded document is limited to 32 MiB, with at most 524,288 nodes, 768 levels of node references, 4,194,304 recursive visits and 64 MiB of string materialization. The reader rejects forward or out-of-range references, nodes in the wrong role, unreachable nodes, oversized expansion and trailing bytes before the XAML runtime receives a document.

## Reproduce the syntax benchmark

```sh
cargo run --release -p ferroui-markup-xaml-loader --no-default-features \
  --features portable-xaml --example portable_document -- \
  src/FerroUI.Themes.WinUI/Controls/Button.xaml \
  src/FerroUI.Themes.WinUI/Resources/Button_themeresources.xaml \
  src/FerroUI.Themes.WinUI/WinUITheme.xaml
```

The example measures 51 in-memory parse-and-drop or decode-and-drop iterations per path. Its median measures syntax reconstruction. Application type resolution, transforms, object construction, asset I/O and process startup are separate costs.

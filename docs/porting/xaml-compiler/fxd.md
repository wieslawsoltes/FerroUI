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

The example measures 51 interleaved in-memory parse-and-drop or decode-and-drop iterations per path, rotating the order on every pass. Its median measures syntax reconstruction. Application type resolution, transforms, object construction, asset I/O and process startup are separate costs.

On Linux x86-64 with Rust 1.99.1 in release mode, the 690 XAML files across the five built-in themes occupied 6,986,943 bytes. Their independently encoded FXD images occupied 3,317,018 bytes, a 52.5% reduction. Every file decoded and re-encoded to the same bytes. The following warm-memory measurements use one pinned CPU and the rotating sample order above.

| WinUI document | XML bytes | Stored FXD bytes | Compressed FXD bytes | Parse XML p50 | Decode stored p50 | Decode compressed p50 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Controls/Button.xaml | 16,102 | 18,937 | 7,990 | 747 µs | 454 µs | 587 µs |
| Resources/Button_themeresources.xaml | 15,733 | 16,570 | 6,368 | 356 µs | 276 µs | 346 µs |
| WinUITheme.xaml | 7,672 | 8,480 | 3,040 | 135 µs | 103 µs | 127 µs |

## Indexed theme archives

FXP v1 packages an inventory of original relative XAML asset paths and independently decodable FXD images. A member retains its source coordinates, namespace scopes and the full syntax graph. A development host can replace an individual member through the existing portable style reload API. For a complete change, `StyleReloadTarget::replace_packed(&revision)` validates the archive, resolves the registered style and its includes from the candidate revision, and replaces the owned style slot after construction succeeds. A corrupt unrelated member also rejects the revision. Custom hosts can call `ThemePack::open` and `ThemePack::validate` to validate all graphs without executing application code.

```sh
cargo run --release -p ferroui-markup-xaml-loader --no-default-features \
  --features portable-xaml --bin ferroui-fxd -- \
  --pack src/FerroUI.Themes.WinUI WinUI.fxp
```

`binary_document::pack::compile_documents` accepts a build host's selected inventory, including the result of an explicit resource-roots pass. The directory CLI includes every regular `.xaml` file recursively. Paths use UTF-8 and slash separators. An FXP reader borrows the set of member images. Its sorted index supports binary search by path. Looking up a member does not decompress its neighbors.

| FXP v1 field | Representation |
| --- | --- |
| Envelope | Four bytes `FXP\0`, version byte 1, little-endian u32 document count, little-endian u32 index byte length |
| Index | In strictly increasing UTF-8 path order, repeated u16-LE path length, path bytes, u32-LE stored image length |
| Images | Concatenated independent complete FXD images in index order |

The reader checks all index and image boundaries up front, including every member's size declaration. Counts are bounded to 16,384 members, 4,096 bytes per path, 256 MiB total stored or expanded archive size and the normal 32 MiB limit per FXD. Duplicate names, traversal components, leading slashes, invalid envelopes, extra index bytes and trailing data fail validation.

### Columnar FXD storage

Layout 2 stores five streams for the node graph. Layout 3 wraps that same syntax payload in the version 1 zlib envelope. Both preserve the metadata, string table, lexical scope table and postorder node numbering of layouts 0 and 1. After the node count, three canonical unsigned LEB128 lengths give the respective byte sizes of the source line, column and scope streams.

| Stream | Count and contents |
| --- | --- |
| Tags | One opcode byte per node |
| Source lines | One ZigZag-encoded i32 delta from the previous line per node. The initial line is zero |
| Source columns | One unsigned column per node |
| Scopes | One optional scope reference per node |
| Operands | The original per-opcode payload of each node in order |

The decoder reads the streams together directly into the syntax graph. Syntax validators enforce the same node roles, resource limits and shared-reference rules. The reader rejects surplus bytes in any stream and source-line overflow. Archives use these layouts to keep opcodes, positions and repeated operand sequences individually compressible. Each member makes its own stored-versus-zlib decision. Older readers continue to load the standalone compiler's layout 0/1 output and reject unfamiliar layout codes.

### Full WinUI corpus

```sh
cargo run --release -p ferroui-markup-xaml-loader --no-default-features \
  --features portable-xaml --example portable_document -- \
  --pack src/FerroUI.Themes.WinUI Controls/Button.xaml
```

The unfiltered WinUI source tree contains 169 documents totaling 2,092,775 bytes. Standalone FXD output totals 900,063 bytes. The complete indexed archive is **432,438 bytes**, including the directory. Decoding and serializing each member as canonical stored FXD reproduced the XML parser's graph byte for byte on the full inventory.

| In-memory operation, Linux x86-64 | Standalone FXD p50 / p95 | FXP p50 / p95 |
| --- | ---: | ---: |
| Open index | — | 82 / 139 µs |
| Decode and drop Button syntax | 431 / 587 µs | 383 / 551 µs |
| Decode and drop all 169 graphs | 55.074 / 66.955 ms | 52.147 / 65.618 ms |

A pinned CPU ran 51 iterations in rotating order using release-optimized Rust 1.99.1. Archive compilation, including source reads from a warm filesystem cache, took a median 214.322 ms over nine builds with identical output. These timings end at syntax reconstruction. Type resolution, framework construction, process startup and first usable UI are separate measurements. The archive contains portable XAML data. An executable also links the decoder, runtime and its selected framework types.

### Solid FXP distribution

The version 2 envelope stores the complete document group in one zlib stream. The WinUI crate selects it with the `solid-theme` feature. The CLI writes the same format with `ferroui-fxd --pack-solid <source-directory> <output.fxp>`. Use `--pack` or the `packed-theme` feature for independent per-document compression. Both encodings retain the original document paths, ordered child lists, generic arguments, markup extension flags, lexical scopes and line information.

| Field | Representation |
| --- | --- |
| Envelope | `FXP\0`, version byte 2, little-endian u32 inflated length, zlib stream |
| Global UTF-8 string pool | Lexical byte order, encoded as shared prefix length and length-prefixed suffix |
| Sorted directory | Path, reconstructed FXD byte size, local string count and lengths of the five per-document columns |
| Local string tables | ZigZag deltas between global string IDs, reset at each document |
| Graph columns | Per-document metadata, node tags, source line deltas, source columns and namespace scopes |
| Operands | Six streams, one per FXD opcode. The tag sequence restores the original instruction order |

Counts and section lengths inside the stream use unsigned LEB128. The decoder bounds the declared, inflated and reconstructed sizes, validates references and consumes the entire compressed stream. Expansion produces an indexed version 1 archive containing stored columnar FXD documents. The WinUI adapter retains this expanded image once per process. Reloading a mounted contribution validates every candidate document and resolves its relative includes against the candidate before publication.

On the complete 169-document WinUI input, `--pack-solid` produces 245,385 bytes and `--pack` produces 432,438 bytes. Both outputs preserve the same canonical XamlX graphs. These are serialized theme sizes. The application benchmark in [compiled-theme-performance.md](../../compiled-theme-performance.md) accounts separately for native code, metadata, unwind tables and the runtime loader.

The byte-level specification defines the directory record, all twelve data streams, how to reconstruct each opcode row, and the expansion limits. [FXP v2 wire layout](fxp-solid.md)

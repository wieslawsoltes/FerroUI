# Encoded file readers and document streaming

This slice ports the encoding-selection responsibility of
[`Utils/FileReader.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/FileReader.cs)
at AvaloniaEdit revision `7277e36366d754219d7ede16b6613786b32caf4c`.
It adds usable byte-stream decoding and file helpers to `ferroui-edit-core`,
building on the existing [UTF-16 document readers and writers](editor-document.md).
It does not claim the complete .NET encoding catalog, file-saving APIs or native
TextEditor load/save integration.

## Source and member mapping

Paths in the destination column are relative to `src/FerroUI.Edit.Core`.

| Pinned source member or delegated responsibility | Rust destination | Behavior and adaptation |
|---|---|---|
| `FileReader.IsUnicode` | `utils/text_encoding.rs`: `TextEncoding::is_unicode` | Typed encoding classification; includes UTF-32 as the source comment intends, correcting its active type switch that only recognizes UTF-8 and UTF-16 |
| `FileReader.IsAsciiCompatible` | `TextEncoding::is_ascii_compatible` | Exact classification for the eight supported encodings; no need to allocate and encode the probe string `Az` |
| `FileReader.RemoveBom`, `UTF8NoBOM` | `FileTextReader::encoding`, `byte_order_mark_length`, `has_byte_order_mark` | Reports encoding and the actually consumed BOM separately; does not expose a managed Encoding object's output-preamble setting |
| `FileReader.OpenStream` | `utils/file_reader.rs`: `FileReader::open_stream`, `open_stream_with_options`, `open_seekable_stream` | BOM recognition and bounded detection; generic nonseekable streams replay the retained prefix, while the seekable entry point enforces position zero |
| `FileReader.AutoDetect` | `utils/file_reader.rs`: `detect`, `Utf8Probe` | Original 500,000-byte structural UTF-8 heuristic, ASCII-compatible default selection, legacy fallback and Unicode-default replacement |
| `FileReader.ReadFileContent(Stream, Encoding)` | `FileReader::read_stream_content` | Decodes the stream and materializes its remaining text as a Rust `String` |
| `FileReader.OpenFile`, `ReadFileContent(string, Encoding)` | `FileReader::open_file`, `open_file_with_options`, `read_file_content` | Real `std::fs::File` operations and ordinary IO errors; both path overloads throw `NotImplementedException` in the pinned source, so this is restored functionality |
| `StreamReader` construction, `CurrentEncoding`, `Read`, `Peek`, `ReadLine`, `ReadToEnd` used by `FileReader` | `utils/file_text_reader.rs`: `FileTextReader<R>` | Incremental decoding, UTF-16 units, peek, line reads, complete reads and metadata; standard Rust ownership replaces managed disposal |
| Byte-stream consumers and existing editor text writers | `FileTextReader::into_utf8`, `Utf8FileTextReader<R>`, `FileTextReader::copy_to` | Explicit `Read`/`BufRead` UTF-8 projection and streaming writes to the existing `Utf16TextWriter` contract |

The native framework already exposes file bytes through
[`IStorageFile::open_read_async`](../../src/FerroUI.Base/platform/storage/i_storage_file.rs)
as `Box<dyn std::io::Read>`. That interface does not require seeking. The Core
reader therefore accepts it directly, without introducing a second storage
service or adding a dependency on FerroUI's native control crates. Local path
helpers use the standard library; applications using native pickers or browser
storage should pass their existing storage stream to `open_stream`.

## Encoding selection

`TextEncoding` supports UTF-8, UTF-16 LE/BE, UTF-32 LE/BE, ASCII, ISO-8859-1
(Latin-1) and Windows-1252. `FileReaderOptions::default()` chooses UTF-8 with
replacement decoding. The overload accepting a default encoding changes only
the fallback; a recognized BOM or the UTF-8 heuristic can override it.

Detection follows these steps:

1. Read up to four initial bytes, tolerating short reads and interrupted IO.
   Recognize the UTF-32 marks before UTF-16 because the little-endian marks
   share their first two bytes. Recognize UTF-8 and both UTF-16 marks, consume
   exactly one leading mark and retain any remaining bytes for decoding.
2. Empty and one-byte inputs retain the supplied default encoding, matching the
   source's short-file branch.
3. If the first two bytes are `00 00`, `FF FE`, `FE FF` or `EF BB`, use the
   reference's StreamReader/BOM path. Without a complete recognized mark this
   path still defaults to UTF-8. For example, `EF BB 78` decodes to `�x`.
4. Otherwise inspect at most 500,000 input bytes. Pure ASCII keeps the supplied
   default if that encoding is ASCII-compatible; a UTF-16/32 default instead
   becomes UTF-8. A structurally plausible multibyte sequence selects UTF-8.
   A structural error selects a non-Unicode default, or UTF-8 when the default
   is Unicode.

The detector is deliberately the source's heuristic, not a claim that bytes
uniquely identify an encoding. It checks continuation structure but does not
enforce every Unicode scalar constraint. It also selects UTF-8 if the probe
ends inside a plausible sequence, whether that occurs at the detection limit
or actual EOF. Thus `63 61 66 E9 20` falls back to Latin-1 when requested, but
`63 61 66 E9` selects UTF-8 and replaces the truncated sequence. The decoder
validates the selected Unicode encoding independently of this heuristic.

The active source `IsUnicode` implementation excludes UTF-32 despite its
documentation. The Rust classifier includes both UTF-32 byte orders, so an
invalid unmarked probe with a UTF-32 default falls back to UTF-8. This is an
explicit correction. UTF-7 is not supported.

Applications that know the encoding, including unmarked UTF-16/32 files, can
bypass the heuristic:

```rust
use ferroui_edit_core::utils::{DecodingErrorPolicy, FileTextReader, TextEncoding};

let mut reader = FileTextReader::new(
    bytes,
    TextEncoding::Utf16Le,
    DecodingErrorPolicy::Strict,
);
let text = reader.read_to_end()?;
```

This constructor performs no input reads and does not strip a BOM. A leading
U+FEFF is ordinary content when an encoding is explicitly selected this way.
Auto-detected readers expose their final encoding and consumed BOM length
immediately after construction. An unmarked file always reports a BOM length
of zero; the reader does not infer a preamble merely from the encoding family.

## Decoding and error contracts

`FileTextReader` reads encoded bytes incrementally. It assembles UTF-8 sequences,
UTF-16 code units and surrogate pairs, and UTF-32 words across arbitrary source
read boundaries. Supplementary scalars produce two UTF-16 units for the editor's
document APIs. A caller may request one unit at a time or peek without advancing
the logical unit cursor.

Replacement decoding uses U+FFFD for malformed Unicode and `?` for non-ASCII
bytes under ASCII. UTF-8 replacement follows Rust's standard maximal-subpart
behavior, including overlong encodings, surrogate encodings, out-of-range
scalars and truncated sequences. Invalid UTF-16 surrogates and trailing odd
bytes are replaced, as are invalid UTF-32 scalar values or a truncated final
word. This is decoding an external encoding; it does not change the lossless
isolated-surrogate contract of `TextDocument`, `TextSnapshot` or `RopeTextReader`.

Latin-1 maps all byte values directly. Windows-1252 maps the C1 range according
to the [Encoding Standard's Windows-1252 index](https://encoding.spec.whatwg.org/index-windows-1252.txt),
including the retained control code points at bytes `81`, `8D`, `8F`, `90` and
`9D`. No operating-system locale or process code page influences these codecs.

`DecodingErrorPolicy::Strict` returns `io::ErrorKind::InvalidData` with the
encoding and encoded byte offset. Offsets are relative to the supplied logical
stream start and include any consumed BOM. A strict error consumes the offending
maximal subpart; a caller that deliberately recovers may continue after it.

The UTF-16 buffer reader and UTF-8 `Read` adapter preserve completed output when
an error follows it. They return that output's count, report the error on the
next nonempty read, and do not replay preceding characters. Empty destination
buffers neither advance the reader nor consume a pending error. An IO error
partway through a scalar retains its incomplete bytes for a later retry.
`Interrupted` reads are retried automatically; other IO errors retain their
ordinary kind and propagate to the caller. Errors during initial detection
prevent construction of a reader.

## Rust IO and document interoperability

`FileReader::open_stream` treats the stream's current position as its logical
beginning. It works with owned files, borrowed readers, in-memory data, short
reads and FerroUI storage streams. `open_seekable_stream` additionally checks
that the underlying position is zero before reading, retaining the upstream
precondition when callers need it. Passing `&mut reader` keeps ownership with
the caller; moving a stream into the file reader transfers its lifetime to it.

Detection retains at most 500,000 encoded prefix bytes and releases that replay
allocation once consumed. Subsequent decoding uses a 4 KB buffered reader and
at most four pending encoded bytes. `read_into`, `copy_to` and the UTF-8 adapter
do not allocate a complete decoded document. `read_to_end`,
`read_to_end_utf16`, `read_file_content` and line-read methods intentionally
materialize the text requested by their APIs.

`into_utf8` produces `Utf8FileTextReader<R>`, which implements both `Read` and
`BufRead`. It works with `std::io::copy`, `read_to_string`, buffered line APIs
and one-byte destination buffers. The conversion is explicit because editor
text-reader positions count UTF-16 units while standard Rust IO counts bytes.
If a caller has already consumed only the high surrogate of a supplementary
character, projecting the remaining low surrogate yields U+FFFD, consistent
with the existing rope-reader projection.

`read_line` and `read_line_utf16` consume CR, LF or CRLF, return the line without
its delimiter, and return `None` at EOF. They do not normalize line endings in
the underlying content or treat U+2028 as a CR/LF delimiter.

The existing `DocumentTextWriter` is a `Utf16TextWriter`, so a decoded file can
be inserted through the same document mutation and undo services already used
by other Core writers:

```rust
use ferroui_edit_core::document::DocumentTextWriter;
use ferroui_edit_core::utils::{FileReader, TextEncoding};

let bytes = storage_file.open_read_async().await?;
let mut reader = FileReader::open_stream(bytes, TextEncoding::Utf8)?;
let update = document.begin_update()?;
let mut writer = DocumentTextWriter::new(document.clone(), insertion_offset)?;
let written_utf16_units = reader.copy_to(&mut writer)?;
update.finish()?;
```

The caller owns the update group. Successful earlier sink writes remain present
if a later read or write fails; `copy_to` does not promise transactional rollback.
Decoding a complete result before replacing a document remains available when
all-or-nothing loading is required. These helpers do not attach editor controls,
assign file-name metadata, reset history or select save encodings implicitly.

## Verification and remaining scope

The focused regression suite covers:

- all five Unicode BOMs, ambiguous UTF-16/32 prefixes, BOM-only files and
  supplementary characters across one-byte source reads;
- ASCII/default selection, legacy fallbacks, UTF-8 probe quirks, partial marks,
  empty/one-byte inputs and the exact 500,000-byte detection boundary;
- a multibyte character spanning the retained prefix and live stream;
- malformed UTF-8 against the standard library's replacement result, invalid
  UTF-16 pairs/odd bytes and invalid/truncated UTF-32 words;
- strict error offsets, partial-output reporting, retry after an IO failure
  inside a scalar and repeated `BufRead::fill_buf` calls;
- UTF-16 peek/line operations, partial-surrogate projection, borrowed-stream
  ownership and seekable position validation;
- real local-file path helpers, standard IO errors and streaming through a
  `Box<dyn Read>` into `DocumentTextWriter` with one caller-owned undo group.

The focused run passed **14 tests**, with no failures or ignored tests. It
compiled the current 337-test Core suite and selected only the file-reader
regressions; the combined Core/native gate is tracked separately by the main
port report. Reproduce the focused run with:

```sh
cargo test --offline --no-default-features -p ferroui-edit-core --lib utils::file_reader
```

The finite codec enum does not represent arbitrary .NET `Encoding` subclasses,
all legacy code pages, UTF-7 or custom encoder/decoder fallback objects. There
is no file-writing codec in this slice, no background/async decoding task, and
no native TextEditor load/save wrapper. Local path helpers depend on the target
platform's `std::fs` support; browser/native storage streams use the generic
reader. The source heuristic's ambiguous-encoding choices remain explicit
rather than being presented as reliable charset inference.

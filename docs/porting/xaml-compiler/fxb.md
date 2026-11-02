# FXB v1 compiled resource blocks

The XAML compiler emits a typed Rust factory for each operation a theme needs. Four repeated deferred dictionary operations can use compact binary operands. They construct a static resource alias, a literal ARGB brush, a brush whose color comes from a static resource, or a brush bound to a dynamic resource. Templates, bindings, user types, ordinary property assignments and other operations continue through the typed Rust emitter. The binary path preserves deferred instantiation, source positions, insertion order and the XML namespace scope at the declaration site.

Enable the `binary-resources` feature of a built-in theme, for example `ferroui-themes-winui/binary-resources`. A custom crate enables `ferroui-markup-xaml/binary-resources` and calls `.binary_resources(true)` on its build script's `ferroui_build::XamlGroup`. Both model and runtime compiler hosts support this option.

Each contiguous run of eligible operations within one dictionary, lexical scope and factory becomes a separate `.fxb` file alongside its generated Rust module in `OUT_DIR/xaml`. Each compiled call includes only its own block and names its typed factory directly. A decoder in a client process needs no reflection, file I/O or second XML parse. The host-side encoder is feature-gated in the dependency-free `ferroui-xaml-binary` crate. Generated Rust keeps operations that cannot fit in a bounded block.

## Wire format

| Field in order | Representation |
| --- | --- |
| Signature and version | Four octets `46 58 42 00`, then major version `01` |
| Factory kind | One octet. Values 1, 2, 3 and 4 identify aliases, literal brushes, dynamic brushes and static resource brushes respectively |
| String count, record count | Two unsigned 32-bit canonical LEB128 integers |
| Strings, sorted by UTF-8 bytes | For each string, previous-string common-prefix length in bytes, suffix length in bytes, and suffix bytes. Prefixes end at UTF-8 scalar boundaries |
| Dictionary key | Index into the string table, zero-based |
| Value operand | Alias, static brush and dynamic brush use a string index. Literal brush uses a little-endian `u32` containing `0xAARRGGBB` |
| Source location | Declaration line and column, then ZigZag-encoded deltas for the value line and column relative to the declaration |
| Remaining records | Repeat key, operand and location in XAML insertion order |

All numeric fields other than the signature, version, kind and ARGB word use canonical unsigned LEB128. Absolute and reconstructed source positions fit a nonnegative `i32`. Blocks and expanded string pools are limited to 16 MiB each, with at most 1,048,576 records and 1,048,576 strings. Unknown versions and kinds, noncanonical or overflowing integers, invalid references, invalid UTF-8, invalid lengths and trailing bytes fail validation before registration begins.

`Batch::decode` expands the string pool in one pass, indexes records, and validates input before yielding any row. A deferred text operand holds a reference-counted slice of the pool, allowing the decoder and input bytes to be dropped immediately. Compression uses shared prefixes, interning, compact indices and source-position deltas; there is no decompression dependency in the executable.

## Hot reload

A reload client can decode and replace one run independently. Build the candidate using the same emitter to obtain the new bytes. Each block has a module-qualified ID, for example `my_app::compiled_xaml/compiled_xaml-0123456789abcdef.fxb`. The suffix is derived from the compiled function name, factory kind and first key, with deterministic disambiguation. Editing another run usually leaves the ID intact. Changing the set of calls, the factory kind, or the first key of a run can change the generated Rust and requires rebuilding.

A debug host with both `hot-reload` and `binary-resources` can stage bytes under that ID and reconstruct the theme under the candidate snapshot. A host can also call `Batch::decode` on a worker thread, transfer the validated batch to the UI thread, and call `replace_decoded` there. Publish the new theme after successful reconstruction. Existing deferred resources keep their original generation alive. The development host can watch a file and replace a mounted theme through the same style contribution change and rollback machinery that handles source edits. Enable `ferroui-hot-reload/binary-resources`, then pass the module’s debug-only `FXB_MANIFEST` constant to the watcher. It supplies the build path and embedded baseline of a JSON manifest containing the exact relative leaf names, factory kinds, per-leaf FNV-1a/64 content fingerprints, module path and native caller fingerprint. The producer writes all leaves and atomically renames the new manifest into place last. Each leaf is validated against this generation before the watcher constructs a candidate. A torn multi-file update preserves the mounted theme. External editors publishing a block must update its fingerprint in the generation manifest. To watch a deployed directory, supply its manifest path paired with the same embedded baseline. A changed native caller or leaf layout requests a restart; a missing, malformed or out-of-directory input rejects the update. The UI loop calls `poll`; the existing file watcher debounces partial saves, and the host validates each stable batch before replacing the contribution. Other styles in the container keep their identity and order. The compiled constructor callback can also restore an application's density or other theme settings.

```rust
use ferroui_hot_reload::BinaryThemeWatcher;
use std::time::Duration;

let mut watcher = BinaryThemeWatcher::from_manifest(
    &owner_styles, theme_index,
    ferroui_themes_winui::compiled_xaml::FXB_MANIFEST,
    Duration::from_millis(50),
)?;

// On the UI tick where the application normally pumps reload events.
watcher.poll(|| Ok(ferroui_themes_winui::WinUITheme::new().as_style()))?;
```

The lower-level snapshot API supports custom publication and rollback policies.

```rust
use std::rc::Rc;
use ferroui_markup_xaml::{
    xaml_il::runtime::compiled::binary_resources::{reload::Snapshot, Kind},
    XamlLoadError,
};

fn reconstruct<T>(
    previous: &Snapshot,
    block_id: &str,
    bytes: &[u8],
    build: impl FnOnce() -> Result<T, XamlLoadError>,
) -> Result<(Rc<Snapshot>, T), XamlLoadError> {
    let mut next = previous.clone();
    next.replace(block_id, Kind::Alias, bytes)?;
    let next = Rc::new(next);
    let theme = next.run(build)?;
    Ok((next, theme))
}
```

## Decode benchmark

With generated blocks in a fresh output directory, measure the decoder independently of I/O and theme construction.

```sh
cargo run --release -p ferroui-xaml-binary --example decode -- "$THEME_OUT"/xaml/*.fxb
```

The program reads the inputs once, decodes and counts every record, then reports the median and 95th percentile of 101 complete in-memory validation, expansion and drop passes. Throughput uses stored bytes.

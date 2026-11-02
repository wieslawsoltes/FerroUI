# FXP v2 wire layout

An FXP v2 reader expands the archive into an indexed FXP v1 image. Each directory entry points to a stored FXD v1 document in columnar mode 2. The document retains the original local numbering of strings, lexical scopes and syntax nodes. The Rust implementation is in `binary_document/solid.rs`. [FXD graph and indexed FXP layouts](fxd.md)

## Envelope

| Offset | Wire type | Field |
| ---: | --- | --- |
| 0 | 4 bytes | `46 58 50 00` (`FXP\0`) |
| 4 | u8 | Version 2 |
| 5 | little-endian u32 | Uncompressed byte length of the following zlib stream |
| 9 | bytes to EOF | Exactly one RFC 1950 stream, including its checksum |

Inflate through end-of-stream and check the checksum and both byte counts. The inflater must consume the entire input and produce exactly the declared length. Concatenated zlib streams and bytes after the trailer are invalid.

## Inflated payload

Every integer below uses canonical unsigned 32-bit LEB128. A **byte string** is a byte length followed by exactly that many bytes. The three sections occur in the following order.

| Section | Fields in wire order |
| --- | --- |
| Global strings | String count. For each string, a prefix byte count followed by a byte string containing the suffix. Copy the specified bytes from the preceding string and append the suffix. The first prefix is zero. The result must be valid UTF-8, and the reconstructed strings must increase in bytewise lexicographic order. Prefixes are byte counts and may end inside a code point. |
| Directory | Document count. For each document, a UTF-8 path byte string, the reconstructed FXD length including its six-byte header, the local string count, then five byte lengths in order for metadata, tags, source lines, source columns and scope references. Paths are unique relative slash-separated asset paths in bytewise lexicographic order. |
| Data | Twelve byte strings in order. The first holds local string mappings. The next five hold metadata, tags, source lines, source columns and scope references. The last six hold operand rows grouped by opcode 0 through 5. Each stream concatenates contributions in directory order. |

The metadata bytes start with the FXD document-name reference and root namespace aliases. They continue with the scope count and scope table, the node count and the three lengths for FXD line, position and scope streams. Path sorting defines the physical index. The original node numbering and child lists determine XAML execution order.

## Reconstructing a member

For each directory entry, append the FXD header `46 58 44 00 01 02` and the local string count. Read that many integers from the local mapping stream. Decode each as ZigZag i32 and add it to the preceding global ID, starting from **zero for each document**. Check the resulting ID against the global table. Append the selected string as a byte string.

Copy each of the five per-document slices from its corresponding stream using the directory lengths. Keep a view of the tag slice. For each tag in order, copy one complete row from that tag's operand stream to the member. The number of nodes is the tag byte count. Each row has the following shape. An integer list is one count followed by that many unsigned integers.

| Tag | FXD node | Fixed integers | Integer lists after fixed fields |
| ---: | --- | ---: | ---: |
| 0 | XML type | 3 | 1 |
| 1 | Named property reference | 3 | 0 |
| 2 | Text | 3 | 0 |
| 3 | Object | 1 | 2 |
| 4 | Property assignment | 2 | 1 |
| 5 | XML directive | 2 | 1 |

The reconstructed size must match the directory entry. All twelve streams and the enclosing payload must end exactly at their boundaries. A reader can now expose the member as an ordinary FXD stream. A reload host validates every graph before publishing a replacement. The graph decoder checks node roles, reference ancestry, depth, string budgets and reachability. [FXD validation rules](fxd.md)

## Allocation limits

| Quantity | Maximum |
| --- | ---: |
| Compressed file, inflated payload and reconstructed FXP image, each | 256 MiB |
| Total materialized global string bytes | 256 MiB |
| Global string count | 1,048,576 |
| Documents | 16,384, with at least one document |
| UTF-8 path bytes | 4,096 |
| One reconstructed FXD image or global string | 32 MiB |
| Local strings and nodes per document, each | 524,288 |

Reject additions, offsets and signed-ID conversions that exceed their bounds. The reconstruction limit includes the complete 13-byte v1 header, directory and all FXD members. The reference reader inflates in fixed 16 KiB steps. It checks the output limit before appending each step.

## Ahead-of-time execution of an embedded revision

Enable `precompiled-theme` on `ferroui-themes-winui` to build a solid FXP revision and execute its XAML through typed Rust and FXB resource blocks. The build script gives the two compilers the same ordered source inventory after applying any explicit resource roots. A clean build pays for both the XamlX semantic pass and the archive encoder. Construction, compiled URI loading and exported dictionary builders execute the prelinked group. They do not inflate the solid archive, resolve type names or run XamlX transforms at startup.

An editor can obtain the complete embedded revision through `ferroui_themes_winui::theme_archive()`. That accessor is the only target-side reference to the archive in precompiled mode. A consumer retaining it ships both representations. A runtime-only consumer can let the linker discard the unused FXP. Supplying a changed archive to the normal style reload target validates and interprets that revision and runs the class constructor with its scoped populate override. Compiled resource and template identity is retained within the installed revision.

With Cargo feature unification, `precompiled-theme` selects this execution path even when another dependency enables `packed-theme` or `solid-theme`. Selecting only a portable mode retains its syntax interpreter. Applications can select `binary-resources` alone when they need only the prelinked representation.

For a portable revision with no linked AOT entry points, call `FerroRuntimeXamlLoader::prepare_packed_document` with its open index, root URI and configuration. Preparation discovers includes and resolves the XamlX program on the UI thread. The returned `PreparedPackedDocument` owns that program after the input bytes are dropped. Call `build(parent)` for each instance. Re-prepare when changing documents, build options or type registrations. Each activation runs the constructor and establishes a new root service provider and name scope.

An editor can retain a revision of an owned style slot with `StyleReloadTarget::prepare_packed`. Preparation validates every archive member and leaves the slot untouched. Calling `apply()` builds a fresh contribution and runs the existing checked replacement. Two prepared revisions for the same URI can be switched independently.

To restore the distributed revision through the editor, configure its style reload target with `with_compiled_archive(ferroui_themes_winui::compiled_archive_root())` and the WinUI local assembly. On `replace_packed` the target compares the complete incoming bytes with the compiled baseline, including when they occupy different allocations. It requires the same root URI and assembly, compatible binding and source-information settings, no diagnostic callback, and no applicable populate or FXB override. Matching bytes run the typed class loader inside the normal slot replacement. Other revisions follow portable validation and preparation. The descriptor is explicit so an application can choose which baseline it ships and trusts.

A Linux x86-64 release experiment measured six full WinUI activations per process through the first Button lookup. Across three alternating fresh processes per arm, the median of the second through sixth activations fell from 124.72 ms with normal archive loading to 49.66 ms with a prepared program. The prepared arm spent a median 1,588.02 ms in setup and 129.22 ms on its first build. The ordinary arm spent 13.07 ms in setup and 1,807.26 ms on its first build. Setup includes type registration, expansion and index opening. Both paths construct new objects. [Raw observations and reproduction](../../benchmarks/prepared-theme-linux-x86_64.json)

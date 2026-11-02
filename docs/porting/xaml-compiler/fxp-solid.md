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

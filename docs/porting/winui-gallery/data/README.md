# WinUI / Gallery parity data

See [`scripts/winui-gallery/README.md`](../../../../scripts/winui-gallery/README.md)
for commands, schemas, source boundaries and the distinction between declaration
candidates and behavioral evidence. Start with [the generated report](REPORT.md).

`sources.lock.json`, `api-mappings.json` and `api-evidence.json` are reviewed inputs. The upstream
snapshots are regenerated only from the pinned, clean repositories. All remaining
reports are regenerated from those snapshots plus current FerroUI sources and the
Gallery sample's curated port-status manifest. No file in this directory asserts
that a name match is functional or visual parity.

Current mappings point to the ordinary native control, navigation, primitive and
automation namespaces. Source `Page` maps explicitly to `FerroUI.Controls.FramePage`
so the existing `FerroUI.Controls.Page` contract remains intact. These identity and
file moves do not advance any behavior or Gallery status.

The current 521 API evidence entries reference
[`native-controls-005.json`](executions/native-controls-005.json): 182 passing native
control tests across 12 targets, with 53 frozen source hashes checked before and after
execution. This covers the promoted implementations, extracted helpers and updated
test imports without changing their parity statuses.

Execution records are immutable observations. Later source changes require new
execution evidence; the scanner rejects stale hashes even for `partial` entries
with execution records. Original WinUI/Gallery snapshots, source pins, captures and
earlier test records keep their original identities, counts and source revisions.

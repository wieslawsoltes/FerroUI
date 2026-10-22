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

The local [`native-controls-005.json`](executions/native-controls-005.json) preserves
182 passing native control tests across 12 targets, with 53 source hashes frozen
before and checked after the local execution. The current 521 API evidence entries
reference the fresh [`native-controls-006.json`](executions/native-controls-006.json):
182 native tests passed on actual CI checkout `63e82a17`, tree `523b4d4e`, with main
`87fb`'s updated `Cargo.lock`. Its hashes were extracted afterward from that checkout's
Git blobs, not frozen around execution. Published merge `aae91cbe` has the same tree
but was not the execution commit. This refresh does not change parity statuses.
The exact successful native-step log is retained as
[`native-controls-006.log.gz`](executions/native-controls-006.log.gz); decompress it
before comparing the record's raw-log SHA-256. The enclosing CI job later failed
three Gallery header-label assertions, which are recorded separately.

[`native-promotion-validation.json`](native-promotion-validation.json) records the
integration executions and CI observations for the published native migration,
capture-import fix and main integration. It preserves earlier failures, environment
limits, actual CI checkout identities and the distinction between the local 005
before/after hashes and hashes extracted from Git blobs afterward.

Execution records are immutable observations. Later source changes require new
execution evidence; the scanner rejects stale hashes even for `partial` entries
with execution records. Original WinUI/Gallery snapshots, source pins, captures and
earlier test records keep their original identities, counts and source revisions.

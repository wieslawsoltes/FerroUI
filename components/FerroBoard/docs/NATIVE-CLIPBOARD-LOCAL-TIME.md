# Native clipboard local timestamps

Clipboard import now accepts offset-less dates and times through Jiff 0.2.38 and
native time-zone data. Explicit wire offsets always take precedence. The public
Rust helpers (with TimeZone re-exported directly from Jiff) are parse_clipboard_timestamp_in_zone and
ClipboardHistory::import_with_time_zone. The original import method remains and
uses the native system zone when required. Core stays independent of controls.

The codec preserves the original local wall time, including a DST gap, and uses
the standard-time side of a DST ambiguity. Other ambiguous transitions use the
post-transition rule. TimeZone objects, TZif parsing, transition data and system
zone discovery come from the native Rust library. There is no TimeZoneInfo or
DateTimeOffset runtime emulation. Stored values remain time::OffsetDateTime.

Unknown/unavailable native zones return UnsupportedLocalTime. There is no UTC or
Clock fallback. Explicit-offset input still works with an unknown supplied zone.
Existing year 1–9999, ±14-hour/minute-offset and 100ns wire constraints remain.

The unchanged pinned ClipboardHistory.cs drives 192 import cases over eight frozen
IANA 2026e-rearguard TZif files: UTC, New York, Lisbon, Sydney, Kathmandu, Dublin,
Casablanca and Apia. Debug and Release source generation agree exactly on
.NET 10.0.13 / SDK 10.0.402. Ordinary Rust tests use only local frozen data.

183 source outcomes match, including imports/exports, offsets/instants, fractional
truncation, invalid inputs, partial-import state and notification order. Four new
Rust tests also cover default zone resolution, explicit-offset precedence, clock
isolation, unavailable zones and strict expiry after a repeated local hour.

Nine historical imports remain explicitly unsupported: native TZif supplies
sub-minute offsets, while the existing wire format accepts whole minutes. They
return TimestampError::OutOfRange, keep the already imported prefix and emit no
successful-import notification. The tests enumerate all nine source/native
differences and assert their errors. They are not counted as source parity. Native
historical offsets are not rounded to reproduce .NET's rule conversion.

See [exact rows, code hashes and checks](native-clipboard-local-time.json). Current
checks pass 746 workspace tests, 82 independent consumer tests, strict all-target
workspace/consumer Clippy, release library checks, sample linking and formatting.
Jiff adds 13 entries to the all-platform locked graph; existing locked versions are
unchanged. Root audit covers 252 packages / 396 retained license files. Mobile/browser
graphs are refreshed; this is not a new platform runtime claim.

System zone/data availability, negative-DST conventions and historical rules may
differ by platform/data release. The frozen fixture suite does not certify other
platforms or all zones. Earlier native and performance receipts retain their own
source/binary scope. The new archive's adjacent validation receipt records its
relocated builds and focused native run. A-001 remains deferred.

To regenerate the source corpus (optional .NET development tooling):

```sh
source scripts/env.sh
python3 scripts/clipboard-time-reference/generate_fixture.py --check --configuration Release
```

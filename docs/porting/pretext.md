# Pretext port

## Baseline and scope

Source: [PretextSharp](https://github.com/wieslawsoltes/PretextSharp), commit
`e70a544e43f7ecffee76d2d22a0224213d7fbba3` (MIT).
FerroUI baseline: `500095d6a688a1077dfaf1e06bfa0d3a6f7fbbb8`.

This port follows [PORTING-GUIDE.md](PORTING-GUIDE.md): preserve product
behavior and source traceability, use safe idiomatic Rust and existing framework
services, accompany algorithms with behavioral tests, and carry source attribution.

Included scope:

- Measurement and shaping contracts, CSS-like font descriptors.
- Reusable prepared text, Unicode segmentation, bidi segment levels, whitespace
  normalization, grapheme-aware wrapping, CJK keep-all and punctuation rules.
- Aggregate, materialized and incremental line layout; discretionary hyphens,
  tabs, hard breaks, geometry-only walking and natural-width measurement.
- Rich inline text and atomic inline items.
- Prepared shaping, glyph-run data and cached shaped line materialization.
- Platform-neutral wrap metrics, column flow and obstacle layout helpers.
- A native FerroUI measurement/shaping adapter using existing font services.
- Portable behavioral tests, an executable example, and CI validation.

Excluded by scope:

- Uno controls, scheduler and application hosts.
- AppKit and Uno sample shells and their application assets.
- C# native backend wrappers (DirectWrite, CoreText, FreeType, SkiaSharp);
  FerroUI already owns platform text services, font fallback and rendering.
- CLR reflection, assembly attributes, compatibility polyfills and NuGet tooling.

## Implementation plan

The draft PR is opened with this source baseline before implementation. Contracts,
core algorithms, rich inline/layout helpers and native integration are committed
in reviewable increments. The completed report will contain the source-to-Rust
inventory, intentional API translations, any behavioral differences, test counts
and actual validation results.

## Validation plan

Port relevant tests from `tests/Pretext.Uno.Tests`; despite the directory name,
these exercise the platform-neutral engine. Add focused tests for encoding
boundaries, cursor progress, prepared-object lifetime, backend replacement and
geometry-only layout. Run the relevant Rust tests, Clippy, formatting, naming and
packaging checks, plus the CI workflow on the PR.

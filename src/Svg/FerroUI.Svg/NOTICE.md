# FerroUI SVG backend notices

The export document, audited context, inspection views and XML optimizer in this crate are
original FerroUI code under the repository's MIT license.

This crate uses the workspace's Skia backend and `skia-safe`; their third-party notices are in
`src/Skia/FerroUI.Skia/NOTICE.md` and the dependencies' packages. XML parsing uses `roxmltree`
(MIT or Apache-2.0), gzip compression uses `flate2` with its pure Rust backend (MIT or Apache-2.0),
and embedded PNG data uses `base64` (MIT or Apache-2.0).

The example uses fonts installed on the host. Exporting glyph outlines does not change the font's
license; the application chooses fonts whose license permits its intended distribution.

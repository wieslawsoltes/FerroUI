# NOTICE

This sample is a port of the RenderDemo sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents (`**/*.xaml`) are converted from the upstream sample by
  `scripts/convert_catalog_xaml.py`; the conversion changes names only.
- The Rust sources are ports of the upstream C# sources of the sample, file by file;
  `Controls/line_bounds_helper.rs` is the port of the helper of the upstream base library that
  the upstream project file links into the sample.
- `tests/` are not ports: the upstream sample has no tests.

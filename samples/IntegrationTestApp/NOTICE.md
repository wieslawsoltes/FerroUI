# NOTICE

This crate is a port of the IntegrationTestApp sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents (`App.xaml`, `MainWindow.xaml`, `ShowWindowTest.xaml`,
  `TopmostWindowTest.xaml` and the documents under `Pages/`) are converted from the upstream
  documents by `scripts/convert_catalog_xaml.py`; the conversion changes names only.
- The Rust sources are ports of the upstream C# sources, file by file. `Embedding/objc.rs`
  is not a port: it holds the Objective-C runtime calls the embedding needs, in place of the
  AppKit binding package the upstream sample is built on.
- `Assets/icon.ico` is the artwork of this project, in place of the icon of the upstream
  project.

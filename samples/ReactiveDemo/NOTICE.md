# NOTICE

This crate is a port of the sample `ReactiveDemo` of ReactiveUI.Samples
(<https://github.com/reactiveui/ReactiveUI.Samples>, `wpf/getting-started/ReactiveDemo`, MIT,
Copyright (c) .NET Foundation and Contributors), the getting-started sample of the ReactiveUI
documentation.

- The markup documents are converted from the reference documents; the conversion changes names
  (the generic reactive window and user control are their non-generic base classes) and the
  inline hyperlink of the details view is a hyperlink button.
- The Rust sources are ports of the reference C# sources, file by file, with their comments. The
  sample browses the crates of crates.io where the reference browses the packages of NuGet:
  `crate_search.rs` stands for what the reference takes from NuGet.Protocol, and
  `code_behind_view.rs` for what the reference's views inherit from ReactiveUI.WPF.
- The descriptions of the crates of the in-memory catalogue (`crate_search.rs`) are written for
  this sample.

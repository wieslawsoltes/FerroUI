# NOTICE

This crate is a port of the demo application `TreeDataGridDemo` of TreeDataGrid
(<https://github.com/wieslawsoltes/TreeDataGrid>, MIT, Copyright (c) 2026 Wiesław Šoltés; earlier
parts MIT, Copyright (c) .NET Foundation and Contributors).

- The markup documents are converted from the reference documents; the conversion changes names,
  and the pages of the reference's main window are views of their own (`Views/`).
- The Rust sources are ports of the reference C# sources, file by file. `ViewModels/random.rs`
  reproduces the seeded pseudo-random number generator of the .NET runtime library (MIT).
- `Assets/file.png`, `Assets/folder.png` and `Assets/folder-open.png` are the icons of the reference
  demo, unchanged.
- `Models/countries.rs` holds the table of countries of the reference, unchanged.

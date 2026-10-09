# Third-party notices

The Vello render backend is an addition of FerroUI: it has no counterpart in
the upstream project. Its files follow the files of the Skia backend
(`src/Skia/FerroUI.Skia`), which is a port of upstream's `Avalonia.Skia`
(MIT, see the `NOTICE.md` at the root of the repository): the structure, the
names and the logic that is not Skia's own are derived from it.

## Crates the backend links

The backend links the crates below from crates.io at the exact versions the
workspace manifest names. All of them are licensed under the terms of both
the Apache License, Version 2.0 and the MIT license, at the choice of the
user; FerroUI uses them under the MIT license. The texts of both licenses
ship with each crate (`LICENSE-APACHE`, `LICENSE-MIT`).

| Crate | Version | Used for | Copyright |
|---|---|---|---|
| `kurbo` | 0.13.1 | paths, strokes, arcs, curve lengths, hit testing | Copyright (c) 2018 Raph Levien; the Kurbo Authors |
| `peniko` | 0.6.1 | brushes, gradients, images, blend modes | Copyright 2020 the Vello Authors; the Peniko Authors |
| `color` | 0.3.3 | colors of `peniko` | the Color Authors |
| `linebender_resource_handle` | 0.1.1 | shared pixel data of `peniko` | the Linebender Resource Handle Authors |
| `vello_cpu` | 0.3.0 | the CPU rendering mode | Copyright 2020 the Vello Authors |
| `vello_common` | 0.3.0 | shared parts of the sparse-strips renderers | Copyright 2020 the Vello Authors |
| `fearless_simd` | 0.7.0 | SIMD of `vello_common` | the Fearless SIMD Authors |
| `linesweeper` | 0.5.0 | boolean operations of combined geometries | Joe Neeman and the linesweeper contributors |
| `polycool` | 0.4.0 | polynomial roots of `kurbo` and `linesweeper` | the polycool contributors |
| `png` | 0.18.1 | PNG decoding and encoding | Copyright (c) 2015 nwin; the image-rs developers |

Their own dependencies (`bytemuck`, `smallvec`, `euclid`, `guillotiere`,
`thiserror`, `fdeflate`, `miniz_oxide`, `foldhash` and crates the workspace
links already) are under the MIT license, the Apache License 2.0, the zlib
license or a choice of them; `Cargo.lock` at the root of the repository
names every one with its version.

No source of these crates is copied into the repository.

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
user (the crates of the zune project also under the zlib license, and
`jpeg-encoder` with a further license, below); FerroUI uses them under the
MIT license. The texts of the licenses ship with each crate (`LICENSE-APACHE`,
`LICENSE-MIT`, `LICENSE-ZLIB`, `LICENSE-IJG`).

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
| `glifo` | 0.4.0 | glyph rendering of `vello_cpu` (its `text` feature): outlines, hinting, colour glyphs | Copyright 2025 the Vello Authors and the Parley Authors |
| `skrifa` | 0.44.0 | the outlines of glyphs, the axes of variable fonts, the names and attributes of a font | the Fontations Authors (Google LLC) |
| `read-fonts` | 0.41.0 | font tables for `skrifa` and `fontique` | the Fontations Authors (Google LLC) |
| `font-types` | 0.12.6 | scalar types of `read-fonts` | the Fontations Authors (Google LLC) |
| `fontique` | 0.12.0 | the installed fonts of the system: enumeration, matching, fallback | Copyright 2024 the Parley Authors |
| `parlance` | 0.1.1 | text property types of `fontique` | the Parley Authors |
| `zune-jpeg` | 0.5.15 | JPEG decoding | Copyright (c) zune-image developers |
| `zune-bmp` | 0.5.2 | BMP decoding, and the bitmaps of an ICO | Copyright (c) zune-image developers |
| `zune-core` | 0.5.3 | options and readers of the two zune crates | Copyright (c) zune-image developers |
| `jpeg-encoder` | 0.7.1 | JPEG encoding | Copyright (c) 2021 Volker Ströbel |
| `gif` | 0.14.2 | GIF decoding | Copyright (c) 2015 nwin; the image-rs developers |
| `weezl` | 0.1.12 | LZW decompression of `gif` | Copyright (c) HeroicKatora 2020 |

`jpeg-encoder` is licensed "(MIT OR Apache-2.0) AND IJG": parts of it (the
quantization and Huffman tables and the scaling of a quality to them, and
its SIMD code, which this workspace does not enable) derive from the software
of the Independent JPEG Group, whose terms ask for this statement in the
documentation of a product that uses it in executable form: **this software is
based in part on the work of the Independent JPEG Group.**

With the feature `hybrid` of the crate (the hybrid rendering mode):

| Crate | Version | Used for | Copyright |
|---|---|---|---|
| `vello_gpu` | 0.3.0 | the hybrid rendering mode (the crate that was `vello_hybrid`) | Copyright 2025 the Vello Authors |
| `vello_gpu_shaders` | 0.3.0 | the shaders of `vello_gpu` | Copyright 2025 the Vello Authors |

With the feature `gpu` (the GPU rendering mode):

| Crate | Version | Used for | Copyright |
|---|---|---|---|
| `vello` | 0.11.0 | the GPU rendering mode | Copyright 2022 the Vello Authors |
| `vello_encoding` | 0.11.0 | the scene encoding of `vello` | Copyright 2022 the Vello Authors |
| `vello_shaders` | 0.11.0 | the compute shaders of `vello`; its shader sources are offered under the Unlicense as well | Copyright 2022 the Vello Authors |
| `skrifa`, `read-fonts`, `font-types` | 0.44.0, 0.41.0, 0.12.6 | font tables, linked by `vello` | the Fontations Authors |

With either of them:

| Crate | Version | Used for | Copyright |
|---|---|---|---|
| `wgpu`, `wgpu-core`, `wgpu-hal`, `wgpu-types`, `wgpu-naga-bridge`, `wgpu-core-deps-apple` | 30.0.1 | the graphics API of both GPU modes (Metal on macOS) | Copyright (c) the gfx-rs developers |
| `naga`, `naga-types` | 30.0.1 | shader translation of `wgpu` | Copyright (c) the gfx-rs developers |

Their own dependencies (`bytemuck`, `smallvec`, `euclid`, `guillotiere`,
`thiserror`, `fdeflate`, `miniz_oxide`, `foldhash`, `hashbrown`, `log`,
`memmap2` and crates the workspace links already) are under the MIT
license, the Apache License 2.0, the zlib license or a choice of them;
`Cargo.lock` at the root of the repository names every one with its
version.

`fontique` reaches the font interface of each platform through bindings
that are linked on that platform only: on Apple platforms `objc2`,
`objc2-encode` and `objc2-foundation` (MIT) and `objc2-core-foundation` and
`objc2-core-text` (zlib, Apache License 2.0 or MIT, at the choice of the
user); on Windows `windows` and `windows-core` with their parts (MIT or
Apache License 2.0); on Linux and FreeBSD `yeslogic-fontconfig-sys` and
`dlib` (MIT); on Android `roxmltree` (MIT or Apache License 2.0).

With the GPU modes the crates above link, beside crates named already,
`objc2-metal`, `objc2-quartz-core`, `objc2-core-graphics`, `block2`,
`parking_lot`, `lock_api`, `raw-window-handle`, `raw-window-metal`,
`codespan-reporting`, `bit-set`, `indexmap`, `half`, `libm`,
`futures-intrusive`, `static_assertions`, `profiling`, `rustc-hash` and
`libloading`: under the MIT license, the Apache License 2.0, the zlib
license, the ISC license (`libloading`) or a choice of them.

No source of these crates is copied into the repository.

## Assets

`assets/noise_asset_256x256_png.png`, the noise texture of acrylic materials,
is the asset of the Skia backend (`src/Skia/FerroUI.Skia/assets`), which has
it from the upstream project named above (MIT).

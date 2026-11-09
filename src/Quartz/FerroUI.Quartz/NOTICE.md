# Third-party notices

The Quartz render backend is an addition of FerroUI: it has no counterpart in
the upstream project. It draws and shapes text with the system frameworks of
macOS and iOS (Core Graphics, Core Text, ImageIO, Metal and, where available,
Metal Performance Shaders), which it links from the operating system. No source of another
project is copied into the crate.

## Crates the backend links

The optional `runtime-shaders` feature links `naga` from crates.io at the
version the workspace manifest names, under the terms of both the Apache
License, Version 2.0 and the MIT license, at the choice of the user; FerroUI
uses it under the MIT license. The texts of the licenses ship with the crate.

## Assets

`assets/noise_asset_256x256_png.png`, the noise texture of acrylic materials,
is the asset of the Skia backend (`src/Skia/FerroUI.Skia/assets`), which has
it from the upstream project (MIT, see the `NOTICE.md` at the root of the
repository).

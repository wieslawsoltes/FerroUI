# Attribution

FerroRibbon derives from RibbonSpace by Wiesław Šoltés, licensed under MIT
(https://github.com/wieslawsoltes/RibbonSpace, commit
9755e281547923f10397dffa366e3355b12b55ea). The MIT license is in `LICENSE` and
`src/FerroRibbon.Core/NOTICE.md`.

FerroUI is a separately licensed MIT dependency, consumed from this repository by path.
Its license and notices are at the root of the repository.

Portions adapted from Uno Platform (Copyright (c) Uno Platform Inc., Apache-2.0) and from
Microsoft sources distributed with it (Copyright (c) Microsoft Corporation, MIT) are
identified in the `NOTICE.md` files next to the code they cover, under `src/FerroRibbon`,
`src/FerroRibbon/compat`, `src/FerroRibbon/controls` and `src/FerroRibbon/metadata`. The
Apache-2.0 license is in `src/FerroRibbon/LICENSE-APACHE-2.0.md`.

The embedded `src/FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf` font is the unchanged
font of Uno.Fonts.Fluent 2.6.1, Copyright (C) 2015-2023 Uno Platform inc., Apache-2.0.

`src/FerroRibbon.Platform/LICENSE-ICU` is the license of ICU, which that crate links on
Linux and in the browser. `samples/FerroRibbon.Browser/wwwroot/Inter/NOTICE.md` covers the
Inter font of the browser sample.

Icons named by `http(s)` URIs use reqwest (MIT OR Apache-2.0) and Tokio (MIT) when the
`http-images` feature is enabled. SVG icons use resvg (Apache-2.0 OR MIT). The async
command boundary uses futures-rs (MIT OR Apache-2.0). The license texts of direct
third-party dependencies are in `docs/licenses`.

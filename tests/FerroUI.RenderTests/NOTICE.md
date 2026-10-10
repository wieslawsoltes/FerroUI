# Third-party notices

## Expected images and source images of the render tests

The files under `TestFiles/` and `Assets/` are test data: the images the
render tests compare their output with, and the images and pixel data some
of the tests draw. They are used by the tests of this crate only. They were
copied unmodified, under their upstream names, from the Avalonia repository
(<https://github.com/AvaloniaUI/Avalonia>, MIT, see the root `NOTICE.md` for
the licence text) at the commit the port tracks (`docs/porting/TRACKING.md`):

| Directory | Upstream directory | Files | Size |
|---|---|---|---|
| `TestFiles/Skia` | `tests/TestFiles/Skia` | 300 | 7.9 MB |
| `TestFiles/CrossTests` | `tests/TestFiles/CrossTests` | 30 | 0.1 MB |
| `TestFiles/PixelFormats` | `tests/TestFiles/PixelFormats` | 30 | 4.8 MB |
| `Assets` (`Ramp64.png`, `Star512.png`) | `tests/Avalonia.RenderTests/Assets` | 2 | 2 KB |

Copyright (c) AvaloniaUI OÜ and the contributors of the Avalonia project.

Among the source images, `TestFiles/Skia/Controls/Image/blend/Cat.jpg` and
`TestFiles/Skia/Controls/Image/blend/ColourShading - by Stib.png` are the
sample images of the blending modes that upstream distributes as test
assets; neither file states a licence of its own. The files under
`TestFiles/PixelFormats/Lenna` are encodings of the standard test image
"Lenna" in the pixel formats of the bitmap tests, as upstream distributes
them.

## Test fonts

The tests register the fonts of upstream's render tests as resources of
this crate. The font files are not copied here: they are the ones under
`src/Skia/FerroUI.Skia/test_assets/assets`, whose copyrights and licences
are listed in `src/Skia/FerroUI.Skia/NOTICE.md`.

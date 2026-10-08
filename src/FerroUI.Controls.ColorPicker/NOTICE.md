# NOTICE

## Source code

The crate is ported from the `Avalonia.Controls.ColorPicker` project of
Avalonia (MIT; see the `NOTICE.md` at the root of the repository). The theme
documents under `Themes/` are converted from the theme documents of that
project (`scripts/sync-color-picker-themes.sh`).

## WinUI

The following files are ported from sources that the upstream project adapted
from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml>), MIT
License, Copyright (c) Microsoft Corporation. Each carries the notice of its
origin.

- `color_changed_event_args.rs`
- `hsv_component.rs`
- `color_spectrum/color_spectrum.rs`
- `color_spectrum/color_spectrum_properties.rs`
- `color_spectrum/color_spectrum_components.rs`
- `color_spectrum/color_spectrum_shape.rs`
- `helpers/color_picker_helpers.rs`
- `helpers/hsv.rs`
- `helpers/increment_amount.rs`
- `helpers/increment_direction.rs`
- `helpers/rgb.rs`

```
MIT License

Copyright (c) Microsoft Corporation.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE
```

## Flat UI colours

The colours of `color_palettes/flat_color_palette.rs` (and of the half
palette built on it) come, as upstream states, from the Flat UI project
(<https://github.com/designmodo/Flat-UI>, MIT License).

# NOTICE

This sample derives its catalog, descriptions, sample source and native XAML adaptations from [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery), pinned to commit `14061eeac21c38ae99f302edf3d9136d4f652c4c`. `catalog_data.rs` retains the full catalog and original sample snippets; `Pages/` contains explicitly documented FerroUI adaptations. The source file paths and SHA-256 hashes are preserved in the catalog and the parity inventory.

The gallery shell is new FerroUI code. The Rust sample behaviors, navigation layouts
and color-contrast calculation adapt the pinned Gallery sources cited in their
files; the original C# is retained for reference and is not executed.

`Assets/ControlImages/` contains 108 original PNG files, and `Assets/SampleMedia/`
contains 22 original JPEG, GIF and SVG files for the Image, FlipView and
ConnectedAnimation examples, all copied byte-for-byte from the same pinned Gallery
repository. Keeping the original media does not imply support for SVG decoding or
animated GIF playback. `assets-manifest.json` records each original
path, SHA-256 hash, size and native resource registration. The source URI spellings
`CheckBox.png` and `AnnotatedScrollbar.png` are registered as aliases for the
original `Checkbox.png` and `AnnotatedScrollBar.png` files, preserving Windows
path resolution on case-sensitive hosts. `cat_data.rs` preserves the original
97 cat names from `AutoSuggestBoxPage.xaml.cs`; its source hash is recorded inline.

## Microsoft WinUI Gallery

    MIT License

    Copyright (c) Microsoft Corporation. All rights reserved.

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

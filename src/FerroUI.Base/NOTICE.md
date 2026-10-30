# NOTICE

Crate-level notices for `ferroui-base` in addition to the project-wide notices in the repository root `NOTICE.md`. Notices of code derived from third-party sources live next to that code (`media/NOTICE.md`, `utilities/NOTICE.md` and the others under this directory).

## WinUI rich text

Parts of the text formatting of `ferroui-base` adapt source and public contracts of [microsoft/microsoft-ui-xaml](https://github.com/microsoft/microsoft-ui-xaml), branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (`docs/porting/winui-rich-text.md`):

- `media/text_formatting/font_family_line_metrics.rs`, `text_line_metrics_mode.rs`, the family-metrics path of `text_line_impl.rs`, `media/font_manager.rs` (`map_characters`), `media/font_fallback_mapping.rs` and `platform/i_font_fallback_mapper.rs`: the requested-family line metrics and the fallback font scale of `dxaml/xcp/core/text/Fonts/CompositeFontFamily.cpp`, `Fonts/FontTypeface.cpp` and `RichTextServices/TextFormatter/` (`TextSegment.cpp`, `LsRun.cpp`, `LsTextLine.cpp`, `lineservicescallbacks.cpp`, `InlineObjectHandlers.cpp`);
- `media/text_formatting/text_tab_stop.rs`, `text_tab_layout.rs` and the tab leaders and bar tabs of `shaped_text_run.rs` and `text_line_impl.rs`: the tab alignment, leader and tab-stop contract of `packaging/Intellisense/generated/Microsoft.UI.Text.xml`;
- `media/font_typography.rs`: the seven typography enumerations and their values, declared in `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.cs`.

```text
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
SOFTWARE.
```

## Test fonts

The fonts under `test_assets/fonts/` are used by the crate's tests only (`media/glyph_typeface_tests.rs`, the font table tests, `media/synthetic_font_variable_tests.rs`); they are not part of the library. Each font carries its own licence.

| File | Origin | Copyright | Licence |
|---|---|---|---|
| `Inter-Regular.ttf` | Copied unmodified from `src/FerroUI.Fonts.Inter/Assets` (the same file as `tests/Avalonia.RenderTests/Assets/Inter-Regular.ttf` of the Avalonia repository). | Copyright 2020 The Inter Project Authors (https://github.com/rsms/inter) | SIL Open Font License 1.1 |
| `AdobeBlank2VF.ttf` | Copied unmodified from `tests/Avalonia.RenderTests/Assets` of the Avalonia repository. | © 2013-2019 Adobe (http://www.adobe.com/) | SIL Open Font License 1.1 |
| `InterVariable.ttf` | Copied unmodified from `tests/Avalonia.RenderTests/Assets` of the Avalonia repository. | Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter) | SIL Open Font License 1.1 |
| `WenQuanYiMicroHei-Subset.ttf` | Modified: WenQuanYi Micro Hei (`samples/ControlCatalog/Assets/Fonts/WenQuanYiMicroHei-01.ttf`) subset to U+0020, U+4E2D, U+6587 and U+5B57 by `scripts/generate_glyph_typeface_test_fonts.py`. | Digitized data copyright © 2007, Google Corporation. Copyright © 2008-2009 WenQuanYi Board of Trustees (http://wenq.org/) and Qianqian Fang | Apache License 2.0 |
| `WenQuanYiMicroHei-NoHead.ttf` | Modified: the subset above reduced to its `OS/2`, `cmap`, `maxp`, `name` and `post` tables, with `head` renamed to `bhed`, by the same script. | As above | Apache License 2.0 |

The SIL Open Font License 1.1 is available at https://scripts.sil.org/OFL.

The two WenQuanYi Micro Hei files are licensed under the Apache License, Version 2.0 (the "License"); you may not use these files except in compliance with the License. You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.

## WinUI input scopes

`input/input_scope.rs` and `input/input_scope_name_value.rs` port WinUI's `InputScope`, `InputScopeName` and the values of `InputScopeNameValue`, declared in `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Input.cs` and `dxaml/xcp/dxaml/idl/winrt/core/microsoft.ui.xaml.coretypes.idl` of [microsoft/microsoft-ui-xaml](https://github.com/microsoft/microsoft-ui-xaml), branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (`docs/porting/winui-rich-text.md`).

```text
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
SOFTWARE.
```

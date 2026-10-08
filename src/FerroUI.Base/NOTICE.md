# NOTICE

Crate-level notices for `ferroui-base` in addition to the project-wide notices in the repository root `NOTICE.md`. Notices of code derived from third-party sources live next to that code (`media/NOTICE.md`, `utilities/NOTICE.md` and the others under this directory).

## Test fonts

The fonts under `test_assets/fonts/` are used by the crate's tests only (`media/glyph_typeface_tests.rs`); they are not part of the library. Each font carries its own licence.

| File | Origin | Copyright | Licence |
|---|---|---|---|
| `Inter-Regular.ttf` | Copied unmodified from `src/FerroUI.Fonts.Inter/Assets` (the same file as `tests/Avalonia.RenderTests/Assets/Inter-Regular.ttf` of the Avalonia repository). | Copyright 2020 The Inter Project Authors (https://github.com/rsms/inter) | SIL Open Font License 1.1 |
| `AdobeBlank2VF.ttf` | Copied unmodified from `tests/Avalonia.RenderTests/Assets` of the Avalonia repository. | © 2013-2019 Adobe (http://www.adobe.com/) | SIL Open Font License 1.1 |
| `WenQuanYiMicroHei-Subset.ttf` | Modified: WenQuanYi Micro Hei (`samples/ControlCatalog/Assets/Fonts/WenQuanYiMicroHei-01.ttf`) subset to U+0020, U+4E2D, U+6587 and U+5B57 by `scripts/generate_glyph_typeface_test_fonts.py`. | Digitized data copyright © 2007, Google Corporation. Copyright © 2008-2009 WenQuanYi Board of Trustees (http://wenq.org/) and Qianqian Fang | Apache License 2.0 |
| `WenQuanYiMicroHei-NoHead.ttf` | Modified: the subset above reduced to its `OS/2`, `cmap`, `maxp`, `name` and `post` tables, with `head` renamed to `bhed`, by the same script. | As above | Apache License 2.0 |

The SIL Open Font License 1.1 is available at https://scripts.sil.org/OFL.

The two WenQuanYi Micro Hei files are licensed under the Apache License, Version 2.0 (the "License"); you may not use these files except in compliance with the License. You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.

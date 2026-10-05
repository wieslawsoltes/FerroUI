# NOTICE

## WPF

`grid.rs` and `definition_base.rs` (with the definition collections and
definitions they use: `definition_list.rs`, `column_definition.rs`,
`column_definitions.rs`, `row_definition.rs`, `row_definitions.rs`) and
`converters/border_gap_mask_converter.rs` are ported
from sources that the upstream project adapted from the Windows Presentation
Foundation project (<https://github.com/dotnet/wpf>).

The upstream files `Grid.cs`, `DefinitionBase.cs`, `DockPanel.cs`,
`StackPanel.cs`, `WrapPanel.cs`, `GridSplitter.cs` and `Primitives/Track.cs`
carry this licence line in their header, verbatim (the Rust files derived
from them keep the adaptation note of the header, not this line):

```
// Licensed to The Avalonia Project under MIT License, courtesy of The .NET Foundation.
```

```
The MIT License (MIT)

Copyright (c) .NET Foundation and Contributors

All rights reserved.

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

## wayland-protocols (xdg_shell)

The documentation and flag names in `primitives/popup_positioning/i_popup_positioner.rs` are initially taken
from the xdg_shell wayland protocol this API is designed after, as in the
upstream project (Avalonia), so the license of the wayland-protocols
repository is included here.

```
Copyright © 2008-2013 Kristian Høgsberg
Copyright © 2010-2013 Intel Corporation
Copyright © 2013      Rafael Antognolli
Copyright © 2013      Jasper St. Pierre
Copyright © 2014      Jonas Ådahl
Copyright © 2014      Jason Ekstrand
Copyright © 2014-2015 Collabora, Ltd.
Copyright © 2015      Red Hat Inc.

Permission is hereby granted, free of charge, to any person obtaining a
copy of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next
paragraph) shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.

---

The above is the version of the MIT "Expat" License used by X.org:

    https://cgit.freedesktop.org/xorg/xserver/tree/COPYING


Adjustments for Avalonia needs:
Copyright © 2019 Nikita Tsukanov
```

## WinUI

`items_source_view.rs` and `selection/index_range.rs` (and the selection model
built on them under `selection/`) are ported from sources that the upstream
project adapted from the WinUI project
(<https://github.com/microsoft/microsoft-ui-xaml>), MIT License,
Copyright (c) Microsoft Corporation.

## Silverlight Toolkit

The files under `auto_complete_box/` (`auto_complete_box.rs`,
`auto_complete_filter_mode.rs`, `populated_event_args.rs`,
`populating_event_args.rs`), the files under `calendar/` (`calendar.rs`,
`calendar_blackout_dates_collection.rs`, `calendar_button.rs`,
`calendar_date_range.rs`, `calendar_day_button.rs`, `calendar_extensions.rs`,
`calendar_item.rs`, `date_time_helper.rs`, `selected_dates_collection.rs`),
the files under `calendar_date_picker/` (`calendar_date_picker.rs`,
`calendar_date_picker_date_validation_error_event_args.rs`,
`calendar_date_picker_format.rs`), `utils/i_selection_adapter.rs` and
`utils/selecting_items_control_selection_adapter.rs` are ported from sources
that the upstream project adapted from the Silverlight Toolkit
(<https://github.com/microsoftarchive/SilverlightToolkit>),
(c) Copyright Microsoft Corporation. Those files carry the notice of their
origin and are subject to the license below.

```
Microsoft Public License (MS-PL)

This license governs use of the accompanying software. If you use the software, you
accept this license. If you do not accept the license, do not use the software.

1. Definitions
The terms "reproduce," "reproduction," "derivative works," and "distribution" have the
same meaning here as under U.S. copyright law.
A "contribution" is the original software, or any additions or changes to the software.
A "contributor" is any person that distributes its contribution under this license.
"Licensed patents" are a contributor's patent claims that read directly on its contribution.

2. Grant of Rights
(A) Copyright Grant- Subject to the terms of this license, including the license conditions and limitations in section 3, each contributor grants you a non-exclusive, worldwide, royalty-free copyright license to reproduce its contribution, prepare derivative works of its contribution, and distribute its contribution or any derivative works that you create.
(B) Patent Grant- Subject to the terms of this license, including the license conditions and limitations in section 3, each contributor grants you a non-exclusive, worldwide, royalty-free license under its licensed patents to make, have made, use, sell, offer for sale, import, and/or otherwise dispose of its contribution in the software or derivative works of the contribution in the software.

3. Conditions and Limitations
(A) No Trademark License- This license does not grant you rights to use any contributors' name, logo, or trademarks.
(B) If you bring a patent claim against any contributor over patents that you claim are infringed by the software, your patent license from such contributor to the software ends automatically.
(C) If you distribute any portion of the software, you must retain all copyright, patent, trademark, and attribution notices that are present in the software.
(D) If you distribute any portion of the software in source code form, you may do so only under this license by including a complete copy of this license with your distribution. If you distribute any portion of the software in compiled or object code form, you may only do so under a license that complies with this license.
(E) The software is licensed "as-is." You bear the risk of using it. The contributors give no express warranties, guarantees or conditions. You may have additional consumer rights under your local laws which this license cannot change. To the extent permitted under your local laws, the contributors exclude the implied warranties of merchantability, fitness for a particular purpose and non-infringement.
```

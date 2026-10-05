# NOTICE

Component-level notices for code under `src/FerroUI.Base/media/` that is derived from third-party sources in addition to the project-wide notices in the repository root `NOTICE.md`.

## Elliptical arc approximation

- File: `precise_elliptic_arc_helper.rs` (ported from Avalonia `src/Avalonia.Base/Media/PreciseEllipticArcHelper.cs`)
- Origin: <http://www.spaceroots.org/documents/ellipse/EllipticalArc.java>
- License: BSD 3-Clause

Upstream file header, unabridged:

```
Copyright © 2003-2004, Luc Maisonobe
2015 - Alexey Rozanov <thehdotx@gmail.com> - Adaptations for Avalonia and oval center computations
2022 - Alexey Rozanov <thehdotx@gmail.com> - Fix for arcs sometimes drawn in inverted order.
All rights reserved.

Redistribution and use in source and binary forms, with
or without modification, are permitted provided that
the following conditions are met:

   Redistributions of source code must retain the
   above copyright notice, this list of conditions and
   the following disclaimer.
   Redistributions in binary form must reproduce the
   above copyright notice, this list of conditions and
   the following disclaimer in the documentation
   and/or other materials provided with the
   distribution.
   Neither the names of spaceroots.org, spaceroots.com
   nor the names of their contributors may be used to
   endorse or promote products derived from this
   software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND
CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED
WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF
USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER
IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.

C#/WPF/Avalonia adaptation by Alexey Rozanov <thehdotx@gmail.com>, 2015.
I do not mind if anyone would find this adaptation useful, but
please retain the above disclaimer made by the original class
author Luc Maisonobe. He worked really hard on this subject, so
please respect him by at least keeping the above disclaimer intact
if you use his code.
```

## Rounded rectangle keypoints

- File: `geometry_builder.rs` (ported from Avalonia `src/Avalonia.Base/Media/GeometryBuilder.cs`)
- Origin: portions adapted from the Windows Presentation Foundation (WPF) project (<https://github.com/dotnet/wpf>, licensed to the upstream project under the MIT License, courtesy of The .NET Foundation) and from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml/tree/winui3/main>, MIT License, Copyright (c) Microsoft Corporation).

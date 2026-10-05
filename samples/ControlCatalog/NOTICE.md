# NOTICE

This sample is a port of the ControlCatalog sample of the upstream project
(Avalonia, MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents (`**/*.xaml`) are converted from the upstream sample by
  `scripts/convert_catalog_xaml.py`; the conversion changes names only.
- The Rust sources are ports of the upstream C# sources of the sample, file by file.
- The files under `Assets/` and `Pages/teapot.bin` are copied unchanged from the upstream
  repository (`samples/ControlCatalog/Assets`, `samples/ControlCatalog/Pages/teapot.bin` and the
  application icons `build/Assets/icon.ico`, `build/Assets/icon-32.png`) by
  `scripts/sync-control-catalog.sh`.

## Assets with a licence of their own

The upstream repository ships these files without separate licence files; the licences below are
the ones the files themselves state (font name tables) or their authors publish.

### Source Sans Pro

`Assets/Fonts/SourceSansPro-Regular.ttf`, `SourceSansPro-Bold.ttf`, `SourceSansPro-Italic.ttf`,
`SourceSansPro-BoldItalic.ttf`.

Copyright 2010, 2012, 2014 Adobe Systems Incorporated (http://www.adobe.com/), with Reserved Font
Name 'Source'. All Rights Reserved. Source is a trademark of Adobe Systems Incorporated in the
United States and/or other countries.

This Font Software is licensed under the SIL Open Font License, Version 1.1.

```
-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded, 
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.
```

### WenQuanYi Micro Hei

`Assets/Fonts/WenQuanYiMicroHei-01.ttf`. The font states "Licensed under the Apache License,
Version 2.0" (http://www.apache.org/licenses/LICENSE-2.0) in its name table; it derives from
Droid Sans Fallback (Copyright Google, Apache License 2.0) and is published by the WenQuanYi
project (Copyright WenQuanYi Board of Trustees) under the Apache License 2.0 or the GPL version 3
with the font embedding exception.

## Images

The images under `Assets/` (the photographs of the demo applications in `Assets/CurvedHeader`,
`Assets/ModernApp`, `Assets/Movies`, `Assets/Pulse`, `Assets/Restaurant`, `Assets/RetroGaming`,
`Assets/Sanctuary`, and the pictures and logos directly under `Assets/`) are part of the upstream
repository, which records no separate origin or licence for them; they are redistributed here as
part of the upstream sample under the upstream licence. Three of the pictures
(`delicate-arch-896885_640.jpg`, `hirsch-899118_640.jpg`, `maple-leaf-888807_640.jpg`) carry
Pixabay file names.

# NOTICE

The Rust sources of this crate and the TypeScript modules under `webapp/modules` are ported from the
`Avalonia.Browser` project of Avalonia (`src/Browser/Avalonia.Browser`, MIT; see the `NOTICE.md` at the root of
the repository). The TypeScript modules follow the upstream modules file by file with the names of the framework
changed and the parts of the threaded mode removed.

`webapp/modules/ferroui/caniuse.ts` contains the mobile browser detection expressions of
http://detectmobilebrowsers.com/ (public domain), as the upstream module does.

`webapp/modules/ferroui/caretHelper.ts` is based on textarea-caret-position
(https://github.com/component/textarea-caret-position, MIT), as the upstream module is:

    The MIT License (MIT)

    Copyright (c) 2015 Jonathan Ong me@jongleberry.com

    Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
    associated documentation files (the "Software"), to deal in the Software without restriction, including
    without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the
    following conditions:

    The above copyright notice and this permission notice shall be included in all copies or substantial
    portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
    LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
    NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
    WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
    SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

The storage bundle (`storage.js`, built from `webapp/modules/storage.ts`) includes the
`native-file-system-adapter` polyfill (https://github.com/jimmywarting/native-file-system-adapter, MIT,
Copyright (c) 2019 Jimmy Wärting), pinned in `webapp/package.json` to commit
`d43ad841581c2cc3ce47bbd1e8f11950ebdff027`, the commit the upstream project pins. Its optional dependency
`fetch-blob` (MIT) is installed with it and is not bundled.

Build-time tools, not distributed with the crate: esbuild (MIT) and TypeScript (Apache-2.0), pinned in
`webapp/package.json` and `webapp/package-lock.json`.

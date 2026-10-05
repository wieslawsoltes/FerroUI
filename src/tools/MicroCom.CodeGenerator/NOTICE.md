# NOTICE

`microcom-codegen` is an independent Rust implementation of the IDL dialect and
of the C++ header output format of **MicroCom**
(<https://github.com/kekekeks/MicroCom>, Copyright 2021 © Nikita Tsukanov, MIT
license). No MicroCom source code is included; the IDL grammar and the shape of
the generated C++ header were reproduced from MicroCom's inputs and outputs so
that the same IDL files produce an equivalent header.

The test fixtures in `tests/fixtures/` are an IDL file from the Avalonia
project (<https://github.com/AvaloniaUI/Avalonia>, MIT license, Copyright (c)
AvaloniaUI OÜ — see `native/FerroUI.Native/NOTICE.md` for the full license
text) at upstream revision 5378af03f1 and the header MicroCom generated from
it, both passed through the identifier rename of `scripts/sync-native.sh`.

MIT license text for MicroCom:

```
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

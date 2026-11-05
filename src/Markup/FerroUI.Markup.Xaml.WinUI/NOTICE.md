# WinUI XAML importer

The WinUI semantics the importer adapts are derived from Microsoft WinUI at revision
`7e4b0c2096be402f51cc8f7d203ebf67cff183d0`:
https://github.com/microsoft/microsoft-ui-xaml

| Implementation | Reference source |
|---|---|
| `importer/migration.rs` (an x:Bind nested in an x:Bind) | `src/XamlCompiler/BuildTasks/Microsoft/Xaml/XamlCompiler/XamlDomValidator.cs` (`ProcessXBindPropertyNode`, `XamlXBindInsideXBindError`); `src/XamlCompiler/Properties/XamlCompilerResources.resx` (`XamlCompiler_XBindInsideXBind`) |

WinUI's `{x:Bind}` and its directives (`x:Phase`, `x:DefaultBindMode`, `x:Load`,
`x:DeferLoadStrategy`) are kept as written: the x:Bind core of `ferroui-markup-xaml` and the
compiler of `ferroui-markup-xaml-loader` implement them (docs/porting/x-bind.md; their NOTICE.md
files list the WinUI sources). The x:Bind property-path adapter this crate had until #525, X6
(`WinUIBindExtension`, `WinUIBindings`) is removed, and with it the WinUI event-target table that
the importer used to reject an x:Bind of an event.

## Microsoft license

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

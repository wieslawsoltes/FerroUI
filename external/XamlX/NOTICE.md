# NOTICE

The `xamlx` crate in `external/XamlX/src/XamlX` is a Rust port of the backend-independent part
of **XamlX**, a general purpose pluggable XAML compiler library
(<https://github.com/kekekeks/XamlX>), originally written in C#.

The port follows the upstream sources file by file (`Ast`, `Parsers`, `TypeSystem`, `Transform`,
`Emit`, `Compiler`, `Diagnostics`). The IL backend of the original library is not part of the port.

XamlX is distributed under the MIT License:

```
MIT License

Copyright (c) 2019 Nikita Tsukanov

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

## Markup extension scanner

The markup extension tokenizer (`parsers/system_xaml_markup_extension_parser`) derives, through
XamlX, from the System.Xaml sources, which are licensed to the .NET Foundation under one or more
agreements and are made available by the .NET Foundation under the MIT license.

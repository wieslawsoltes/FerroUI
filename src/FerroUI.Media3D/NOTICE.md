# NOTICE

## API and behaviour

The public names, the members and the behaviour of the values of this crate (`Point3D`,
`Vector3D`, `Size3D`, `Rect3D`, `Point4D`, `Matrix3D`, `Quaternion`) and their text forms follow
the `System.Windows.Media.Media3D` namespace of Windows Presentation Foundation (WPF,
<https://github.com/dotnet/wpf>), MIT License:

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

The Rust code is written for FerroUI: exceptions are `Result` errors, the overloads are methods
with suffixes, and the operators are the traits of `std::ops`. The design and the mapping are in
`docs/porting/media3d.md`. See `NOTICE-PROJECT.md` for the notices of the framework the crate is
built on.

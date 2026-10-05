# NOTICE

The types in this module (`Vector2`, `Vector3`, `Vector4`, `Quaternion`,
`Matrix3x2`, `Matrix4x4`) reproduce the behaviour of the `System.Numerics`
value types of the **.NET runtime** (<https://github.com/dotnet/runtime>, MIT
license): the member set used by the composition engine, the formulas and
their edge cases (NaN propagation, singular matrices, exact quarter-turn
rotations, text formatting). It is a Rust implementation written against that
behaviour and verified against the output of the .NET runtime; the expected
values in `tests.rs` were produced by running the same operations on .NET.

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

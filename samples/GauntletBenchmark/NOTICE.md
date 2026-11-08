# NOTICE

This sample is a port of the .NET desktop app of the framework comparison benchmark of the
Cranpose project (https://github.com/samoylenkodmitry/Cranpose, `benchmarks/compose-vs-cranpose`),
and of the shared C# data file every .NET app of that benchmark compiles.

- `perf_data.rs` is the port of the shared data file: the deterministic generator, the tiers and
  the per-frame arithmetic of the frame index, bit for bit.
- `gauntlet.rs` and `wall.rs` are the ports of the gauntlet and the still wall screens, control for
  control.
- `app.rs` and `program.rs` are the ports of the application class, its launch options and its
  desktop entry point.

The fonts the sample loads at run time (Roboto, from the folder `PERF_FONTS` names) are not part of
this sample; the benchmark ships them in its `fonts` folder under their own license.

The original code is licensed under the MIT license:

```text
MIT License

Copyright (c) 2026 Dmitry Samoylenko and Cranpose contributors

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

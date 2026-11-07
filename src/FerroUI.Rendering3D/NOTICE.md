# NOTICE

## Lighting and materials

The lighting function of this crate (`lighting.rs`: the sum of the ambient lights times the
ambient colour of the material, Lambert diffuse, the Blinn-Phong specular term, the attenuation and
range of point and spot lights and the falloff of a spot light between its cones) and the layout of
its light records are a port of `ComputeWpfLighting` and `GpuLight3DRecord` of the 3D shaders of
ProGPU (<https://github.com/wieslawsoltes/ProGPU>, `src/ProGPU.Scene/Shaders/Mesh3DSolid.wgsl`),
MIT License:

```
MIT License

Copyright (c) 2026 Wiesław Šoltés

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

## Behaviour

What is drawn follows the 3D model of Windows Presentation Foundation (WPF,
<https://github.com/dotnet/wpf>, MIT License, .NET Foundation): counterclockwise front faces,
`Material` and `BackMaterial`, the layers of a `MaterialGroup`, WPF's lights and cameras and a
transparent viewport. The rasterizer itself (homogeneous clipping, the edge functions with the
top-left rule, perspective-correct interpolation, the depth buffer and the supersampling) is
written for FerroUI. The design is in `docs/porting/media3d.md`. See `NOTICE-PROJECT.md` for the
notices of the framework the crate is built on.

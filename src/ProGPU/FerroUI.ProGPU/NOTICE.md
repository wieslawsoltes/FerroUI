# NOTICE

`ferroui-progpu` renders with the native engine of [ProGPU](https://github.com/wieslawsoltes/ProGPU)
(`v0.1.0-preview.70`), loaded at run time from its native SDK packages through the
`progpu-native-sys` bindings.

ProGPU is licensed under the MIT License, Copyright (c) 2026 Wiesław Šoltés. The SDK packages carry
the notices of the engine's optional static dependencies (FreeType, ICU) in
`runtimes/<rid>/native/licenses/`, and the wgpu-native runtime (`Silk.NET.WebGPU.Native.WGPU` 2.23.0)
is licensed under the MIT and Apache 2.0 licenses; applications that ship the runtime ship these
notices with it.

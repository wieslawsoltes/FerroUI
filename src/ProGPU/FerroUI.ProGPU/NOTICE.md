# NOTICE

`ferroui-progpu` renders with the native engine of [ProGPU](https://github.com/wieslawsoltes/ProGPU),
built from its sources (the submodule `third_party/ProGPU`) and linked statically through the
`progpu-native-sys` bindings. Nothing is loaded at run time.

ProGPU is licensed under the MIT License, Copyright (c) 2026 Wiesław Šoltés.

What an application that uses this backend ships with it:

| Part | Where it applies | License |
|---|---|---|
| The ProGPU engine and bindings | every target | MIT |
| [wgpu-native](https://github.com/FerroUI/wgpu-native) (a fork) with `wgpu-core`, `wgpu-hal` and `naga` | desktop | MIT or Apache 2.0 |
| Unicode 17 data tables compiled into the text stack of the engine | every target | Unicode License v3, `LICENSE-UNICODE` |
| Tables the engine generates from the Unicode data of [HarfBuzz](https://github.com/harfbuzz/harfbuzz) (`hb-ucd-table.hh`, `hb-script-list.h`) | every target | the "Old MIT" license of HarfBuzz, `LICENSE-HARFBUZZ` |
| The Emscripten runtime, its C and C++ standard libraries and the `emdawnwebgpu` bindings | the browser module | their notices come with the Emscripten SDK that builds the module |

The engine can also be built with FreeType (hinting) and ICU (word classification). Both need an
explicit prepared input, which the build of `progpu-native-sys` does not give, so neither is in
this backend.

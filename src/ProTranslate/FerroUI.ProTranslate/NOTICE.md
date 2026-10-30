# ProTranslate adapter attribution

This crate ports the behavior of the MIT-licensed ProTranslate Avalonia adapter
from [wieslawsoltes/ProTranslate](https://github.com/wieslawsoltes/ProTranslate),
commit `b088669a22c8979d54dd81098991f8d16f880156`.

Copyright (c) Wiesław Šoltés and ProTranslate contributors. The upstream MIT
license is included with the port's source attribution in its core crate.

The Rust implementation uses FerroUI's native binding expressions, property
inheritance, markup metadata and weak object handles. It does not embed .NET or
call a managed adapter.

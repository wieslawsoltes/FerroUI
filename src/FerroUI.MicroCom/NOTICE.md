# NOTICE

The design of this runtime (COM-style proxies for native objects and
reference-counted callable wrappers for objects implemented on the managed
side) follows the runtime of **MicroCom**
(<https://github.com/kekekeks/MicroCom>, Copyright 2021 © Nikita Tsukanov, MIT
license) and the callback base class the Avalonia project
(<https://github.com/AvaloniaUI/Avalonia>, MIT license) builds on top of it.
It is an independent Rust implementation; no source code of either project is
included. See `src/tools/MicroCom.CodeGenerator/NOTICE.md` and
`native/FerroUI.Native/NOTICE.md` for the license texts.

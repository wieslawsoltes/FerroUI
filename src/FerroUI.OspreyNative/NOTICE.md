# NOTICE

`ferroui-osprey-native` is a host for applications written with Osprey Native, the application
framework of the Osprey language. It implements the protocol those applications speak (the
envelope, the node kinds, the scene primitives, the events and the capabilities) and follows the
design of Osprey's GPUI host, from their published specifications and sources at the release
v0.29.0 (commit `19ab5d4f231fcd20b016704606d9527645aa288f`):

- `docs/specs/platforms/0046-GpuiDesktopHost.md` and `docs/specs/applications/0047-OspreyNativeHosts.md`;
- `src/osprey/packages/native` (`@osprey/native`: the node, encoder, message and capability types);
- `src/toolchain/hosts/gpui` (the GPUI host).

The code is written for FerroUI; no Osprey source file is copied into this crate. Names of the
protocol (node kinds, members, event and capability kinds) are the protocol's own.

## Osprey

- Project: <https://github.com/Nimblesite/osprey>
- License: MIT

```
Released under MIT License

Copyright (c) 2025-2026 NIMBLESITE PTY LTD, Australia

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

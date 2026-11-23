# Third-party notices

The Direct2D render backend is an addition of FerroUI: it has no counterpart in
the upstream project. It draws, shapes text and decodes images with the system
components of Windows (Direct3D 11, Direct2D, DirectComposition, DXGI,
DirectWrite and the Windows Imaging Component), which it links from the
operating system. Its effect kernels are compiled at build time by the `fxc.exe`
of the Windows SDK. No source of another project is copied into the crate.

## Crates the backend links

The optional `runtime-shaders` feature links `naga` from crates.io at the
version the workspace manifest names, under the terms of both the Apache
License, Version 2.0 and the MIT license, at the choice of the user; FerroUI
uses it under the MIT license. The texts of the licenses ship with the crate.
It also loads `d3dcompiler_47.dll`, a component of Windows.

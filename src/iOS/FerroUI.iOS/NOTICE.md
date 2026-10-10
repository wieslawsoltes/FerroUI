# NOTICE

This crate is ported from the `Avalonia.iOS` project of Avalonia (`src/iOS/Avalonia.iOS`, MIT; see the
`NOTICE.md` at the root of the repository). No file of the upstream project carries a licence header of
another project.

## Dependencies outside the workspace

The crate calls UIKit, Core Animation, Core Foundation and Metal through these crates, on iOS targets
only, each pinned to an exact version in the manifest (`docs/porting/ios-platform.md`, section 2):

| Crate | Version | Licence |
|---|---|---|
| `objc2` | 0.6.5 | MIT |
| `block2` | 0.6.2 | MIT |
| `objc2-foundation` | 0.3.2 | MIT |
| `objc2-core-foundation` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-quartz-core` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-metal` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-ui-kit` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |

All seven are parts of the objc2 project (https://github.com/madsmtm/objc2). On Apple targets the crate
also uses `libc` (MIT OR Apache-2.0) for one function of the C library.

Nothing of these crates is copied into this one. The declarations of `interop.rs` are written from the
system headers of Core Foundation and libdispatch, as the declarations of the upstream file are.

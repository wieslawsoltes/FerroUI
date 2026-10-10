# Third-party notices

## Avalonia

The Android platform backend is a port of `src/Android/Avalonia.Android` of
the Avalonia project (MIT, Copyright (c) AvaloniaUI OÜ; the license text is in
the `NOTICE.md` at the root of the repository): its structure, its names, its
logic and the comments that explain that logic are derived from it, file by
file. The mapping of the files is in `docs/porting/android-platform.md`.

The Java sources under `java/org/ferroui/android` are part of the port: each
class stands for a class of the upstream project that derives from a class of
the Android framework (the application, the activities, the view, the surface
view, the handler and runnables of the dispatcher), and forwards to the Rust
code that holds the logic of that class.

## The Android framework and the NDK

The Java sources are compiled against `android.jar` of the Android SDK and the
native code is linked against the libraries of the Android NDK (`libandroid`,
`liblog`, `libEGL`, `libGLESv2`, `libc`, `libm`, `libdl`). Neither is
distributed with the crate: an application is built with the SDK and the NDK
of whoever builds it, under their licence terms (the Android Software
Development Kit License Agreement), and runs against the libraries of the
device. The declarations of the NDK functions the backend calls
(`interop/ndk.rs`) are written from the documentation of the NDK, as the
upstream project declares them in its own source.

## Dependencies

| Crate | Version | Licence | Used for |
|---|---|---|---|
| `jni-sys` | 0.4.1 (exact) | MIT OR Apache-2.0 | The declarations of the Java Native Interface (`jni.h`), which `interop/java.rs` wraps in safe functions. Android only. |
| `libc` | 0.2 | MIT OR Apache-2.0 | `dlopen` and `dlsym`, for the EGL library of the system and for one function of the NDK that is newer than the API level the library is linked against. Android only. |

`jni-sys` depends on `jni-sys-macros` 0.4.1 (MIT OR Apache-2.0), a procedural
macro that depends on `quote` and `syn` (both MIT OR Apache-2.0) at build
time.

The renderer and the text shaper an application gets with `use_android`
(`ferroui-skia` with Skia, `ferroui-harfbuzz` with HarfBuzz) carry their own
notices.

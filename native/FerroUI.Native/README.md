# FerroUI.Native

Objective-C++ implementation of the macOS native platform backend. It exposes
COM-style C++ interfaces (reference counted, `IUnknown`-based) that are
described in `src/FerroUI.Native/frn.idl`.

* `inc/` — hand-written support headers (`com.h`, `comimpl.h`, ...).
  The interface header `ferro-native.h` is **not** checked in: it is generated
  from the IDL by `microcom-codegen` at build time.
* `src/OSX/` — the backend sources. The entry point is
  `extern "C" IFerroNativeFactory* CreateFerroNative()` in `main.mm`.

The library is built by `src/FerroUI.Native/build.rs` (crate `ferroui-native`)
with the `cc` crate: ARC enabled for everything except `noarc.mm`,
`-std=gnu++0x`, libc++, linked statically into the Rust crate.

These files are imported, not edited by hand: see `NOTICE.md` for origin and
license, `UPSTREAM_REVISION` for the imported revision, and
`scripts/sync-native.sh` for how to refresh them. Local changes, should any
become necessary, belong in `patches/*.patch` (applied by the sync script).

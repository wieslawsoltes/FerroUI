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

## COM references

The header is generated in the smart pointer wrapper mode of MicroCom 0.11.7
(`CppOptions::smart_pointer_wrappers` of `microcom-codegen`, upstream dc0105179b).
Every interface method has a raw `impl_*` vtable slot and a
non-virtual wrapper that takes and returns `ComPtr`. An implementation derives from
`MicroComImpl<IFoo>` (or `ComSingleObject<IFoo, &IID_IFoo>`), never from the raw
interface, and overrides the `ComPtr` shaped methods:

| IDL | Override |
|---|---|
| `IFoo* GetFoo()` | `ComPtr<IFoo> GetFoo()` |
| `HRESULT GetFoo(IFoo** ppv)` | `HRESULT GetFoo(ComPtr<IFoo>& ppv)` |
| `HRESULT SetFoo(IFoo* foo)` | `HRESULT SetFoo(const ComPtr<IFoo>& foo)` |
| no interface types | unchanged |

The generated `impl_*` thunk runs the `@cpp-call-preamble` of the IDL (an autorelease
pool) and keeps `this` and every interface argument alive for the call, so the sources
have no call prologue of their own. In a `cpp-virtual-inherits` chain (`IFrnTopLevel`,
`IFrnWindowBase`, `IFrnWindow` and `IFrnPopup`) every class inherits the adapters with
`virtual`; `INTERFACE_MAP_ENTRY` names the interface, not the adapter.

The wrappers of `inc/comimpl.h`: `ComPtr<T>` owns a reference and has no raw pointer
conversions (`comnew<T>(...)` and `comself(this)` make one, `getRetainedReference()`
hands an owned reference to a raw boundary, `dynamicCast<T>()` finds the implementation
behind an interface and gives null for an object implemented in Rust); `ComObjectWeakPtr<T>`
is a weak reference whose `tryGet()` never revives an object in destruction;
`ComStaticPtr<T>` holds a process-wide reference that is not released at exit. Reference
counts are atomic: the Rust side releases objects from other threads.

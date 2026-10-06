// The script-based setjmp and longjmp of Skia's prebuilt archive, in a module that unwinds with
// WebAssembly exceptions.
//
// The archive of skia-bindings (libskia.a) was compiled with Emscripten's script-based setjmp and
// longjmp (`-sSUPPORT_LONGJMP=emscripten`, the default without `-fwasm-exceptions`); libjpeg-turbo,
// libpng and FreeType use them for errors. In a function that calls `setjmp`, every call that may
// jump is made through an imported `invoke_<signature>(index, ...)`, and `longjmp` is a call of
// `emscripten_longjmp`. `emscripten_longjmp` records the target in `__THREW__` and `__threwValue`
// (`setThrew`) and throws; the innermost `invoke_*` catches, restores the stack pointer and returns
// to its caller, which reads `__THREW__` and either returns from its `setjmp` or passes the jump on
// with `emscripten_longjmp` (LLVM's WebAssemblyLowerEmscriptenEHSjLj).
//
// Rust 1.93 and later unwind with WebAssembly exceptions on wasm32-unknown-emscripten and link with
// `-fwasm-exceptions`. Emscripten then uses WebAssembly setjmp and longjmp: it refuses
// `-sSUPPORT_LONGJMP=emscripten` with `-fwasm-exceptions`, builds its compiler runtime without
// `emscripten_longjmp` and generates no `invoke_*` functions, so the archive does not link as it is.
// Skia is not built from source, so this file provides both parts of the protocol inside the module,
// with the behaviour of Emscripten 6.0.10's own (`invoke_*` of tools/js_manipulation.py,
// `emscripten_longjmp` of system/lib/compiler-rt/emscripten_setjmp.c): `emscripten_longjmp` records
// the target and throws, and the innermost `invoke_*` catches the jump and returns as the script
// version does. Only the jump is caught: Rust panics and C++ exceptions pass through.
//
// A host-level difference, recorded in docs/porting/browser-platform.md, section 14: remove this file,
// its build in build.rs and the `cc` build dependency once rust-skia publishes binaries for the target
// compiled with `-sSUPPORT_LONGJMP=wasm`.
//
// The stack pointer: the script version restores it with `stackRestore` in its catch. Here the catch
// of `invoke` does the same, because LLVM saves `__stack_pointer` on entry to a function with a catch
// and writes it back at the catch (see the disassembly of `invoke_vi`).
//
// Compiled for wasm32-unknown-emscripten only, with -fwasm-exceptions (build.rs). The signatures
// are the ones the archive imports (`llvm-nm -u libskia.a | grep invoke_`); a signature that a new
// Skia version adds fails the link ("invoke_ functions exported but exceptions and longjmp are both
// disabled").

#include <stdint.h>

extern "C" {
// Emscripten's compiler runtime (system/lib/compiler-rt/emscripten_exception_builtins.c): records
// the target of a jump in `__THREW__` and `__threwValue`, unless one is already recorded.
void setThrew(uintptr_t threw, int value);
}

namespace {

// The exception of a jump; the target is in `__THREW__`.
struct LongJump {};

// Calls the function at `index` of the function table, and returns to the caller when it jumps.
template <typename Result, typename... Arguments>
Result invoke(intptr_t index, Arguments... arguments) {
  try {
    return reinterpret_cast<Result (*)(Arguments...)>(index)(arguments...);
  } catch (const LongJump&) {
    setThrew(1, 0);
    return Result();
  }
}

}  // namespace

extern "C" {

[[noreturn]] void emscripten_longjmp(uintptr_t env, int value) {
  setThrew(env, value == 0 ? 1 : value);
  throw LongJump();
}

void invoke_v(intptr_t index) { invoke<void>(index); }
void invoke_vi(intptr_t index, int a1) { invoke<void>(index, a1); }
void invoke_vii(intptr_t index, int a1, int a2) { invoke<void>(index, a1, a2); }
void invoke_viii(intptr_t index, int a1, int a2, int a3) { invoke<void>(index, a1, a2, a3); }
void invoke_viiii(intptr_t index, int a1, int a2, int a3, int a4) { invoke<void>(index, a1, a2, a3, a4); }
void invoke_viiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5) {
  invoke<void>(index, a1, a2, a3, a4, a5);
}
void invoke_viiiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5, int a6) {
  invoke<void>(index, a1, a2, a3, a4, a5, a6);
}
void invoke_viiiiiiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5, int a6, int a7, int a8,
                       int a9) {
  invoke<void>(index, a1, a2, a3, a4, a5, a6, a7, a8, a9);
}
int invoke_ii(intptr_t index, int a1) { return invoke<int>(index, a1); }
int invoke_iii(intptr_t index, int a1, int a2) { return invoke<int>(index, a1, a2); }
int invoke_iiii(intptr_t index, int a1, int a2, int a3) { return invoke<int>(index, a1, a2, a3); }
int invoke_iiiii(intptr_t index, int a1, int a2, int a3, int a4) {
  return invoke<int>(index, a1, a2, a3, a4);
}
int invoke_iiiiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5, int a6) {
  return invoke<int>(index, a1, a2, a3, a4, a5, a6);
}
int invoke_iiiiiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5, int a6, int a7) {
  return invoke<int>(index, a1, a2, a3, a4, a5, a6, a7);
}
int invoke_iiiiiiiiii(intptr_t index, int a1, int a2, int a3, int a4, int a5, int a6, int a7, int a8,
                      int a9) {
  return invoke<int>(index, a1, a2, a3, a4, a5, a6, a7, a8, a9);
}

}  // extern "C"

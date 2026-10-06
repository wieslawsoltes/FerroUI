// The function table lookup of the Emscripten runtime, with the cache that the runtime leaves out of
// builds optimised for size (-Os, -Oz).
//
// Rust code on the Emscripten target unwinds with JavaScript exceptions: every call that may unwind
// goes through an `invoke_*` function of the runtime, which looks the callee up in the function table
// with `getWasmTableEntry`. Without the cache that lookup is a `WebAssembly.Table.get` per call, and in
// a size-optimised module, where fewer calls are inlined, it was most of the start-up time of the
// application. Linked with `--js-library` (see .cargo/config.toml).
//
// Written against the runtime of Emscripten 6.0.10, the version the browser toolchain pins
// (scripts/browser/setup.sh): the cache is the one of the runtime's own optimised-for-speed builds
// (`wasmTableMirror`, `$getWasmTableEntry` and `$setWasmTableEntry` of src/lib/libcore.js). In that
// runtime the table entries are only written through `setWasmTableEntry`, which this file replaces
// as well and which updates the cache; the table only grows (dynamic linking, not used here), which
// appends entries the cache has not seen. Check both again when the pinned version changes.
//
// Builds that are not optimised for size (the dev profile links at -O0) keep the runtime's own cache,
// with its assertion that the cache is up to date.
#if SHRINK_LEVEL > 0
addToLibrary({
  $ferrouiWasmTableMirror: [],

  $setWasmTableEntry__internal: true,
  $setWasmTableEntry__deps: ['$ferrouiWasmTableMirror', '$wasmTable'],
  $setWasmTableEntry: (idx, func) => {
    wasmTable.set(idx, func);
    // Read back, as the runtime does: the table may hold a wrapper of `func`.
    ferrouiWasmTableMirror[idx] = wasmTable.get(idx);
  },

  $getWasmTableEntry__internal: true,
  $getWasmTableEntry__deps: ['$ferrouiWasmTableMirror', '$wasmTable'],
  $getWasmTableEntry: (funcPtr) => {
    var func = ferrouiWasmTableMirror[funcPtr];
    if (!func) {
      ferrouiWasmTableMirror[funcPtr] = func = wasmTable.get(funcPtr);
    }
    return func;
  },
});
#endif

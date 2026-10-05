// The function table lookup of the Emscripten runtime, with the cache that the runtime leaves out of
// builds optimised for size (-Os, -Oz).
//
// Rust code on the Emscripten target unwinds with JavaScript exceptions: every call that may unwind
// goes through an `invoke_*` function of the runtime, which looks the callee up in the function table
// with `getWasmTableEntry`. Without the cache that lookup is a `WebAssembly.Table.get` per call, and in
// a size-optimised module, where fewer calls are inlined, it was most of the start-up time of the
// application. The cache is the one of the runtime's own optimised-for-speed builds (`wasmTableMirror`
// of src/lib/libcore.js): the table is only written through `setWasmTableEntry`, which updates it.
// Linked with `--js-library` (see .cargo/config.toml).
addToLibrary({
  $ferrouiWasmTableMirror__internal: true,
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

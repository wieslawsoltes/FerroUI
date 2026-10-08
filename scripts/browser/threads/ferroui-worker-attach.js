// Linked into the script of a module built with threads (`scripts/build-browser.sh --threads`,
// emcc's --post-js): the end of the factory function of the module. It only acts in a web worker
// that runs a thread (a pthread), where nobody else can do the things below; in the page it
// does nothing, and the host page attaches the module as before. Everything here runs while the
// script of the module is evaluated in the worker, before the worker takes its first message, so
// it is in place before the start function of any thread runs.
//
// It uses names of the script emcc generates: `ENVIRONMENT_IS_PTHREAD`, `Module`, `PThread`,
// `wasmMemory` and `proxyToMainThread` (Emscripten), `___wbindgen_start` (the receiving name of an
// export the wasm-bindgen tool adds) and `__ferroui_FerroExports` (ferroui-worker-import.js). Read
// in Emscripten 6.0.10 and wasm-bindgen 0.2.129; see docs/porting/browser-render-worker.md, "B2.1",
// "B2.2" and "B2.6".
if (ENVIRONMENT_IS_PTHREAD) {
  // 0. The memory of the module is exported as `Module.wasmMemory`, and ferroui.js makes its views
  // of the memory from it (`FerroExports.heapU8`). The export is an assignment made while this
  // script is evaluated, and in a worker the memory arrives later, with the message that loads the
  // WebAssembly module: the property would stay undefined. It becomes a property that reads the
  // variable at each use.
  Object.defineProperty(Module, 'wasmMemory', { get: () => wasmMemory, enumerable: true, configurable: true });

  // 1. The script side of the platform finds the module through `FerroExports.attach`, which the
  // host page calls for the module of the page. The worker has its own copy of ferroui.js and
  // its own `Module` (its own `GL`, its own exports, its own view of the shared memory): they are
  // attached to each other here. The properties of `Module` are filled in later, when the worker
  // is sent the WebAssembly module; ferroui.js reads them at each use.
  __ferroui_FerroExports.attach(Module);

  // 2. The wasm-bindgen glue keeps the script objects that Rust holds in a table of the
  // WebAssembly instance, and every worker has an instance of its own. The table gets its fixed
  // entries (undefined, null, true, false) from the start function of the glue, which Emscripten
  // runs with the initialisers of the runtime, and those do not run in a thread. Without it the
  // first objects handed to Rust in the worker would take the places of the fixed entries. It is
  // run once per worker, after the thread-local storage of the first thread of the worker is set
  // up and before the start function of that thread.
  // 3. A function of the C library that only the main thread can serve (a write to the console, a
  // file, the environment) is carried there by `proxyToMainThread`, and the thread waits for the
  // answer. A render thread must not depend on the thread of the page while it draws a frame, so
  // the calls of this worker are counted, with the index of the function of the last one
  // (`proxiedFunctionTable` of this script); ferroui.js hands the two numbers to the framework
  // (`FerroExports.proxiedCalls`, `FerroExports.lastProxiedFunction`). An EM_ASM block has no
  // index: it is recorded as 0x7fffffff.
  if (typeof proxyToMainThread == 'function') {
    var ferrouiProxyToMainThread = proxyToMainThread;
    Module['ferrouiProxiedCalls'] = 0;
    Module['ferrouiLastProxiedFunction'] = -1;
    proxyToMainThread = (funcIndex, emAsmAddr, ...rest) => {
      Module['ferrouiProxiedCalls']++;
      Module['ferrouiLastProxiedFunction'] = emAsmAddr ? 0x7fffffff : funcIndex;
      return ferrouiProxyToMainThread(funcIndex, emAsmAddr, ...rest);
    };
  }

  var ferrouiThreadInitTLS = PThread.threadInitTLS;
  var ferrouiBindgenStarted = false;
  PThread.threadInitTLS = () => {
    ferrouiThreadInitTLS();
    if (!ferrouiBindgenStarted) {
      ferrouiBindgenStarted = true;
      // A module without script objects in Rust has no such function.
      if (typeof ___wbindgen_start == 'function') {
        ___wbindgen_start();
      }
    }
  };
}

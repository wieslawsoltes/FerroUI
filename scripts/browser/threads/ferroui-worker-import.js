// Linked into the script of a module built with threads (`scripts/build-browser.sh --threads`,
// emcc's --extern-pre-js): one import at the top of the script, outside the factory function of
// the module, next to the imports the wasm-bindgen glue writes there.
//
// The script of the module is loaded once more in every web worker that runs a thread, and each
// worker has its own copy of ferroui.js. This import gives the code of ferroui-worker-attach.js,
// which is linked inside the factory function, the `FerroExports` of the copy of the thread it
// runs in. It is a static import, so it is resolved before the script runs: nothing has to wait
// for it. See docs/porting/browser-render-worker.md, section 3 and "B2.1".
import { FerroExports as __ferroui_FerroExports } from './ferroui.js';

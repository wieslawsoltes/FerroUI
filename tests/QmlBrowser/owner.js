import init, {probe} from "./pkg/qml_browser_tests.js";
try {
    await init();
    postMessage(await probe());
} catch (e) {
    postMessage(String(e?.stack || e));
}

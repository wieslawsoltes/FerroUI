import init, {probe, probe_fetch} from "./pkg/qml_browser_tests.js";
try {
    await init();
    await probe_fetch(new URL("./Fetched.qml", self.location.href).href);
    postMessage(await probe());
} catch (e) {
    postMessage(String(e?.stack || e));
}

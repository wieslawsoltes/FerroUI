import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";

const loaded = await loadModule({ script: "./live_inspector.js", element: document.getElementById("out") });
if (loaded) {
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    runtime.runMain();
}

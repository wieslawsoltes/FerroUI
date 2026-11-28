import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";

const element = document.getElementById("out");

try {
    const loaded = await loadModule({ script: "./ferroribbon-browser.js", element });
    if (loaded) {
        const runtime = await loaded.createRuntime();
        FerroExports.attach(runtime);
        runtime.runMain();
    }
} catch (error) {
    element.textContent = "Unable to open FerroRibbon Gallery.";
    throw error;
}

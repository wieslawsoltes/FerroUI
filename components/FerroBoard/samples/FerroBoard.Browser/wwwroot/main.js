import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";

try {
    const loaded = await loadModule({script: "./ferroboard-browser.js", element: document.getElementById("out")});
    if (!loaded) throw new Error("The browser could not start FerroBoard.");
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    runtime.runMain(new URLSearchParams(location.search).get("options") ?? "");
    document.getElementById("loading")?.remove();
    document.documentElement.dataset.ferroboard = "ready";
    const save = () => runtime.saveSession();
    document.addEventListener("visibilitychange", () => { if (document.hidden) save(); });
    window.addEventListener("pagehide", save);
} catch (error) {
    document.documentElement.dataset.ferroboard = "failed";
    const message = document.getElementById("startup-error");
    message.hidden = false;
    message.textContent = `FerroBoard could not start: ${error?.stack ?? error}`;
    console.error(error);
}

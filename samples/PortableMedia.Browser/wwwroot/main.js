import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
const loaded = await loadModule({script:"./portable-media-browser.js",element:document.getElementById("out")});
if (loaded) {
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    globalThis.portableMedia = runtime;
    runtime.runMain(globalThis.location.search);
    document.getElementById("file").addEventListener("change",async event=>{
        const file=event.target.files[0];
        if(file){runtime.openResource(new Uint8Array(await file.arrayBuffer()),file.type||"video/mp4");}
    });
    document.getElementById("camera").addEventListener("click",()=>runtime.startCamera());
    document.getElementById("photo").addEventListener("click",()=>runtime.capturePhoto());
    document.getElementById("clear").addEventListener("click",()=>runtime.clearMedia());
}

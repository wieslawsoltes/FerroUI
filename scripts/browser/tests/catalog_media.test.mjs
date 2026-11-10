// Real catalog UI, native HTML media and page-owned Skia/WebGL on a GPU.
// Build: scripts/build-browser.sh control-catalog-browser --debug
// Run: FERROUI_BROWSER_ANGLE=metal node scripts/browser/tests/catalog_media.test.mjs <site>
// Threaded site (#601): FERROUI_CATALOG_MEDIA_THREADS=1 with the site of `build-browser.sh --both`.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { open, sleep } from "../harness.mjs";
import { drive, waitUntilReady, visit, inContent } from "../catalog-pages.mjs";

const site = path.resolve(process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? "target", "browser/control-catalog-browser"));
const source = fs.readFileSync("samples/ControlCatalog/Media/sintel-sample.mp4");
assert.deepEqual(fs.readFileSync(path.join(site, "media/sintel-sample.mp4")), source);
const manifest = fs.readFileSync(path.join(site, "assets/ControlCatalog.json"), "utf8");
assert.ok(!manifest.includes(".mp4"), "video must stay out of startup and idle-prefetch manifests");
assert.ok(!fs.readFileSync(path.join(site, "control_catalog_browser.wasm")).includes(source), "video must stay out of WebAssembly");
const initScript = `(() => {
    globalThis.__catalogVideos = [];
    globalThis.__catalogCameraRequests = 0;
    const create = document.createElement;
    document.createElement = function(name, ...args) {
        const result = create.call(this, name, ...args);
        if (name.toLowerCase() === "video") __catalogVideos.push(result);
        return result;
    };
    globalThis.__holdCameraTransfers = false;
    globalThis.__heldCameraTransfers = [];
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function(message, ...rest) {
        if (message?.ferrouiCmd === "mediaFrame" && __holdCameraTransfers) {
            __heldCameraTransfers.push(() => post.call(this, message, ...rest));
        } else { post.call(this, message, ...rest); }
    };
    const acquire = navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
    navigator.mediaDevices.getUserMedia = options => {
        __catalogCameraRequests++;
        if (options.audio !== false) throw new Error("Preview requested microphone access");
        return acquire(options);
    };
})();`;
// FERROUI_CATALOG_MEDIA_THREADS=1 runs the same checks on the module with threads, its compositor on
// the render worker, where frames are transferred to the worker as WebCodecs frames (#601).
const threaded = process.env.FERROUI_CATALOG_MEDIA_THREADS === "1";
const page = drive(await open(site, {
    query: threaded
        ? "?RenderingMode=WebGL2&Threads=true&RenderThread=true"
        : "?RenderingMode=WebGL2&Threads=false&RenderThread=false",
    isolated: threaded,
    width: 1100, height: 1000, network: true,
    angle: process.env.FERROUI_BROWSER_ANGLE ?? "metal", initScript,
    chromeArgs: ["--use-fake-device-for-media-stream", "--mute-audio"],
}));
const state = () => page.evaluate("JSON.parse(controlCatalog.catalogMediaState())");
async function until(predicate, description) {
    let current;
    for (const deadline = Date.now() + 30000; Date.now() < deadline; await sleep(50)) {
        current = await state();
        if (predicate(current)) return current;
        if (page.errors.length) throw new Error(page.errors.join("\n"));
    }
    throw new Error(`${description}: ${JSON.stringify(current)}\n${page.log.slice(-8).join("\n")}`);
}
const click = async text => page.clickElement(await page.find(text, inContent));
const videoRequests = () => page.requests.filter(url => /\/media\/sintel-sample\.mp4(?:\?|$)/.test(url));
try {
    await waitUntilReady(page, 60000);
    if (threaded) {
        assert.equal(await page.evaluate("self.crossOriginIsolated"), true, "the threaded site is not cross-origin isolated");
    }
    // Leave ordinary idle asset prefetch enabled: it must still exclude video.
    await sleep(1800);
    assert.equal(videoRequests().length, 0, "startup/idle prefetch downloaded video");
    await visit(page, { section: "Media & Graphics", name: "Video" }, 1000);
    await sleep(350);
    assert.equal(videoRequests().length, 0, "opening Video downloaded the clip");
    assert.equal(await page.evaluate("__catalogVideos.length"), 0);
    assert.equal(await page.evaluate("__catalogCameraRequests"), 0);
    console.log("CATALOG_VIDEO_LAZY_ASSET_PASS startup_requests=0 navigation_requests=0");

    await click("Load bundled video");
    await until(s => s?.paused && s.frame && s.imports > 0, "sample first frame");
    assert.ok(videoRequests().length > 0);
    const native = await page.evaluate(`(() => {
        const v = __catalogVideos[0];
        return { count: __catalogVideos.length, src: v.src, paused: v.paused, duration: v.duration, width: v.videoWidth };
    })()`);
    assert.equal(native.count, 1);
    assert.equal(native.paused, true, "autoplay is off");
    assert.ok(native.width > 0);
    assert.ok(Math.abs(native.duration - 12) < 0.1);
    assert.equal(new URL(native.src).origin, new URL(page.url).origin);
    assert.ok(native.src.endsWith("/media/sintel-sample.mp4"), "use native URL loading, not a fetched byte/blob copy");
    await page.clickElement(await page.element("Play transport button", e => e.name === "MediaPlayPause" && inContent(e)));
    const playing = await until(s => s?.playing && s.imports > 1, "native playback and GPU presentation");
    assert.equal(playing.previewAttempts, 0);
    assert.equal(playing.previewBytes, 0);
    console.log("CATALOG_BROWSER_VIDEO_PASS", JSON.stringify(playing));
    if (process.env.FERROUI_CATALOG_MEDIA_SCREENSHOT) {
        await sleep(1200);
        const picture = await page.send("Page.captureScreenshot", { format: "png" });
        fs.writeFileSync(process.env.FERROUI_CATALOG_MEDIA_SCREENSHOT, Buffer.from(picture.data, "base64"));
    }
    await click("Unload video");
    await until(s => s?.frames === 0 && s.reads === 0 && s.quarantine === 0, "unload completion");
    assert.equal(await page.evaluate("__catalogVideos[0].getAttribute('src')"), null);

    await visit(page, { section: "Media & Graphics", name: "Camera" }, 1000);
    await click("Discover cameras");
    await until(s => s?.status.includes("camera(s); permission"), "camera discovery");
    assert.equal(await page.evaluate("__catalogCameraRequests"), 0);
    await page.setPermission("camera", "granted");
    if (threaded) {
        await page.evaluate("__holdCameraTransfers=true");
        await click("Start camera");
        await page.waitFor("__heldCameraTransfers.length > 0");
        await sleep(250);
        assert.equal((await state()).imports, 0, "unreceived frames must not be imported");
        await click("Stop camera");
        assert.ok((await state()).frames > 0, "a pending transfer still owns its native storage");
        await page.evaluate("__holdCameraTransfers=false; __heldCameraTransfers.splice(0).forEach(deliver=>deliver())");
        await until(s => !s.cameraActive && s.frames===0 && s.quarantine===0, "cancelled transfer retirement");
    }
    const requestsBeforeStart = await page.evaluate("__catalogCameraRequests");
    await click("Start camera");
    const camera = await until(s => s?.cameraActive && s.imports > 0, "synthetic browser camera preview");
    assert.equal(await page.evaluate("__catalogCameraRequests"), requestsBeforeStart + 1);
    assert.equal(camera.previewAttempts, 0);
    assert.equal(camera.analysisAttempts, 0);
    await click("Capture photo");
    await until(s => s?.status.startsWith("Photo "), "independently decoded photo");
    await click("Stop camera");
    await until(s => !s?.cameraActive && s.frames === 0 && s.reads === 0, "camera retirement");
    console.log("CATALOG_BROWSER_CAMERA_PASS fake_device=true explicit_start=true photo=true");

    await visit(page, { section: "Media & Graphics", name: "QR codes" }, 1000);
    await click("Decode bundled QR image");
    const qr = await until(s => s?.status.startsWith("Decoded 2 QR code(s):"), "offline QR image");
    assert.ok(qr.status.includes("ferroui-native-qr-left") && qr.status.includes("ferroui-native-qr-right"));
    assert.equal(await page.evaluate("__catalogCameraRequests"), requestsBeforeStart + 1);
    console.log("CATALOG_BROWSER_QR_PASS", qr.status);
    assert.equal(page.requests.filter(url=>/\/qr-worker\.js(?:\?|$)/.test(url)).length, 0, "QR decoder must stay lazy");
    const fixture = fs.readFileSync("samples/ControlCatalog/Assets/Media/qr-multiple.png").toString("base64");
    await page.evaluate(`(async()=>{
        const image=await createImageBitmap(await (await fetch("data:image/png;base64,${fixture}")).blob());
        const canvas=document.createElement("canvas");canvas.width=640;canvas.height=480;
        const context=canvas.getContext("2d");
        const draw=()=>{context.fillStyle="white";context.fillRect(0,0,640,480);context.drawImage(image,64,112);};
        draw();const timer=setInterval(draw,33);
        const original=navigator.mediaDevices;
        Object.defineProperty(navigator,"mediaDevices",{configurable:true,value:{
            enumerateDevices:original.enumerateDevices.bind(original),
            getUserMedia:async options=>{
                if(options.audio!==false)throw new Error("QR capture requested audio");
                __catalogCameraRequests++;
                const stream=canvas.captureStream(30);const track=stream.getVideoTracks()[0];
                const settings=track.getSettings.bind(track);track.getSettings=()=>({...settings(),width:640,height:480,frameRate:30});
                const stop=track.stop.bind(track);track.stop=()=>{clearInterval(timer);image.close();stop();};
                globalThis.__qrTrack=track;return stream;
            }
        }});
    })()`);
    await click("Start scanning");
    const live = await until(s=>s?.status.startsWith("Last detected:") && s.status.includes("ferroui-native-qr-left") && s.status.includes("ferroui-native-qr-right"), "live QR analysis worker");
    assert.equal(live.previewAttempts,0);assert.ok(live.analysisAttempts>0&&live.analysisBytes>0);
    assert.ok(page.requests.some(url=>/\/qr-worker\.js(?:\?|$)/.test(url)), "live scanning must load the decoder worker");
    await click("Stop camera");
    await until(s=>!s.cameraActive&&s.frames===0&&s.reads===0&&s.quarantine===0,"QR camera retirement");
    assert.equal(await page.evaluate("__qrTrack.readyState"),"ended");
    console.log("CATALOG_BROWSER_LIVE_QR_PASS",JSON.stringify(live));
    assert.deepEqual(page.errors, []);
    console.log("CATALOG_BROWSER_MEDIA_PASS");
} finally {
    await page.close();
}

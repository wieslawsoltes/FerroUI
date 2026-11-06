// Actual page-owned Skia/WebGL2 and native browser media, not a simulated renderer.
// Build: scripts/build-browser.sh portable-media-browser --debug
// Run on macOS: FERROUI_BROWSER_ANGLE=metal node scripts/browser/tests/portable_media.test.mjs
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { open, sleep, decodePng, serve } from "../harness.mjs";

const site=path.resolve(process.argv[2]??"target/browser/portable-media-browser");
const angle=process.env.FERROUI_BROWSER_ANGLE??"metal";
const gated=`(() => {
    globalThis.__holdMediaFences=true;
    globalThis.__mediaHeartbeat=0;
    setInterval(()=>globalThis.__mediaHeartbeat++,10);
    globalThis.__cameraRequests=0;
    const acquire=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
    navigator.mediaDevices.getUserMedia=(options)=>{globalThis.__cameraRequests++;if(options.audio!==false){throw new Error("Camera requested audio");}return acquire(options);};
    const wait=WebGL2RenderingContext.prototype.clientWaitSync;
    WebGL2RenderingContext.prototype.clientWaitSync=function(sync,flags,timeout){
        if(timeout!==0){throw new Error("A media fence blocked the UI");}
        return globalThis.__holdMediaFences?this.TIMEOUT_EXPIRED:wait.call(this,sync,flags,timeout);
    };
})();`;
function fields(value){return Object.fromEntries(value.split(";").map(v=>v.split("=")));}
async function state(page){return fields(await page.evaluate('globalThis.portableMedia?.mediaProbe?.()??"phase=starting"'));}
async function until(page,predicate,label,timeout=30000){
    const end=Date.now()+timeout;let current;
    while(Date.now()<end){current=await state(page);if(predicate(current))return current;if(page.errors.length)throw new Error(page.errors.join("\n"));await sleep(50);}
    throw new Error(`${label}: ${JSON.stringify(current)}\n${page.log.slice(-10).join("\n")}`);
}
const page=await open(site,{angle,width:700,height:500,initScript:gated,chromeArgs:["--autoplay-policy=no-user-gesture-required","--use-fake-device-for-media-stream"]});
try {
    await until(page,s=>s.frames==="1"&&s.gpu_reads==="1","producer fence submitted");
    assert.equal((await state(page)).imports,"0");
    await sleep(150);
    assert.ok(await page.evaluate("__mediaHeartbeat>5"),"UI tasks must continue while completion is pending");
    await page.evaluate("__holdMediaFences=false");
    const shared=await until(page,s=>s.phase==="ready"&&Number(s.imports)>=2,"two direct consumers");
    assert.equal(shared.frames,"1");assert.equal(shared.bytes,"512");
    assert.equal(shared.copies,"0");assert.equal(shared.conversions,"0");
    assert.equal(shared.preview_attempts,"0");assert.equal(shared.preview_bytes,"0");
    await sleep(150);
    const screenshot=await page.send("Page.captureScreenshot",{format:"png"});
    const pixels=decodePng(Buffer.from(screenshot.data,"base64"));
    // Independent fixed sample geometry: two 320×160 views at margin 12.
    for(const [x,y,rgb] of [[90,85,[255,0,0]],[230,85,[0,255,0]],[90,160,[0,0,255]],[230,160,[255,255,0]],
        [90,250,[255,0,0]],[230,250,[0,255,0]],[90,320,[0,0,255]],[230,320,[255,255,0]],[20,60,[255,0,255]]]){
        assert.deepEqual(pixels.pixel(x,y),rgb,`pixel ${x},${y}`);
    }
    await page.evaluate("portableMedia.rejectCpuUpload(new ImageData(4,4))");
    await until(page,s=>s.phase==="cpu-rejected:true:1:0:0","prohibited CPU upload negative control");
    // Delay notification of real native fences after another draw. This is a
    // notification-delay lifetime test, not proof of delayed hardware execution.
    await page.evaluate("__holdMediaFences=true");await page.resize(720,520);
    await until(page,s=>Number(s.gpu_reads)>0,"consumer fence submitted");
    await page.evaluate("portableMedia.clearMedia()");await sleep(80);
    assert.equal((await state(page)).frames,"1","submitted consumer keeps storage after detach");
    await page.evaluate("__holdMediaFences=false");
    console.log("WEBGL_SHARED_PASS",await until(page,s=>s.frames==="0"&&s.gpu_reads==="0","native retirement"));

    const fixture=path.join(site,"portable-media-fixture.mp4");
    fs.copyFileSync("tests/media/fixtures/avc-aac.mp4",fixture);
    try {
        await page.evaluate('portableMedia.openVideo("./portable-media-fixture.mp4")');
        await until(page,s=>s.phase==="video-ready","native video prepared");
        assert.match(await page.evaluate("portableMedia.videoState()"),/state=Paused/);
        await page.evaluate("portableMedia.playVideo()");
        await page.waitFor('portableMedia.videoState().includes("state=Playing") && Number(/position=(\\d+)/.exec(portableMedia.videoState())[1])>250',30000);
        await until(page,s=>Number(s.copies)>0&&Number(s.imports)>2,"native video texture import");
        await page.evaluate("portableMedia.pauseVideo();portableMedia.seekVideo(750)");
        await until(page,s=>s.phase.startsWith("seek:"),"achieved seek");
        assert.match(await page.evaluate("portableMedia.videoState()"),/position=750/);
        await sleep(80);
        const videoScreenshot=await page.send("Page.captureScreenshot",{format:"png"});
        const videoPixels=decodePng(Buffer.from(videoScreenshot.data,"base64"));
        for(const [x,y,expected] of [[190,100,[255,0,0]],[510,100,[0,255,0]],[190,270,[0,0,255]],[510,270,[255,255,0]]]) {
            const actual=videoPixels.pixel(x,y);assert.ok(actual.every((v,i)=>Math.abs(v-expected[i])<=4),`native video pixel ${x},${y}: ${actual}`);
        }
        await page.evaluate("portableMedia.seekVideo(300);portableMedia.seekVideo(1100)");
        await until(page,s=>s.phase==="seek:1100","latest overlapping seek");
        assert.match(await page.evaluate("portableMedia.videoState()"),/position=1100/);
        await page.evaluate("portableMedia.stopVideo()");
        await until(page,s=>s.phase==="stop:0","stop resets zero");
        await page.waitFor('/(?:^|;)frame=0$/.test(portableMedia.videoState())',30000);
        await page.evaluate("portableMedia.clearMedia()");
        console.log("BROWSER_VIDEO_PASS",await until(page,s=>s.frames==="0"&&s.gpu_reads==="0","video retirement"));
        const encoded=[...fs.readFileSync(fixture)];
        await page.evaluate(`portableMedia.openResource(new Uint8Array(${JSON.stringify(encoded)}),"video/mp4")`);
        await until(page,s=>s.phase==="video-ready","owned encoded resource");
        await page.evaluate("portableMedia.clearMedia()");
        await until(page,s=>s.frames==="0"&&s.gpu_reads==="0","resource retirement");
    } finally {fs.unlinkSync(fixture);}

    await page.setPermission("camera","prompt");
    await page.evaluate("portableMedia.discoverCameras()");
    await until(page,s=>s.phase.startsWith("discovery:"),"non-prompting discovery");
    assert.equal(await page.evaluate("__cameraRequests"),0);
    await page.setPermission("camera","denied");
    await page.evaluate("portableMedia.startCamera()");
    await until(page,s=>s.phase.startsWith("camera-error:"),"denied camera");
    assert.equal((await state(page)).frames,"0");
    await page.setPermission("camera","granted");
    await page.evaluate("portableMedia.startCamera()");
    await until(page,s=>s.phase==="camera-ready","explicit camera start");
    await until(page,s=>Number(s.frames)>0,"camera preview");
    console.log("CAMERA_SETTINGS",await page.evaluate("portableMedia.cameraSettings()"));
    await page.evaluate("portableMedia.capturePhoto()");
    await until(page,s=>s.phase.startsWith("photo:"),"owned JPEG capture");
    const photo=await page.evaluate('(async()=>{const bytes=portableMedia.photoBytes();const image=await createImageBitmap(new Blob([bytes],{type:"image/jpeg"}));const result={length:bytes.length,signature:[...bytes.slice(0,2)],width:image.width,height:image.height};image.close();return result;})()');
    assert.deepEqual(photo.signature,[255,216]);assert.ok(photo.length>100);assert.ok(photo.width>0&&photo.height>0);
    await page.evaluate("portableMedia.clearMedia()");
    const cleaned=await until(page,s=>s.frames==="0"&&s.gpu_reads==="0","camera retirement");
    assert.equal(cleaned.preview_attempts,"0");assert.equal(cleaned.preview_bytes,"0");assert.equal(cleaned.quarantined,"0");
    assert.deepEqual(page.errors,[]);
    console.log("BROWSER_SYNTHETIC_CAMERA_PASS",photo,cleaned);
    // Hold the permission boundary, cancel, then return a real synthetic native
    // stream. Its tracks must stop even though the abandoned future is gone.
    await page.evaluate(`(()=>{const acquire=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);globalThis.__lateTracks=[];navigator.mediaDevices.getUserMedia=async options=>{await new Promise(resolve=>globalThis.__allowLateCamera=resolve);const stream=await acquire(options);__lateTracks.push(...stream.getTracks());return stream;};})()`);
    await page.evaluate("portableMedia.startCamera()");
    await page.waitFor('typeof __allowLateCamera==="function"');
    await page.evaluate("portableMedia.clearMedia();__allowLateCamera()");
    await page.waitFor('__lateTracks.length>0&&__lateTracks.every(t=>t.readyState==="ended")');
    assert.equal((await state(page)).phase,"cleared");assert.equal((await state(page)).frames,"0");
    console.log("BROWSER_LATE_CAMERA_CANCELLATION_PASS");
} finally {await page.close();}

// A known software adapter cannot close a primary GPU implementation gate.
const software=await open(site,{angle:"swiftshader",width:700,height:500});
try {
    const result=await until(software,s=>s.phase.startsWith("error:"),"software rejection");
    assert.match(result.phase,/Software graphics/);assert.equal(result.frames,"0");assert.equal(result.imports,"0");
    console.log("WEBGL_SOFTWARE_REJECTION_PASS",result);
} finally {await software.close();}

const policy=await open(site,{angle,width:700,height:500,chromeArgs:["--autoplay-policy=document-user-activation-required"],
    initScript:`(()=>{const create=Document.prototype.createElement;Document.prototype.createElement=function(...args){const value=create.apply(this,args);if(args[0]==="video"){globalThis.__nativeVideo=value;}return value;};})()`});
let cors;
const policyFixture=path.join(site,"portable-media-fixture.mp4");
try {
    await until(policy,s=>s.phase==="ready","policy page ready");
    const encoded=[...fs.readFileSync("tests/media/fixtures/avc-aac.mp4")];
    await policy.evaluate(`portableMedia.openResource(new Uint8Array(${JSON.stringify(encoded)}),"video/mp4")`);
    await until(policy,s=>s.phase==="video-ready","policy source ready");
    await policy.evaluate("portableMedia.playVideo()");
    await until(policy,s=>s.phase==="play-error:AutoplayDenied","native autoplay denial");
    assert.match(await policy.evaluate("portableMedia.videoState()"),/state=Paused/);
    await policy.send("Runtime.evaluate",{expression:"portableMedia.playVideo()",userGesture:true});
    await until(policy,s=>s.phase==="video-playing","gesture recovery");
    // Diagnostic native audio samples prove AAC output reaches the audio graph.
    // This does not claim a physical speaker/audibility acceptance test.
    const audio=await policy.send("Runtime.evaluate",{userGesture:true,awaitPromise:true,returnByValue:true,expression:`(async()=>{
        const context=new AudioContext();const source=context.createMediaElementSource(__nativeVideo);
        const analyser=context.createAnalyser();source.connect(analyser);analyser.connect(context.destination);await context.resume();
        await new Promise(resolve=>setTimeout(resolve,150));const samples=new Float32Array(2048);analyser.getFloatTimeDomainData(samples);
        const rms=Math.sqrt(samples.reduce((sum,v)=>sum+v*v,0)/samples.length);await context.close();return rms;
    })()`});
    assert.ok(audio.result?.value>0.01,`native decoded audio RMS: ${JSON.stringify(audio)}`);
    console.log("BROWSER_AUTOPLAY_AUDIO_PASS",audio.result.value);
    fs.copyFileSync("tests/media/fixtures/avc-aac.mp4",policyFixture);cors=await serve(site);
    await policy.evaluate(`portableMedia.openVideo("http://127.0.0.1:${cors.address().port}/portable-media-fixture.mp4")`);
    await until(policy,s=>s.phase.startsWith("error:"),"cross-origin media rejection");
    await policy.evaluate("portableMedia.clearMedia()");
    await until(policy,s=>s.frames==="0"&&s.gpu_reads==="0","CORS cleanup");
    console.log("BROWSER_CORS_REJECTION_PASS");
} finally {
    await policy.close();if(cors)await new Promise(resolve=>cors.close(resolve));
    if(fs.existsSync(policyFixture))fs.unlinkSync(policyFixture);
}

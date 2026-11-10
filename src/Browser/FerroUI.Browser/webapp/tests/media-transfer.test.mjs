import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";

const bundled = await build({
    entryPoints: [new URL("../modules/ferroui/rendering/sharedGlFrames.ts", import.meta.url).pathname],
    bundle: true, write: false, format: "esm", target: "es2020",
    plugins: [{ name: "platform-fixture", setup(builder) {
        builder.onResolve({filter:/^\.\.\/ferroExports$|^\.\/webGlRenderTarget$|^\.\/webRenderTargetRegistry$/}, args => ({path:args.path,namespace:"fixture"}));
        builder.onLoad({filter:/.*/,namespace:"fixture"}, args => ({contents:args.path.includes("Registry")
            ? "export const WebRenderTargetRegistry={renderWorker:()=>globalThis.__worker};"
            : args.path.includes("webGl") ? "export const WebGlRenderTarget={};" : "export const FerroExports={};",loader:"js"}));
    }}]
});
const { SharedGlFrames } = await import(`data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`);
class Video {videoWidth=16;videoHeight=16;currentTime=0;}
class Frame {
    closed=0;
    constructor() { frames.push(this); }
    close() {this.closed++;}
    clone() {return new Frame();}
}
let frames=[];
globalThis.HTMLVideoElement=Video;globalThis.VideoFrame=Frame;
class WorkerFixture extends EventTarget {
    message;
    postMessage(message) {this.message=message;}
    deliver() {const {id,frame,receipt}=this.message;SharedGlFrames.receive(id,frame,receipt);}
}
const tick=()=>new Promise(resolve=>setTimeout(resolve,15));
function start(analysis=false) {frames=[];globalThis.__worker=new WorkerFixture();return SharedGlFrames.beginTransfer(new Video(),16,16,analysis);}

test("postMessage is not receipt: a shared-memory render batch waits for the worker",async()=>{
    const transfer=start();let ready=false;transfer.ready.then(()=>{ready=true;});
    await tick();assert.equal(ready,false);
    __worker.deliver();await transfer.ready;assert.equal(ready,true);assert.equal(frames[0].closed,0);
    await SharedGlFrames.discard(transfer.id);assert.equal(frames[0].closed,1);
});
test("cancellation before delivery retains storage until the retirement receipt",async()=>{
    const transfer=start();const ready=assert.rejects(transfer.ready,/cancelled/i);
    let retired=false;const disposal=SharedGlFrames.discard(transfer.id).then(()=>{retired=true;});
    await ready;await tick();assert.equal(retired,false);assert.equal(frames[0].closed,0);
    __worker.deliver();await disposal;assert.equal(retired,true);assert.equal(frames[0].closed,1);
});
test("preview has no analysis copy; opt-in analysis retires its frozen copy with the lease",async()=>{
    let transfer=start();assert.equal(frames.length,1);__worker.deliver();await transfer.ready;await SharedGlFrames.discard(transfer.id);
    transfer=start(true);assert.equal(frames.length,2);__worker.deliver();await transfer.ready;
    assert.deepEqual(frames.map(f=>f.closed),[0,0]);await SharedGlFrames.discard(transfer.id);assert.deepEqual(frames.map(f=>f.closed),[1,1]);
});
test("worker failure rejects readiness and retirement instead of claiming storage is free",async()=>{
    const transfer=start(true);const ready=assert.rejects(transfer.ready,/worker failed/);
    __worker.dispatchEvent(new Event("error"));await ready;
    await assert.rejects(SharedGlFrames.discard(transfer.id),/worker failed/);assert.equal(frames[1].closed,1);
    frames[0].close(); // The fixture owns what an unresponsive worker may still retain.
});
test("a failed transfer tells the worker to release a frame that arrives late",async()=>{
    frames=[];const posted=[];globalThis.__worker=new WorkerFixture();__worker.postMessage=message=>{posted.push(message);};
    const transfer=SharedGlFrames.beginTransfer(new Video(),16,16,false);const ready=assert.rejects(transfer.ready,/worker failed/);
    __worker.dispatchEvent(new Event("error"));await ready;
    assert.deepEqual(posted.map(message=>message.ferrouiCmd),["mediaFrame","discardMediaFrame"]);
    assert.equal(posted[1].id,transfer.id);
    await assert.rejects(SharedGlFrames.discard(transfer.id),/worker failed/);assert.equal(frames[0].closed,0);
    // The worker handles its messages in order: the late frame, then the discard.
    SharedGlFrames.receive(posted[0].id,posted[0].frame,posted[0].receipt);
    await SharedGlFrames.discard(posted[1].id);assert.equal(frames[0].closed,1);
});
test("failed posting closes provisional images without an orphaned receipt",()=>{
    frames=[];globalThis.__worker=new WorkerFixture();__worker.postMessage=()=>{throw new Error("post failed");};
    assert.throws(()=>SharedGlFrames.beginTransfer(new Video(),16,16,true),/post failed/);
    assert.deepEqual(frames.map(f=>f.closed),[1,1]);
});

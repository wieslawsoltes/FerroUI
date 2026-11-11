import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { build } from "esbuild";
import { decodePng } from "../../../../../scripts/browser/harness.mjs";

const fixture = decodePng(await readFile(new URL("../../../../../samples/ControlCatalog/Assets/Media/qr-multiple.png", import.meta.url)));
const rgba = new Uint8ClampedArray(fixture.width * fixture.height * 4);
for (let y=0;y<fixture.height;y++) for(let x=0;x<fixture.width;x++) {
    rgba.set([...fixture.pixel(x,y),255],(y*fixture.width+x)*4);
}
let answer;
const worker = {postMessage(value) {answer=value;}};
globalThis.self=worker;
const bundle=await build({entryPoints:[new URL("../modules/qr-worker.ts",import.meta.url).pathname],bundle:true,write:false,format:"esm",target:"es2020"});
await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`);
function decode(pixels=rgba.slice(),width=fixture.width,height=fixture.height,frame) {
    answer=undefined;worker.onmessage({data:{id:17,pixels,width,height,frame}});return answer;
}
function check(result) {
    assert.equal(result.id,17);assert.equal(result.error,undefined);
    assert.deepEqual(result.results.map(v=>v.text).sort(),["ferroui-native-qr-left","ferroui-native-qr-right"]);
    for(const value of result.results) {
        assert.equal(Buffer.from(value.payload).toString("utf8"),value.text);
        assert.equal(value.corners.length,4);
        assert.ok(value.corners.flat().every(v=>Number.isFinite(v)&&v>=0&&v<=1));
    }
}
test("the browser worker decodes both independent symbols without BarcodeDetector",()=>check(decode()));
test("portrait camera frames retain payloads and normalized source geometry",()=>{
    const pixels=new Uint8ClampedArray(rgba.length);
    for(let y=0;y<fixture.height;y++)for(let x=0;x<fixture.width;x++) {
        pixels.set(rgba.subarray((y*fixture.width+x)*4,(y*fixture.width+x)*4+4),(x*fixture.height+fixture.height-y-1)*4);
    }
    check(decode(pixels,fixture.height,fixture.width));
});
test("a blank frame returns an empty result",()=>assert.deepEqual(decode(new Uint8ClampedArray(32*32*4).fill(255),32,32).results,[]));
test("invalid dimensions and pixel lengths fail without decoding",()=>{
    assert.match(decode(new Uint8ClampedArray(4),1281,1).error,/bound/);
    assert.match(decode(new Uint8ClampedArray(4),32,32).error,/Invalid QR image/);
});
test("a rejected native image is closed even on the error path",()=>{
    let closed=0;const result=decode(undefined,1281,1,{close(){closed++;}});
    assert.match(result.error,/bound/);assert.equal(closed,1);
});

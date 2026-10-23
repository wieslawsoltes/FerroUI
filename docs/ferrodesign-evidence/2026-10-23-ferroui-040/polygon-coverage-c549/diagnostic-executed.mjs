import assert from "node:assert/strict";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { chromium } from "/workspace/scratch/e99dd7d32d05/FerroUI/samples/FerroDesign/node_modules/playwright/index.mjs";
import pngjs from "/workspace/scratch/e99dd7d32d05/FerroUI/samples/FerroDesign/node_modules/pngjs/lib/png.js";
import { areaAverageAlpha, coverageConvergence } from "/tmp/ferrodesign-e99-harness-c549/tests/alpha-coverage.mjs";
const { PNG } = pngjs;
function alphaDiagnostics(png, svgAlpha) {
    const { width, height } = png;
    const native = index => png.data[index * 4 + 3];
    const partial = alpha => alpha > 0 && alpha < 255;
    const summary = {
        width, height, pixels: width * height, differingOver12: 0,
        edgeCoverageDifferences: 0, oppositeOpaqueTransparent: 0,
        threshold128Differences: 0, maxAlphaDifference: 0,
        nativePartialPixels: 0, svgPartialPixels: 0,
        nativeAlphaArea: 0, svgAlphaArea: 0, absoluteAlphaAreaDifference: 0,
        differenceBounds: null, quadrantDifferences: [0, 0, 0, 0],
        // Each pair is counted once; a centered uniform fixture is symmetric.
        nativeMirrorMax: { horizontal: 0, vertical: 0 },
        svgMirrorMax: { horizontal: 0, vertical: 0 },
        samples: [],
    };
    const sampleCounts = [0, 0, 0, 0];
    for (let index = 0; index < svgAlpha.length; index++) {
        const x = index % width, y = Math.floor(index / width);
        const a = native(index), b = svgAlpha[index], delta = Math.abs(a - b);
        summary.nativePartialPixels += Number(partial(a));
        summary.svgPartialPixels += Number(partial(b));
        summary.nativeAlphaArea += a / 255;
        summary.svgAlphaArea += b / 255;
        summary.absoluteAlphaAreaDifference += delta / 255;
        summary.maxAlphaDifference = Math.max(summary.maxAlphaDifference, delta);
        summary.threshold128Differences += Number((a >= 128) !== (b >= 128));
        if (x < width / 2) {
            const mirror = y * width + width - 1 - x;
            summary.nativeMirrorMax.horizontal = Math.max(summary.nativeMirrorMax.horizontal, Math.abs(a - native(mirror)));
            summary.svgMirrorMax.horizontal = Math.max(summary.svgMirrorMax.horizontal, Math.abs(b - svgAlpha[mirror]));
        }
        if (y < height / 2) {
            const mirror = (height - 1 - y) * width + x;
            summary.nativeMirrorMax.vertical = Math.max(summary.nativeMirrorMax.vertical, Math.abs(a - native(mirror)));
            summary.svgMirrorMax.vertical = Math.max(summary.svgMirrorMax.vertical, Math.abs(b - svgAlpha[mirror]));
        }
        if (delta <= 12) continue;
        summary.differingOver12++;
        summary.edgeCoverageDifferences += Number(partial(a) || partial(b));
        summary.oppositeOpaqueTransparent += Number((a === 0 && b === 255) || (a === 255 && b === 0));
        const quadrant = Number(x >= width / 2) + 2 * Number(y >= height / 2);
        summary.quadrantDifferences[quadrant]++;
        const bounds = summary.differenceBounds ??= { left: x, top: y, right: x, bottom: y };
        bounds.left = Math.min(bounds.left, x); bounds.top = Math.min(bounds.top, y);
        bounds.right = Math.max(bounds.right, x); bounds.bottom = Math.max(bounds.bottom, y);
        if (sampleCounts[quadrant]++ < 3) summary.samples.push({ x, y, native: a, svg: b });
    }
    for (const key of ["nativeAlphaArea", "svgAlphaArea", "absoluteAlphaAreaDifference"]) {
        summary[key] = Number(summary[key].toFixed(6));
    }
    return summary;
}

async function areaAveragedSvgAlpha(page, source, width, height, scale) {
    // Keep RGBA in the page and raster only sixteen output rows at a time.
    // Integer translations preserve the intrinsic 1:1 sampling grid. Only
    // 28000 averaged alpha values cross the protocol; the same pure averaging
    // function is unit tested. Even at 32x, a canvas/readback is at most 13 MB.
    const decoded = await page.evaluateHandle(async ({ source, width, height, scale }) => {
        if (width !== 180 || height !== 160 || ![16, 32].includes(scale)) {
            throw new Error("The corner coverage reference has a fixed bounded fixture and scales.");
        }
        const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
        if (parsed.querySelector("parsererror")) throw new Error("Exported SVG must parse.");
        const root = parsed.documentElement, viewBox = root.getAttribute("viewBox");
        if (!viewBox) throw new Error("An explicit SVG viewBox is required to preserve export coordinates.");
        const paths = [...parsed.querySelectorAll("path")].map(path => path.getAttribute("d"));
        root.setAttribute("width", String(width * scale));
        root.setAttribute("height", String(height * scale));
        const encoded = new XMLSerializer().serializeToString(parsed);
        const verified = new DOMParser().parseFromString(encoded, "image/svg+xml");
        if (verified.documentElement.getAttribute("viewBox") !== viewBox ||
            JSON.stringify([...verified.querySelectorAll("path")].map(path => path.getAttribute("d"))) !== JSON.stringify(paths)) {
            throw new Error("Changing the SVG viewport must preserve its viewBox and every path.");
        }
        const url = URL.createObjectURL(new Blob([encoded], { type: "image/svg+xml" }));
        const image = new Image();
        try {
            image.src = url;
            await image.decode();
            if (image.naturalWidth !== width * scale || image.naturalHeight !== height * scale) {
                throw new Error("SVG must decode at its declared reference resolution.");
            }
            return { image, url, width, height, scale };
        } catch (error) {
            image.removeAttribute("src");
            URL.revokeObjectURL(url);
            throw error;
        }
    }, { source, width, height, scale });
    try {
        const alpha = [];
        for (let row = 0; row < height; row += 16) {
            const rows = Math.min(16, height - row);
            const pixels = await page.evaluateHandle(({ decoded, row, rows }) => {
                const { image, width, height, scale } = decoded;
                if (!Number.isInteger(row) || !Number.isInteger(rows) || row < 0 || rows < 1 ||
                    rows > 16 || row + rows > height) throw new Error("Coverage strip exceeds its fixed bounds.");
                const canvas = document.createElement("canvas");
                try {
                    canvas.width = width * scale; canvas.height = rows * scale;
                    const context = canvas.getContext("2d");
                    if (!context) throw new Error("The independent SVG decoder requires Canvas2D.");
                    context.imageSmoothingEnabled = false;
                    context.drawImage(image, 0, -row * scale); // Intrinsic 1:1, integer translation only.
                    return context.getImageData(0, 0, canvas.width, canvas.height).data;
                } finally { canvas.width = 0; canvas.height = 0; }
            }, { decoded, row, rows });
            try { alpha.push(...await page.evaluate(areaAverageAlpha, { pixels, width, height: rows, scale })); }
            finally { await pixels.dispose(); }
        }
        return alpha;
    } finally {
        try {
            await page.evaluate(({ image, url }) => {
                image.removeAttribute("src"); URL.revokeObjectURL(url);
            }, decoded);
        } finally { await decoded.dispose(); }
    }
}


const input = "/tmp/ferrodesign-e99-browser-c549-focused-report/Software2D-polygon-corners-triangle";
const output = "/tmp/ferrodesign-e99-polygon-alpha-diagnosis";
const svgBytes = await readFile(input + ".svg"), pngBytes = await readFile(input + ".png");
const svg = svgBytes.toString("utf8"), png = PNG.sync.read(pngBytes);
assert.deepEqual([png.width,png.height],[180,160]);
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const report = {created:new Date().toISOString(), runtimeCommit:"c549f6bcafe614a6373c743f5efbf2a3cb0eccd2", input:{pngSha256:sha(pngBytes),svgSha256:sha(svgBytes)}, method:"Exact c549 corners reference helpers with only fixed fixture dimensions changed to180x160; convergence evaluated in bounded slices with unchanged4-alpha gate. Original polygon gate12-alpha/85-pixels is reported without modification.", results:[]};
const browser = await chromium.launch({headless:true,executablePath:"/tmp/ferrodesign-e99-browsers/chrome-linux64/chrome",args:["--use-gl=angle","--use-angle=swiftshader","--enable-unsafe-swiftshader","--ignore-gpu-blocklist"]});
try {
    report.browser = browser.version();
    const page = await browser.newPage({viewport:{width:800,height:600},deviceScaleFactor:1});
    report.pageErrors=[];report.consoleErrors=[];
    page.on("pageerror",e=>report.pageErrors.push(String(e)));
    page.on("console",e=>{if(e.type()==="error")report.consoleErrors.push(e.text());});
    const one = await page.evaluate(async source=>{
        const url=URL.createObjectURL(new Blob([source],{type:"image/svg+xml"}));
        const image=new Image();
        try {
            image.src=url;await image.decode();
            const canvas=document.createElement("canvas");canvas.width=image.width;canvas.height=image.height;
            const ctx=canvas.getContext("2d");ctx.drawImage(image,0,0);
            const pixels=ctx.getImageData(0,0,canvas.width,canvas.height).data;
            return Array.from({length:pixels.length/4},(_,i)=>pixels[i*4+3]);
        } finally {image.removeAttribute("src");URL.revokeObjectURL(url);}
    },svg);
    report.raw1x=alphaDiagnostics(png,one);
    for (const [label,source] of [["exported",svg],["deliberately-shifted-one-pixel",svg.replace('matrix(1 0 0 1 40 40)','matrix(1 0 0 1 41 40)')]]) {
        if(label!=="exported")assert.notEqual(source,svg);
        const coarse=await areaAveragedSvgAlpha(page,source,180,160,16);
        const fine=await areaAveragedSvgAlpha(page,source,180,160,32);
        const chunks=[];
        for(let offset=0;offset<fine.length;offset+=28000) chunks.push({offset,...coverageConvergence(coarse.slice(offset,offset+28000),fine.slice(offset,offset+28000))});
        const comparison=alphaDiagnostics(png,fine.map(Math.round));
        const row={label,scales:[16,32],convergenceChunks:chunks,converged:chunks.every(c=>c.converged),comparison,originalPixelGatePassed:comparison.differingOver12<=85};
        report.results.push(row);console.log(JSON.stringify(row));
    }
    assert.equal(report.raw1x.differingOver12,212,"Reproduce the frozen failing one-pixel SVG alpha comparison");
    assert.deepEqual(report.pageErrors,[]);assert.deepEqual(report.consoleErrors,[]);
} catch(e){report.error=e.stack||String(e);throw e;}
finally {
    try {await browser.close();report.browserClosed=true;}finally {report.finished=new Date().toISOString();await mkdir(output,{recursive:true});await writeFile(output+"/report.json",JSON.stringify(report,null,2)+"\n");}
}
console.log(JSON.stringify({output:output+"/report.json",raw1x:report.raw1x,results:report.results.map(r=>({label:r.label,converged:r.converged,differingOver12:r.comparison.differingOver12,originalPixelGatePassed:r.originalPixelGatePassed})),browserClosed:report.browserClosed}));

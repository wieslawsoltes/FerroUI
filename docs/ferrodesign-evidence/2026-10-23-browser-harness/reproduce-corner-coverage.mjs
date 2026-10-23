import assert from 'node:assert/strict';
import { chromium } from '../../../samples/FerroDesign/node_modules/playwright/index.mjs';
import { readFile, writeFile } from 'node:fs/promises';
import { PNG } from '../../../samples/FerroDesign/node_modules/pngjs/lib/png.js';
import { areaAverageAlpha, coverageConvergence } from '../../../samples/FerroDesign/tests/alpha-coverage.mjs';
const source = await readFile(new URL('./b7-software2d-smoothed-corners.svg', import.meta.url),'utf8');
const native = PNG.sync.read(await readFile(new URL('./b7-software2d-smoothed-corners.png', import.meta.url)));
const harness = await readFile(new URL('../../../samples/FerroDesign/tests/corners-browser.mjs', import.meta.url),'utf8');
const start = harness.indexOf('async function areaAveragedSvgAlpha('), end = harness.indexOf('// Author through the native inspector',start);
assert.ok(start > 0 && end > start);
const strip = new Function('areaAverageAlpha','return (' + harness.slice(start,end).trim() + ')')(areaAverageAlpha);
const browser=await chromium.launch({headless:true,executablePath:process.env.FERRODESIGN_CHROMIUM || undefined,args:['--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader','--ignore-gpu-blocklist']});
const output={browser:browser.version(),fixture:'official b7 Software2D native PNG+SVG, local baseline run',comparisons:[]},references=[];
try {
  const page=await browser.newPage({viewport:{width:1600,height:1000},deviceScaleFactor:1});
  for(const scale of [16,32]){
    const alpha=await strip(page,source,200,140,scale);
    const fullPixels=await page.evaluateHandle(async({source,scale})=>{
      const doc=new DOMParser().parseFromString(source,'image/svg+xml');
      doc.documentElement.setAttribute('width',String(200*scale));doc.documentElement.setAttribute('height',String(140*scale));
      const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(doc)],{type:'image/svg+xml'}));
      const image=new Image(),canvas=document.createElement('canvas');
      try{
        image.src=url;await image.decode();canvas.width=200*scale;canvas.height=140*scale;
        const ctx=canvas.getContext('2d');ctx.imageSmoothingEnabled=false;ctx.drawImage(image,0,0);
        return ctx.getImageData(0,0,canvas.width,canvas.height).data;
      }finally{image.removeAttribute('src');canvas.width=0;canvas.height=0;URL.revokeObjectURL(url);}
    },{source,scale});
    let full;
    try{full=await page.evaluate(areaAverageAlpha,{pixels:fullPixels,width:200,height:140,scale});}finally{await fullPixels.dispose();}
    let max=0,differingOver12=0,stripFullMax=0,stripFullPixels=0;
    for(let i=0;i<alpha.length;i++){
      const delta=Math.abs(native.data[i*4+3]-Math.round(alpha[i]));
      max=Math.max(max,delta);differingOver12+=Number(delta>12);
      const fd=Math.abs(alpha[i]-full[i]);stripFullMax=Math.max(stripFullMax,fd);stripFullPixels+=Number(fd!==0);
    }
    assert.ok(differingOver12 <= 48, 'Keep the unchanged native comparison limit');
    const comparison={scale,max,differingOver12,stripFullMax,stripFullPixels};
    if(references.length) comparison.convergence=coverageConvergence(references.at(-1),alpha);
    references.push(alpha);output.comparisons.push(comparison);console.log(JSON.stringify(comparison));
  }
  assert.equal(output.comparisons.at(-1).convergence.converged,true);
}finally{await browser.close();}
if (process.env.FERRODESIGN_COVERAGE_REPORT) await writeFile(process.env.FERRODESIGN_COVERAGE_REPORT,JSON.stringify(output,null,2) + '\n');

// Change only tkhd's orthogonal matrix; reuse checked-in encoded media samples.
// Output is evidence outside Git. Never replaces an existing file.
import fs from 'node:fs';
import path from 'node:path';
const destination=process.argv[2];
if(!destination)throw Error('Usage: node tests/media/make-windows-geometry-fixtures.mjs <new-output-directory>');
fs.mkdirSync(destination,{recursive:true});
function boxes(b,start,end){
 const out=[];
 while(start<end){
  let size=b.readUInt32BE(start),header=8;
  if(size===1){size=Number(b.readBigUInt64BE(start+8));header=16;}
  if(size===0)size=end-start;
  if(size<header||size>end-start)throw Error('Invalid fixture box');
  out.push({kind:b.toString('ascii',start+4,start+8),payload:start+header,end:start+size});start+=size;
 }
 return out;
}
function matrix(b){
 const moov=boxes(b,0,b.length).find(b=>b.kind==='moov');
 for(const track of boxes(b,moov.payload,moov.end).filter(b=>b.kind==='trak')){
  const children=boxes(b,track.payload,track.end),mdia=children.find(b=>b.kind==='mdia');
  const handler=boxes(b,mdia.payload,mdia.end).find(b=>b.kind==='hdlr');
  if(b.toString('ascii',handler.payload+8,handler.payload+12)!=='vide')continue;
  const tkhd=children.find(b=>b.kind==='tkhd');return tkhd.payload+(b[tkhd.payload]===0?40:52);
 }
 throw Error('No video track');
}
const matrices=[[1,0,0,1],[0,1,-1,0],[-1,0,0,-1],[0,-1,1,0],[-1,0,0,1],[0,1,1,0],[1,0,0,-1],[0,-1,-1,0]];
for(const crop of [false,true]){
 const source=fs.readFileSync(new URL(crop?'fixtures/avc-aac-geometry-crop.mp4':'fixtures/avc-aac.mp4',import.meta.url));
 const at=matrix(source);
 for(let orientation=0;orientation<8;orientation++){
  const b=Buffer.from(source);
  for(const [i,offset] of [0,4,12,16].entries())b.writeInt32BE(matrices[orientation][i]*65536,at+offset);
  const filename=path.join(destination,`orientation-${orientation}${crop?'-crop':''}.mp4`);
  fs.writeFileSync(filename,b,{flag:'wx'});
  console.log(filename);
 }
}

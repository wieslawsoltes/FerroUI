const fs = require('fs');
const path = require('path');
const tm = require('vscode-textmate');
const onig = require('vscode-oniguruma');
const resource = path.resolve(__dirname, '../../resources');
const grammars = new Map();
for (const dir of fs.readdirSync(resource + '/grammars')) {
  const pkg = JSON.parse(fs.readFileSync(`${resource}/grammars/${dir}/package.json`, 'utf8'));
  for (const g of pkg.contributes?.grammars ?? []) grammars.set(g.scopeName, path.resolve(resource, 'grammars', dir, g.path));
}
const cases = [
 ['csharp','source.cs',['using System;','namespace Example {','  class Foo<T> where T : class {','    /* multiline','       comment */ int x = 42;','    string s = $"answer {x + 1}";','    var raw = @"one ""quoted""','two";','  }','}']],
 ['rust','source.rust',['use std::collections::HashMap;','fn main() {','  let x: Option<&str> = Some("hello\\n");','  /* outer /* nested */','    comment */ let n = 0xffu32;','  let raw = r##"text # " quotes','  more"##;','  println!("{}", x.unwrap());','}']],
 ['python','source.python',['#!/usr/bin/env python','@decorator(x=1)','def main(value: str) -> int:','    """A multiline','    docstring."""','    return f"hello {value!r}" # comment','x = [v for v in range(10) if v % 2]']],
 ['javascript','source.js',['const re = /a(b+)\\s*/gi;','function example(x) {','  return `hello ${x + 1}!`;','}','/* multiline','comment */ let n = 123.4e-5;']],
 ['typescript','source.ts',['interface Shape<T> { value: T; }','const x: Shape<number> = { value: 42 };','export async function run(): Promise<void> {','  await Promise.resolve();','}']],
 ['json','source.json',['{','  "key": "value\\n",','  "array": [1, 2.5, true, null],','  "unicode": "😀"','}']],
 ['css','source.css',['body { margin: 25px; color: #aabbcc; }','@media screen and (max-width: 600px) {','.foo:hover { --custom: "abc"; background: url("a.png"); }','}']],
 ['html','text.html.basic',['<!DOCTYPE html>','<html><body>','<script>const x = "hello";</script>','<style>.foo { color: red; }</style>','<div class="foo">Hello &amp; goodbye</div>','</body></html>']],
 ['markdown','text.html.markdown',['# Heading','A **bold** and *italic* [link](https://example.com).','','```rust','fn main() { let x = 1; }','```','> quoted','> paragraph','','- item']],
 ['shell','source.shell',['#!/bin/bash','name="world"','echo "hello ${name}"','cat <<EOF','a $name','EOF']],
 ['yaml','source.yaml',['name: example','values: [1, true, null]','text: |','  first line','  second line','next: "value"']]
];
(async () => {
 await onig.loadWASM(fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm')).buffer);
 const registry = new tm.Registry({onigLib:Promise.resolve({createOnigScanner:patterns=>new onig.OnigScanner(patterns),createOnigString:s=>new onig.OnigString(s)}),loadGrammar:async scope=> { const file=grammars.get(scope); return file ? tm.parseRawGrammar(fs.readFileSync(file,'utf8'),file) : null; }});
 const output=[];
 for (const [name,scope,lines] of cases) {
  const grammar=await registry.loadGrammar(scope); if(!grammar)throw Error(scope);
  let state=tm.INITIAL;const expected=[];
  for(const line of lines){const result=grammar.tokenizeLine(line,state);state=result.ruleStack;
   const tokens=[];
   for(const tok of result.tokens){const t={start_index:Math.min(tok.startIndex,line.length),end_index:Math.min(tok.endIndex,line.length),scopes:tok.scopes};if(t.start_index>=t.end_index)continue;const prev=tokens.at(-1);if(prev&&prev.end_index===t.start_index&&JSON.stringify(prev.scopes)===JSON.stringify(t.scopes))prev.end_index=t.end_index;else tokens.push(t);}
   if(tokens.length===0)tokens.push({start_index:0,end_index:line.length,scopes:result.tokens[0]?.scopes??[scope]});
   expected.push({text:line,tokens,depth:state.depth-1});
  }
  output.push({name,scope,lines:expected});
 }
 const result={upstream:'vscode-textmate 9.3.2 + vscode-oniguruma 2.0.1',grammar_revision:'e0db45be30278c0f3f80302c31f498c8f8f1c4f2',cases:output};
 fs.writeFileSync(path.resolve(__dirname, '../fixtures/upstream-tokens.json'),JSON.stringify(result,null,2)+'\n');
 console.log(`${output.length} language cases, ${output.reduce((n,c)=>n+c.lines.length,0)} lines`);
})();

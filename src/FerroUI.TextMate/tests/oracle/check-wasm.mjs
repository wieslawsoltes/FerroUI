import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";

// wasm-bindgen --target experimental-nodejs-module --out-dir <output> <example.wasm>
const generatedModule = process.argv[2];
if (!generatedModule) throw new Error("Pass the generated wasm-bindgen ES module path");
const module = await import(pathToFileURL(resolve(generatedModule)).href);
module.verify_initialization_error();
console.log(await module.verify_host_contracts());
const corpus = await readFile(new URL("../fixtures/upstream-tokens.json", import.meta.url), "utf8");
console.log(await module.verify_upstream_corpus(corpus));

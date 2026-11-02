import {adjust} from "./leaf.mjs"
let calls = 0
export function next() { return adjust(++calls) }

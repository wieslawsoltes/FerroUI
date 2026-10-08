// The target features of WebAssembly objects: what each object file of an archive, each object file
// and each linked module says it was compiled with, read from its `target_features` section, and
// whether its memory is shared. The check of a threaded build (docs/porting/browser-render-worker.md,
// "Skia shim"): every object linked into a module with shared memory has to be compiled with
// atomics and bulk memory, or what C++ expects to be safe between threads is not (the reference
// counts touched by inline code, the guards of local statics, `thread_local` storage).
//
//   node scripts/browser/wasm-features.mjs [--require <feature>[,<feature>...]] [--summary] <file>...
//
// <file> is a static archive (.a), an object file or a module (.wasm). One line per object:
//   <file>[(<member>)]: <features in use, sorted> [disallows: <features>] [shared memory]
// `disallows: shared-mem` is the mark the compiler leaves in an object from which it stripped
// thread-local storage because the object was compiled without atomics.
// With --summary an archive is one line per distinct set of features, with the number of its objects.
// With --require the script exits with 1 when an object does not use every feature named, and lists
// those objects on the standard error. An object file with no `target_features` section is reported
// as such and does not fail (the compiler writes the section whenever the object uses a feature, and
// every object Emscripten compiles does). A linked module (it has a memory of its own, or imports
// one, and no `linking` section) with no such section is judged by its memory, which has to be
// shared: Emscripten strips the section from an optimised module.
import fs from "node:fs";

const argv = process.argv.slice(2);
const files = [];
let required = [];
let summary = false;
for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--require") required = (argv[++i] ?? "").split(",").filter((name) => name);
    else if (argv[i] === "--summary") summary = true;
    else if (argv[i].startsWith("--")) { console.error(`unknown option: ${argv[i]}`); process.exit(2); }
    else files.push(argv[i]);
}
if (files.length === 0) {
    console.error("usage: node scripts/browser/wasm-features.mjs [--require <feature>[,<feature>...]] [--summary] <file>...");
    process.exit(2);
}

/** Reads an unsigned LEB128 number; returns the value and the offset after it. */
function leb(bytes, offset) {
    let value = 0;
    let shift = 0;
    for (;;) {
        const byte = bytes[offset++];
        value += (byte & 0x7f) * 2 ** shift;
        shift += 7;
        if ((byte & 0x80) === 0) return [value, offset];
    }
}

function name(bytes, offset) {
    const [length, start] = leb(bytes, offset);
    return [bytes.subarray(start, start + length).toString("utf8"), start + length];
}

/** Skips the limits of a memory or a table; returns the flags and the offset after them. */
function limits(bytes, offset) {
    const flags = bytes[offset++];
    [, offset] = leb(bytes, offset);
    if (flags & 0x01) [, offset] = leb(bytes, offset);
    // A custom page size.
    if (flags & 0x08) [, offset] = leb(bytes, offset);
    return [flags, offset];
}

/**
 * What a WebAssembly binary says about itself: the features of its `target_features` section (null
 * without the section), whether a memory it defines or imports is shared, and whether it is an
 * object file (it has a `linking` section).
 */
function inspect(bytes) {
    if (bytes.length < 8 || bytes.readUInt32LE(0) !== 0x6d736100) return null;
    const result = { features: null, disallowed: [], shared: false, memory: false, object: false };
    let offset = 8;
    while (offset < bytes.length) {
        const id = bytes[offset++];
        let size;
        [size, offset] = leb(bytes, offset);
        const end = offset + size;
        if (id === 0) {
            let section;
            let at;
            [section, at] = name(bytes, offset);
            if (section === "linking") result.object = true;
            if (section === "target_features") {
                result.features = [];
                let count;
                [count, at] = leb(bytes, at);
                for (let i = 0; i < count; i++) {
                    const prefix = String.fromCharCode(bytes[at++]);
                    let feature;
                    [feature, at] = name(bytes, at);
                    if (prefix === "-") result.disallowed.push(feature);
                    else result.features.push(feature);
                }
                result.features.sort();
            }
        } else if (id === 2) {
            // Imports: only a memory import is of interest, but every entry has to be stepped over.
            let [count, at] = leb(bytes, offset);
            for (let i = 0; i < count; i++) {
                [, at] = name(bytes, at);
                [, at] = name(bytes, at);
                const kind = bytes[at++];
                if (kind === 0) [, at] = leb(bytes, at);
                else if (kind === 1) { at++; [, at] = limits(bytes, at); }
                else if (kind === 2) {
                    let flags;
                    [flags, at] = limits(bytes, at);
                    result.memory = true;
                    if (flags & 0x02) result.shared = true;
                } else if (kind === 3) at += 2;
                else if (kind === 4) { at++; [, at] = leb(bytes, at); }
                else break;
            }
        } else if (id === 5) {
            let [count, at] = leb(bytes, offset);
            for (let i = 0; i < count; i++) {
                let flags;
                [flags, at] = limits(bytes, at);
                result.memory = true;
                if (flags & 0x02) result.shared = true;
            }
        }
        offset = end;
    }
    return result;
}

/** The members of a static archive (the System V and the BSD variants), without its symbol tables. */
function members(bytes) {
    const found = [];
    let longNames = null;
    let offset = 8;
    while (offset + 60 <= bytes.length) {
        let member = bytes.subarray(offset, offset + 16).toString("latin1").trimEnd();
        const size = Number.parseInt(bytes.subarray(offset + 48, offset + 58).toString("latin1"), 10);
        let start = offset + 60;
        let length = size;
        offset = start + size + (size & 1);
        if (member === "/" || member === "/SYM64/") continue;
        if (member === "//") { longNames = bytes.subarray(start, start + length).toString("latin1"); continue; }
        if (member.startsWith("#1/")) {
            const nameLength = Number.parseInt(member.slice(3), 10);
            member = bytes.subarray(start, start + nameLength).toString("latin1").replace(/\0+$/, "");
            start += nameLength;
            length -= nameLength;
        } else if (/^\/\d+$/.test(member) && longNames !== null) {
            const from = Number.parseInt(member.slice(1), 10);
            member = longNames.slice(from, longNames.indexOf("\n", from)).replace(/\/$/, "");
        } else member = member.replace(/\/$/, "");
        if (member.startsWith("__.SYMDEF")) continue;
        found.push([member, bytes.subarray(start, start + length)]);
    }
    return found;
}

function describe(info) {
    const parts = [info.features === null ? "no target_features section" : (info.features.join(" ") || "(none)")];
    if (info.disallowed.length > 0) parts.push(`[disallows: ${info.disallowed.sort().join(" ")}]`);
    // The memory an object file imports is a placeholder of the linker and says nothing.
    if (info.memory && !info.object) parts.push(info.shared ? "[shared memory]" : "[memory not shared]");
    return parts.join(" ");
}

/** What the object lacks of the required features; a linked module is judged by its memory. */
function lacking(info) {
    if (required.length === 0) return [];
    if (info.features === null) {
        if (!info.object && info.memory && !info.shared) return ["a shared memory"];
        return [];
    }
    return required.filter((feature) => !info.features.includes(feature));
}

const failures = [];
for (const file of files) {
    const bytes = fs.readFileSync(file);
    if (bytes.subarray(0, 8).toString("latin1") === "!<arch>\n") {
        const sets = new Map();
        let objects = 0;
        for (const [member, content] of members(bytes)) {
            const info = inspect(content);
            if (info === null) continue;
            objects++;
            const text = describe(info);
            sets.set(text, (sets.get(text) ?? 0) + 1);
            if (!summary) console.log(`${file}(${member}): ${text}`);
            const missing = lacking(info);
            if (missing.length > 0) failures.push(`${file}(${member}): lacks ${missing.join(", ")}`);
        }
        if (summary) for (const [text, count] of sets) console.log(`${file}: ${count} of ${objects} objects: ${text}`);
        if (objects === 0) console.log(`${file}: no WebAssembly objects`);
    } else {
        const info = inspect(bytes);
        if (info === null) { console.error(`${file}: neither an archive nor a WebAssembly binary`); process.exit(2); }
        console.log(`${file}: ${describe(info)}`);
        const missing = lacking(info);
        if (missing.length > 0) failures.push(`${file}: lacks ${missing.join(", ")}`);
    }
}
if (failures.length > 0) {
    for (const failure of failures) console.error(failure);
    process.exit(1);
}

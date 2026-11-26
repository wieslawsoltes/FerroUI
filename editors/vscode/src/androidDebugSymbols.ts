import { open, stat } from 'node:fs/promises';
import { basename, posix } from 'node:path';
import type { AndroidBridge, AndroidReceipt } from './androidBridge';
import { elfIdentity } from './elfIdentity';

interface LibrarySymbols { file: string; slide: string }

export async function androidDebugSymbols(bridge: AndroidBridge, packageId: string, receipt: AndroidReceipt,
    files: readonly string[], signal: AbortSignal): Promise<LibrarySymbols[]> {
    if (!files.length) return [];
    if (!receipt.debuggerPath) throw new Error('Install a debuggable APK that includes the FerroUI LLDB server.');
    if (!await bridge.matches(packageId, receipt, signal)) throw new Error('The Android app process changed before its symbols could be loaded.');
    const maps = await bridge.command(['exec-out', 'run-as', packageId, 'cat', `/proc/${receipt.pid}/maps`],
        signal, 6_000, 4 * 1024 * 1024);
    const bases = new Map<string, bigint>();
    for (const line of maps.split('\n')) {
        const mapping = /^([a-f0-9]+)-[a-f0-9]+\s+r\S{3}\s+0+\s+\S+\s+[1-9]\d*\s+(\/\S+)$/.exec(line.trim());
        if (mapping) bases.set(mapping[2], BigInt(`0x${mapping[1]}`));
    }
    const directory = posix.dirname(receipt.debuggerPath);
    const symbols: LibrarySymbols[] = [];
    for (const file of files) {
        const name = basename(file);
        const installed = `${directory}/${name}`;
        const base = bases.get(installed);
        if (base === undefined) throw new Error(`${name} is not loaded by the selected Android process. Check the app profile's symbolFiles.`);
        const [local, remote] = await Promise.all([
            localIdentity(file, signal),
            elfIdentity((offset, length) => bridge.commandBytes(['exec-out', 'run-as', packageId, 'dd',
                `if=${installed}`, 'bs=1', `skip=${offset}`, `count=${length}`, 'status=none'], signal, 6_000, length + 1)),
        ]);
        if (!local || !remote || local.elfClass !== remote.elfClass || local.machine !== remote.machine
            || local.buildId !== remote.buildId || local.virtualBase !== remote.virtualBase || base < remote.virtualBase) {
            throw new Error(`${name} does not match the library in the installed APK. Use symbolFiles from the same debuggable build and ABI.`);
        }
        symbols.push({ file, slide: `0x${(base - remote.virtualBase).toString(16)}` });
    }
    return symbols;
}

async function localIdentity(path: string, signal: AbortSignal) {
    if (!(await stat(path).catch(() => undefined))?.isFile()) throw new Error(`Open the unstripped Android library at ${path}.`);
    const file = await open(path, 'r');
    try {
        return await elfIdentity(async (offset, length) => {
            signal.throwIfAborted();
            const buffer = Buffer.alloc(length);
            const { bytesRead } = await file.read(buffer, 0, length, offset);
            return buffer.subarray(0, bytesRead);
        });
    } finally { await file.close(); }
}

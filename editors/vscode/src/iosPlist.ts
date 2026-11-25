import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);

export async function appleTool(executable: string, args: string[], signal: AbortSignal, timeout = 5_000): Promise<string> {
    signal.throwIfAborted();
    return (await execute(executable, args, { timeout, signal, maxBuffer: 1024 * 1024 })).stdout.trim();
}

export function plistValue(file: string, key: string, format: 'raw' | 'json', signal: AbortSignal): Promise<string> {
    return appleTool('/usr/bin/plutil', ['-extract', key, format, '-o', '-', file], signal);
}

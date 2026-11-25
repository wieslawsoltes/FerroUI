import { posix } from 'node:path';
import { fileURLToPath } from 'node:url';

export function installedDeviceBundle(value: unknown, bundleId: string): string | undefined {
    const apps = (value as { apps?: unknown } | null)?.apps;
    if (!Array.isArray(apps)) throw new Error('Xcode did not return the installed iOS apps.');
    const matching = apps.filter((app: { bundleIdentifier?: unknown } | null) => app?.bundleIdentifier === bundleId);
    if (!matching.length) return;
    const path = devicePath(matching[0].url);
    if (matching.length !== 1 || !path || posix.extname(path) !== '.app') {
        throw new Error('Xcode could not identify the installed iOS app bundle.');
    }
    return path;
}

export function deviceAppProcesses(value: unknown, bundle: string): number[] {
    const processes = (value as { runningProcesses?: unknown } | null)?.runningProcesses;
    if (!Array.isArray(processes)) throw new Error('Xcode did not return the running iOS processes.');
    return processes.flatMap((item: { executable?: unknown; processIdentifier?: unknown } | null) => {
        const executable = devicePath(item?.executable);
        const pid = item?.processIdentifier;
        return executable && posix.dirname(executable) === bundle && typeof pid === 'number' && Number.isSafeInteger(pid) && pid > 0
            ? [pid] : [];
    });
}

function devicePath(value: unknown): string | undefined {
    if (typeof value !== 'string' || !value.startsWith('file:') || value.length > 4096) return;
    try {
        const path = posix.normalize(fileURLToPath(value, { windows: false })).replace(/\/$/, '');
        if (!posix.isAbsolute(path) || path.includes('\0')) return;
        return path.startsWith('/var/') ? '/private' + path : path;
    } catch { return; }
}

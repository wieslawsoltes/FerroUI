import { spawn, type ChildProcess } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { constants } from 'node:fs';
import { copyFile, unlink } from 'node:fs/promises';
import * as path from 'node:path';
import { createInterface } from 'node:readline';
import { setTimeout as delay } from 'node:timers/promises';
import type { CargoTarget } from './cargoTarget';
import type { ProcessOutput } from './reloadLog';
import { parseFrameReceipt, positiveId, reloadFramePrefix, type RuntimeFrameReceipt } from './reloadFrames';
import { mcpEndpointPrefix, parseMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';

const eventPrefix = 'FERROUI_RELOAD_V1=';

export interface RuntimeEvent {
    language: 'xaml' | 'rsx';
    status: 'ready' | 'applied' | 'failed' | 'rebuild';
    file?: string;
    revision?: string;
    message?: string;
    frame?: number;
    acknowledgedAt: number;
}

export function revision(bytes: Buffer): string {
    let hash = 0xcbf29ce484222325n;
    for (const byte of bytes) hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
    return hash.toString(16).padStart(16, '0');
}

export class ReloadProcess {
    readonly closed: Promise<number | null>;
    private exited = false;
    private stopping = false;
    private escalation?: ReturnType<typeof setTimeout>;

    constructor(private readonly child: ChildProcess, cleanup?: () => Promise<void>) {
        const closed = new Promise<number | null>((resolve, reject) => {
            let failure: Error | undefined;
            child.on('error', error => { failure ??= error; });
            child.once('close', code => {
                this.exited = true;
                clearTimeout(this.escalation);
                if (failure) reject(failure);
                else resolve(code);
            });
        });
        this.closed = cleanup ? closed.finally(cleanup) : closed;
    }

    get pid(): number | undefined { return this.child.pid; }
    get hasExited(): boolean { return this.exited || this.child.exitCode !== null || this.child.signalCode !== null; }

    stop(): Promise<void> {
        const stopped = this.closed.then(() => undefined, () => undefined);
        const pid = this.pid;
        if (this.exited || this.stopping || !pid) return stopped;
        this.stopping = true;
        if (process.platform === 'win32') {
            const shutdown = spawn('taskkill.exe', ['/pid', String(pid), '/t', '/f'], { stdio: 'ignore', windowsHide: true });
            shutdown.on('error', () => this.child.kill());
            shutdown.on('exit', code => { if (code) this.child.kill(); });
            return stopped;
        }
        this.signalGroup(pid, 'SIGTERM');
        this.escalation = setTimeout(() => this.signalGroup(pid, 'SIGKILL'), 1500);
        this.escalation.unref();
        return stopped;
    }

    private signalGroup(pid: number, signal: NodeJS.Signals): void {
        try { process.kill(-pid, signal); }
        catch { this.child.kill(signal); }
    }
}

export interface ProjectMetadata { manifest: string; directories: string[]; package?: string; features?: string[] }

export function buildCargo(target: CargoTarget, output: ProcessOutput): {
    process: ReloadProcess; artifact: Promise<string>; metadata(): ProjectMetadata; localManifests(): string[];
} {
    const args = ['build', '--manifest-path', target.manifest, '--package', target.packageName,
        '--bin', target.binary, '--message-format=json-render-diagnostics'];
    if (target.features.length) args.push('--features', target.features.join(','));
    const child = spawn(target.cargo, args, { ...options(target), cwd: target.cargoDirectory });
    const proc = new ReloadProcess(child);
    let artifact: string | undefined;
    let features: string[] | undefined;
    const directories = new Set<string>();
    const manifests = new Set<string>();
    lines(child.stdout, line => {
        try {
            const message = JSON.parse(line);
            if (message.reason === 'compiler-artifact' && typeof message.manifest_path === 'string' && path.isAbsolute(message.manifest_path)) {
                manifests.add(message.manifest_path);
            }
            if (message.reason === 'compiler-artifact' && message.package_id === target.packageId
                && message.target?.name === target.binary && message.target.kind.includes('bin')
                && typeof message.executable === 'string') {
                artifact = message.executable;
                features = Array.isArray(message.features) && message.features.every((feature: unknown) => typeof feature === 'string')
                    ? message.features : undefined;
            }
            if (message.reason === 'build-script-executed' && typeof message.out_dir === 'string' && path.isAbsolute(message.out_dir)) {
                directories.add(message.out_dir);
            }
            if (message.reason === 'compiler-message' && message.message?.rendered) output.append(message.message.rendered);
        } catch { output.appendLine(line); }
    });
    lines(child.stderr, line => output.appendLine(line));
    return { process: proc, metadata: () => ({ manifest: target.manifest, directories: [...directories], package: target.packageName, features }),
        localManifests: () => [...manifests], artifact: proc.closed.then(code => {
        if (code !== 0) throw new Error(`Cargo build failed${code === null ? '' : ` with exit code ${code}`}.`);
        if (!artifact) throw new Error('Cargo did not report an executable for the selected binary.');
        return artifact;
    }) };
}

export async function launchArtifact(artifact: string, target: CargoTarget, output: ProcessOutput,
    onEvent: (event: RuntimeEvent) => void, onMcp: (endpoint: RuntimeMcpEndpoint) => void,
    onFrame?: (receipt: RuntimeFrameReceipt) => void): Promise<{ process: ReloadProcess; ready: Promise<void> }> {
    // Keep the live executable side by side with Cargo's output. Native libraries keep their
    // relative lookup path, and Windows lets Cargo replace its own executable on rebuild.
    const executable = path.join(path.dirname(artifact), `.ferroui-${randomUUID()}-${path.basename(artifact)}`);
    await copyFile(artifact, executable, constants.COPYFILE_FICLONE | constants.COPYFILE_EXCL);
    const cleanup = () => removeLaunchCopy(executable, output);
    let child: ChildProcess;
    try {
        child = spawn(executable, target.args, { ...options(target), env: { ...process.env, FERROUI_EDITOR_RELOAD: '1',
            ...(onFrame ? { FERROUI_RELOAD_FRAMES: '1' } : {}) } });
    } catch (error) {
        await cleanup();
        throw error;
    }
    const proc = new ReloadProcess(child, cleanup);
    let acknowledge: () => void;
    const ready = new Promise<void>(resolve => { acknowledge = resolve; });
    lines(child.stdout, line => output.appendLine(line));
    lines(child.stderr, line => {
        if (line.startsWith(mcpEndpointPrefix)) {
            const endpoint = parseMcpEndpoint(line.slice(mcpEndpointPrefix.length));
            if (endpoint) onMcp(endpoint);
            return;
        }
        if (onFrame && line.startsWith(reloadFramePrefix)) {
            const receipt = parseFrameReceipt(line.slice(reloadFramePrefix.length));
            if (receipt) onFrame(receipt);
            return;
        }
        if (!line.startsWith(eventPrefix)) { output.appendLine(line); return; }
        const event = parseEvent(line.slice(eventPrefix.length));
        if (!event) return;
        if (event.status === 'ready') acknowledge();
        onEvent(event);
    });
    return { process: proc, ready };
}

async function removeLaunchCopy(file: string, output: ProcessOutput): Promise<void> {
    for (let retry = 0; ; retry++) {
        try {
            await unlink(file);
            return;
        } catch (error) {
            const code = (error as NodeJS.ErrnoException).code;
            if (code === 'ENOENT') return;
            if (retry === 3 || (code !== 'EBUSY' && code !== 'EPERM' && code !== 'EACCES')) {
                output.appendLine(`Could not remove retired launch copy ${path.basename(file)}. ${String(error)}`);
                return;
            }
            // Windows can retain the executable briefly after the child process exits.
            await delay(100 * (retry + 1));
        }
    }
}

function options(target: CargoTarget) {
    return { cwd: path.dirname(target.manifest), windowsHide: true, detached: process.platform !== 'win32',
        stdio: ['ignore', 'pipe', 'pipe'] as ['ignore', 'pipe', 'pipe'] };
}

function lines(stream: NodeJS.ReadableStream | null, line: (text: string) => void): void {
    if (stream) createInterface({ input: stream, crlfDelay: Infinity }).on('line', line);
}

function parseEvent(text: string): RuntimeEvent | undefined {
    try {
        const event = JSON.parse(text) as Omit<RuntimeEvent, 'acknowledgedAt'>;
        if (!['xaml', 'rsx'].includes(event.language)) return undefined;
        if ((event.status !== 'ready' && event.status !== 'applied') || !positiveId(event.frame)) delete event.frame;
        if (event.status === 'ready' || (['applied', 'failed', 'rebuild'].includes(event.status) && typeof event.file === 'string'
            && typeof event.revision === 'string' && /^[a-f0-9]{16}$/.test(event.revision))) {
            return { ...event, acknowledgedAt: Date.now() };
        }
    } catch { /* unrelated process output */ }
    return undefined;
}

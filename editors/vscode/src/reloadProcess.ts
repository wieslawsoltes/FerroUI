import { spawn, type ChildProcess } from 'node:child_process';
import { constants } from 'node:fs';
import { copyFile, rm } from 'node:fs/promises';
import * as path from 'node:path';
import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';
import type { CargoTarget } from './cargoTarget';
import type { ProcessOutput } from './reloadLog';
import { mcpEndpointPrefix, parseMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';

const eventPrefix = 'FERROUI_RELOAD_V1=';

export interface RuntimeEvent {
    language: 'xaml' | 'rsx';
    status: 'ready' | 'applied' | 'failed' | 'rebuild';
    file?: string;
    revision?: string;
    message?: string;
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

    constructor(private readonly child: ChildProcess) {
        this.closed = new Promise((resolve, reject) => {
            child.once('error', reject);
            child.once('close', code => {
                this.exited = true;
                clearTimeout(this.escalation);
                resolve(code);
            });
        });
    }

    get pid(): number | undefined { return this.child.pid; }

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

export interface ProjectMetadata { manifest: string; directories: string[]; package?: string }

export function buildCargo(target: CargoTarget, output: ProcessOutput): { process: ReloadProcess; artifact: Promise<string>; metadata(): ProjectMetadata } {
    const args = ['build', '--manifest-path', target.manifest, '--package', target.packageName,
        '--bin', target.binary, '--message-format=json-render-diagnostics'];
    if (target.features.length) args.push('--features', target.features.join(','));
    const child = spawn(target.cargo, args, { ...options(target), cwd: target.cargoDirectory });
    const proc = new ReloadProcess(child);
    let artifact: string | undefined;
    const directories = new Set<string>();
    lines(child.stdout, line => {
        try {
            const message = JSON.parse(line);
            if (message.reason === 'compiler-artifact' && message.package_id === target.packageId
                && message.target?.name === target.binary && message.target.kind.includes('bin')
                && typeof message.executable === 'string') artifact = message.executable;
            if (message.reason === 'build-script-executed' && typeof message.out_dir === 'string' && path.isAbsolute(message.out_dir)) {
                directories.add(message.out_dir);
            }
            if (message.reason === 'compiler-message' && message.message?.rendered) output.append(message.message.rendered);
        } catch { output.appendLine(line); }
    });
    lines(child.stderr, line => output.appendLine(line));
    return { process: proc, metadata: () => ({ manifest: target.manifest, directories: [...directories], package: target.packageName }), artifact: proc.closed.then(code => {
        if (code !== 0) throw new Error(`Cargo build failed${code === null ? '' : ` with exit code ${code}`}.`);
        if (!artifact) throw new Error('Cargo did not report an executable for the selected binary.');
        return artifact;
    }) };
}

export async function launchArtifact(artifact: string, target: CargoTarget, output: ProcessOutput,
    onEvent: (event: RuntimeEvent) => void, onMcp: (endpoint: RuntimeMcpEndpoint) => void): Promise<{ process: ReloadProcess; ready: Promise<void> }> {
    // Keep the live executable side by side with Cargo's output. Native libraries keep their
    // relative lookup path, and Windows lets Cargo replace its own executable on rebuild.
    const executable = path.join(path.dirname(artifact), `.ferroui-${randomUUID()}-${path.basename(artifact)}`);
    await copyFile(artifact, executable, constants.COPYFILE_FICLONE);
    const child = spawn(executable, target.args, { ...options(target), env: { ...process.env, FERROUI_EDITOR_RELOAD: '1' } });
    const proc = new ReloadProcess(child);
    let acknowledge: () => void;
    const ready = new Promise<void>(resolve => { acknowledge = resolve; });
    lines(child.stdout, line => output.appendLine(line));
    lines(child.stderr, line => {
        if (line.startsWith(mcpEndpointPrefix)) {
            const endpoint = parseMcpEndpoint(line.slice(mcpEndpointPrefix.length));
            if (endpoint) onMcp(endpoint);
            return;
        }
        if (!line.startsWith(eventPrefix)) { output.appendLine(line); return; }
        const event = parseEvent(line.slice(eventPrefix.length));
        if (!event) return;
        if (event.status === 'ready') acknowledge();
        onEvent(event);
    });
    const cleanup = () => rm(executable, { force: true, maxRetries: 3 }).catch(error => output.appendLine(String(error)));
    void proc.closed.then(cleanup, cleanup);
    return { process: proc, ready };
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
        const event = JSON.parse(text) as RuntimeEvent;
        if (!['xaml', 'rsx'].includes(event.language)) return undefined;
        if (event.status === 'ready') return event;
        if (['applied', 'failed', 'rebuild'].includes(event.status) && typeof event.file === 'string'
            && typeof event.revision === 'string' && /^[a-f0-9]{16}$/.test(event.revision)) return event;
    } catch { /* unrelated process output */ }
    return undefined;
}

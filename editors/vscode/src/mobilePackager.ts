import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { OwnedProcess } from './ownedProcess';
import type { ProcessOutput } from './reloadLog';

export async function runMobilePackager(platform: string, args: string[], workspace: string, env: NodeJS.ProcessEnv,
    output: ProcessOutput, signal: AbortSignal): Promise<void> {
    signal.throwIfAborted();
    const child = spawn('bash', args, {
        cwd: workspace, detached: true, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env,
    });
    const compiler = new OwnedProcess(child);
    const stop = () => { void compiler.stop(); };
    signal.addEventListener('abort', stop, { once: true });
    for (const stream of [child.stdout, child.stderr]) {
        createInterface({ input: stream }).on('line', line => output.appendLine(line));
    }
    let exit: number | null;
    try { exit = await compiler.closed; }
    finally { signal.removeEventListener('abort', stop); }
    signal.throwIfAborted();
    if (exit !== 0) throw new Error(`The ${platform} build failed${exit === null ? '' : ` with exit code ${exit}`}. See Show App Output.`);
}

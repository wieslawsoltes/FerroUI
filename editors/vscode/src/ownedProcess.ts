import { spawn, type ChildProcess } from 'node:child_process';

export class OwnedProcess {
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

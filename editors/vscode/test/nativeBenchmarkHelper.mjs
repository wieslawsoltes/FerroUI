import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { basename } from 'node:path';
import { createMessageConnection, StreamMessageReader, StreamMessageWriter } from 'vscode-jsonrpc/node.js';

export async function withHelper(executable, run) {
    const child = spawn(executable, [], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
    const closed = new Promise(resolve => child.once('close', resolve));
    let stderr = '';
    child.stderr.setEncoding('utf8').on('data', chunk => { stderr = (stderr + chunk).slice(-4096); });
    let connection;
    try {
        await once(child, 'spawn');
        connection = createMessageConnection(new StreamMessageReader(child.stdout), new StreamMessageWriter(child.stdin));
        connection.listen();
        const helper = {
            pid: child.pid,
            async request(method, params) {
                let timer;
                try {
                    return await Promise.race([
                        connection.sendRequest(method, params),
                        new Promise((_, reject) => {
                            timer = setTimeout(() => reject(new Error(`Timed out waiting for ${method}`)), 20_000);
                        }),
                    ]);
                } finally { clearTimeout(timer); }
            },
            notify: (method, params) => connection.sendNotification(method, params),
        };
        return await run(helper);
    } catch (error) {
        const detail = stderr.trim();
        throw new Error(`${basename(executable)}: ${error.message}${detail ? `\n${detail}` : ''}`, { cause: error });
    } finally {
        connection?.dispose();
        child.stdin.end();
        const timer = setTimeout(() => child.kill(), 1_000);
        try { await closed; }
        finally { clearTimeout(timer); }
    }
}

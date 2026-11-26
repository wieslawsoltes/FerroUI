import { strict as assert } from 'node:assert';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import type { AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { attachIosDeviceInspector, watchIosDeviceInspector, type IosDeviceAttachment } from '../src/iosDeviceAttachment';
import { RuntimeClient } from '../src/runtimeClient';
import { deviceFrame, deviceProfile, readyDevice } from './iosDeviceFixture';

const device = { ...readyDevice, status: 'ready' as const, udid: '00008110-0123456789ABCDEF' };
const credentials = { port: 54_321, token: 'a'.repeat(64) };
const pid = 9421;
const instance = 'b'.repeat(32);

export async function verifyIosDeviceAttachment(): Promise<void> {
    if (process.platform === 'win32') return;
    const root = await mkdtemp(join(tmpdir(), 'ferroui-ios-usb-test-'));
    const controller = new AbortController();
    const signal = AbortSignal.any([controller.signal, AbortSignal.timeout(15_000)]);
    let attachment: IosDeviceAttachment | undefined;
    let running = [pid];
    let reportedPid = pid + 1;
    let reportedInstance = instance;
    let fixtureCalls = 0;
    let identityChecks = 0;
    let changesDuringAttach = false;
    let slowRequest!: () => void;
    const slowStarted = new Promise<void>(resolve => { slowRequest = resolve; });
    const server = createServer((request, response) => {
        void reply(request, response).catch(error => response.destroy(error));
    });
    async function reply(request: IncomingMessage, response: ServerResponse) {
        assert.equal(request.headers.host, `127.0.0.1:${credentials.port}`);
        if (request.headers.authorization !== `Bearer ${credentials.token}`) {
            response.writeHead(401).end();
            return;
        }
        if (request.method === 'DELETE') {
            response.writeHead(204).end();
            return;
        }
        const text = await request.toArray();
        const message = JSON.parse(Buffer.concat(text).toString());
        const answer = (result: unknown) => response.end(JSON.stringify({ jsonrpc: '2.0', id: message.id, result }));
        if (message.method === 'initialize') {
            identityChecks++;
            response.setHeader('Mcp-Session-Id', 'ios-usb-fixture');
            answer({ protocolVersion: '2025-11-25', _meta: { 'org.ferroui/inspector': { pid: reportedPid, instance: reportedInstance } } });
            return;
        }
        assert.equal(request.headers['mcp-session-id'], 'ios-usb-fixture');
        if (message.method === 'notifications/initialized') {
            response.writeHead(202).end();
            return;
        }
        fixtureCalls++;
        if (message.method === 'tools/list') {
            answer({ tools: ['list_windows', 'get_tree', 'screenshot'].map(name => ({ name, inputSchema: { type: 'object' } })) });
        } else if (message.params.name === 'list_windows') {
            answer({ content: [{ type: 'text', text: '[{"node_id":1,"bounds":{"width":100,"height":80}}]' }] });
        } else if (message.params.name === 'screenshot') {
            answer({ content: [{ type: 'image', mimeType: 'image/png', data: deviceFrame }] });
            if (changesDuringAttach) running = [pid + 1];
        } else {
            assert.equal(message.params.name, 'get_tree');
            slowRequest();
        }
    }
    server.listen(0, '127.0.0.1');
    try {
        await once(server, 'listening');
        const executable = await writeUsbFixture(root, (server.address() as AddressInfo).port);
        const bridge = { async appProcesses() { return running; } };
        const attach = (request = credentials) => attachIosDeviceInspector(bridge, device, deviceProfile, request, executable, signal);
        await assert.rejects(attach({ ...credentials, token: 'c'.repeat(64) }), /401/);
        await assert.rejects(attach(), /inspector does not identify the selected app process/);
        assert.equal(fixtureCalls, 0);

        reportedPid = pid;
        changesDuringAttach = true;
        await assert.rejects(attach(), /changed while connecting/);
        assert.equal(running[0], pid + 1);
        running = [pid];
        changesDuringAttach = false;
        attachment = await attach();
        assert.equal(attachment.pid, pid);
        assert.deepEqual(attachment.endpoint.expectedProcess, { pid, instance });
        assert.ok(attachment.matches(credentials.port, credentials.token));
        assert.ok(!attachment.matches(credentials.port + 1, credentials.token));
        const client = new RuntimeClient(attachment.endpoint);
        try {
            assert.equal(await client.call<string>('screenshot', { node_id: 1 }, signal), deviceFrame);
            assert.ok(identityChecks >= 4);
        } finally { client.dispose(); }

        reportedInstance = 'd'.repeat(32);
        const replacement = new RuntimeClient(attachment.endpoint);
        try {
            await assert.rejects(replacement.tools(signal), /inspector does not identify the selected app process/);
            await assert.rejects(attachment.verify(signal), /inspector does not identify the selected app process/);
        } finally { replacement.dispose(); }
        reportedInstance = instance;

        const active = new RuntimeClient(attachment.endpoint);
        const pending = assert.rejects(active.call('get_tree', {}, signal));
        await slowStarted;
        await attachment.close();
        await pending;
        active.dispose();
        assert.ok(server.listening, 'Disconnecting the private stream leaves the independently running inspector available');
        assert.deepEqual(running, [pid]);

        const retired = watchIosDeviceInspector(attachment, signal, 5);
        running = [];
        await retired;
    } finally {
        controller.abort();
        await attachment?.close();
        server.closeAllConnections();
        server.close();
        await rm(root, { recursive: true, force: true });
    }
}

async function writeUsbFixture(folder: string, address: number): Promise<string> {
    const executable = join(folder, 'inetcat');
    const helper = join(folder, 'usb.cjs');
    const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
    await writeFile(executable, `#!/bin/sh\nELECTRON_RUN_AS_NODE=1 exec ${quote(process.execPath)} ${quote(helper)} "$@"\n`, { mode: 0o700 });
    await writeFile(helper, `
const assert = require('node:assert/strict');
const { connect } = require('node:net');
assert.deepEqual(process.argv.slice(2), ${JSON.stringify(['--local', '--udid', device.udid, String(credentials.port)])});
assert.equal(process.env.USBMUXD_SOCKET_ADDRESS, undefined);
const socket = connect(${address}, '127.0.0.1');
process.stdin.pipe(socket);
socket.pipe(process.stdout);
socket.on('error', () => process.exit(1));
`);
    return executable;
}

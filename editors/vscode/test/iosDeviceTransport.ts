import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { IosDeviceBridge, deviceInventory } from '../src/iosDeviceBridge';
import { launchIosDeviceApp } from '../src/iosDeviceProcess';
import { RuntimeClient } from '../src/runtimeClient';

const id = '01234567-89ab-cdef-0123-456789abcdef';
const bundleId = 'com.ferroui.ContactReview';
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==';
const profile = { id: 'test', bundleId, label: 'Contact Review', folder: 'file:///test', args: ['--mcp-stdio', '--mcp-input'] };
const record = {
    identifier: id,
    hardwareProperties: { platform: 'iOS' },
    deviceProperties: { name: 'Test iPhone', osVersionNumber: '26.4', developerModeStatus: 'enabled' },
    connectionProperties: { pairingState: 'paired', tunnelState: 'disconnected', transportType: 'wired' },
};
const ready = { id, name: 'Test iPhone', system: 'iOS 26.4', status: 'ready' };

export async function verifyIosDeviceTransport(): Promise<void> {
    const devices = deviceInventory({ devices: [
        record,
        { ...record, connectionProperties: { pairingState: 'paired', tunnelState: 'unavailable' } },
        { ...record, connectionProperties: { pairingState: 'unpaired' } },
        { ...record, deviceProperties: { ...record.deviceProperties, developerModeStatus: 'disabled' } },
        { ...record, hardwareProperties: { platform: 'watchOS' } },
        { ...record, identifier: 'invalid-id' },
    ] });
    assert.deepEqual(devices.map(device => device.status), ['ready', 'unavailable', 'unpaired', 'developerMode']);
    assert.deepEqual(devices[0], ready);
    assert.throws(() => deviceInventory({}), /CoreDevice inventory/);
    if (process.platform === 'win32') return;

    const scratch = await mkdtemp(join(tmpdir(), 'ferroui-coredevice-test-'));
    const executable = join(scratch, 'xcrun');
    const helper = join(scratch, 'device.cjs');
    const signaled = join(scratch, 'signal');
    const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
    const signal = AbortSignal.timeout(8000);
    try {
        await writeFile(executable, `#!/bin/sh\nELECTRON_RUN_AS_NODE=1 exec ${quote(process.execPath)} ${quote(helper)} "$@"\n`, { mode: 0o700 });
        await writeFile(helper, consoleFixture(signaled));
        const [device] = await new IosDeviceBridge(executable).devices(signal);
        assert.deepEqual(device, ready);

        const log: string[] = [];
        const output = { append: (line: string) => log.push(line), appendLine: (line: string) => log.push(line) };
        const app = await launchIosDeviceApp(executable, device, profile, output, signal);
        const client = new RuntimeClient(app.endpoint);
        try {
            assert.equal(app.pid, 9421);
            assert.equal(await client.call<string>('screenshot', { node_id: 1 }, signal), png);
            assert.ok(log.includes('Console log · café'));
            assert.ok(log.every(line => !/FERROUI_MCP_DEVICE_V1|Authorization|launch:/i.test(line)));

            const cancelled = new AbortController();
            const slow = client.call('get_tree', {}, cancelled.signal);
            const rejected = assert.rejects(slow, /Cancel the first query/);
            assert.deepEqual(await client.call('list_windows', {}, signal), [{ node_id: 1, bounds: { width: 100, height: 80 } }]);
            cancelled.abort(new Error('Cancel the first query'));
            await rejected;
        } finally {
            client.dispose();
            await app.close();
        }
        assert.equal(await readFile(signaled, 'utf8'), 'owned');

        const unowned = new AbortController();
        await assert.rejects(launchIosDeviceApp(executable, device, { ...profile, args: ['--already-running'] }, {
            append() {},
            appendLine(line) { if (line === 'Existing app has no launch receipt') unowned.abort(new Error('Cancel unowned launch')); },
        }, AbortSignal.any([unowned.signal, signal])), /Cancel unowned launch/);
        assert.equal(await readFile(signaled, 'utf8'), 'owned');
    } finally {
        await rm(scratch, { recursive: true, force: true });
    }
}

function consoleFixture(signaled: string): string {
    return `
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { createInterface } = require('node:readline');
const args = process.argv.slice(2);
if (args[1] === 'list') {
    assert.deepEqual(args.slice(0, 3), ['devicectl', 'list', 'devices']);
    fs.writeFileSync(args[args.indexOf('--json-output') + 1], JSON.stringify({ result: { devices: [${JSON.stringify(record)}] } }));
} else {
    assert.deepEqual(args.slice(0, 8), ['devicectl', 'device', 'process', 'launch', '--console', '--device', ${JSON.stringify(id)}, '--']);
    assert.equal(args[8], ${JSON.stringify(bundleId)});
    assert.equal(process.env.DEVICECTL_CHILD_FERROUI_EDITOR_SOURCE, '1');
    const launch = process.env.DEVICECTL_CHILD_FERROUI_MCP_DEVICE_LAUNCH;
    assert.match(launch, /^[0-9a-f]{32}$/);
    const unowned = args[9] === '--already-running';
    process.on('SIGTERM', () => {
        fs.writeFileSync(${JSON.stringify(signaled)}, unowned ? 'unowned' : 'owned');
        process.exit(0);
    });
    setInterval(() => {}, 1000);
    process.stderr.write('FERROUI_MCP_DEVICE_V1={"launch":"foreign","pid":123}\\r\\n');
    if (unowned) process.stdout.write('Existing app has no launch receipt\\n');
    else {
        assert.deepEqual(args.slice(9), ['--mcp-stdio', '--mcp-input']);
        const receipt = 'FERROUI_MCP_DEVICE_V1=' + JSON.stringify({ launch, pid: 9421 });
        process.stderr.write('Console log · café\\r\\nAuthorization: Bearer secret\\nlaunch: ' + launch + '\\n');
        process.stderr.write(receipt.slice(0, 15));
        setTimeout(() => process.stderr.write(receipt.slice(15) + '\\r\\n'), 10);
        let initialized = false;
        createInterface({ input: process.stdin }).on('line', line => {
            const call = JSON.parse(line);
            const reply = result => process.stdout.write(JSON.stringify({jsonrpc: '2.0', id: call.id, result}) + '\\n');
            if (call.method === 'initialize') reply({ protocolVersion: '2025-11-25' });
            else if (call.method === 'notifications/initialized') initialized = true;
            else if (call.method === 'tools/list') {
                assert.ok(initialized);
                reply({ tools: ['list_windows', 'get_tree', 'screenshot'].map(name => ({name, inputSchema: {type: 'object'}})) });
            } else if (call.method === 'tools/call') {
                assert.ok(initialized);
                if (call.params.name === 'list_windows') reply({ content: [{ type: 'text', text: '[{"node_id":1,"bounds":{"width":100,"height":80}}]' }] });
                if (call.params.name === 'screenshot') reply({ content: [{ type: 'image', mimeType: 'image/png', data: ${JSON.stringify(png)} }] });
            }
        });
    }
}
`;
}

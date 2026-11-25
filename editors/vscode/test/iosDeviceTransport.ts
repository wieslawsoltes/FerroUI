import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { IosDeviceBridge, deviceInventory } from '../src/iosDeviceBridge';
import { launchIosDeviceApp } from '../src/iosDeviceProcess';
import { RuntimeClient } from '../src/runtimeClient';

import { createIosDeviceFixture, deviceFrame as png, deviceProfile as profile, deviceRecord as record, readyDevice as ready } from './iosDeviceFixture';

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
    const signal = AbortSignal.timeout(8000);
    try {
        const { executable, signaled } = await createIosDeviceFixture(scratch);
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

import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export const deviceId = '01234567-89ab-cdef-0123-456789abcdef';
export const deviceBundleId = 'com.ferroui.ContactReview';
export const deviceFrame = 'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==';
export const deviceProfile = { id: 'test', bundleId: deviceBundleId, label: 'Contact Review', folder: 'file:///test', args: ['--mcp-stdio', '--mcp-input'] };
export const deviceRecord = {
    identifier: deviceId,
    hardwareProperties: { platform: 'iOS' },
    deviceProperties: { name: 'Test iPhone', osVersionNumber: '26.4', developerModeStatus: 'enabled' },
    connectionProperties: { pairingState: 'paired', tunnelState: 'disconnected', transportType: 'wired' },
};
export const readyDevice = { id: deviceId, name: 'Test iPhone', system: 'iOS 26.4', status: 'ready' };

export async function createIosDeviceFixture(scratch: string) {
    const executable = join(scratch, 'xcrun');
    const helper = join(scratch, 'device.cjs');
    const state = { signaled: join(scratch, 'signal'), installed: join(scratch, 'installed'), running: join(scratch, 'running') };
    const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
    await writeFile(executable, `#!/bin/sh\nELECTRON_RUN_AS_NODE=1 exec ${quote(process.execPath)} ${quote(helper)} "$@"\n`, { mode: 0o700 });
    await writeFile(helper, consoleFixture(state));
    return { executable, ...state };
}

function consoleFixture(state: { signaled: string; installed: string; running: string }): string {
    return `
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { createInterface } = require('node:readline');
const args = process.argv.slice(2);
const state = ${JSON.stringify(state)};
const appUrl = 'file:///private/var/containers/Bundle/Application/test/Contact%20Review.app/';
const report = result => fs.writeFileSync(args[args.indexOf('--json-output') + 1], JSON.stringify({result}));
if (args[1] === 'list') {
    assert.deepEqual(args.slice(0, 3), ['devicectl', 'list', 'devices']);
    report({ devices: [${JSON.stringify(deviceRecord)}] });
} else if (args[2] === 'info') {
    assert.deepEqual(args.slice(4, 6), ['--device', ${JSON.stringify(deviceId)}]);
    if (args[3] === 'apps') {
        assert.deepEqual(args.slice(6, 9), ['--bundle-id', ${JSON.stringify(deviceBundleId)}, '--include-default-apps']);
        const apps = fs.existsSync(state.installed) ? [{ bundleIdentifier: ${JSON.stringify(deviceBundleId)}, url: appUrl }] : [];
        report({ apps });
    } else {
        assert.equal(args[3], 'processes');
        const pid = fs.existsSync(state.running) ? Number(fs.readFileSync(state.running, 'utf8')) : 0;
        report({ runningProcesses: pid ? [{ executable: appUrl + 'ContactReview', processIdentifier: pid }] : [] });
    }
} else {
    assert.deepEqual(args.slice(0, 8), ['devicectl', 'device', 'process', 'launch', '--console', '--device', ${JSON.stringify(deviceId)}, '--']);
    assert.equal(args[8], ${JSON.stringify(deviceBundleId)});
    assert.equal(process.env.DEVICECTL_CHILD_FERROUI_EDITOR_SOURCE, '1');
    const launch = process.env.DEVICECTL_CHILD_FERROUI_MCP_DEVICE_LAUNCH;
    assert.match(launch, /^[0-9a-f]{32}$/);
    const unowned = args[9] === '--already-running';
    const managed = !unowned && fs.existsSync(state.installed);
    const failedFrame = managed && fs.readFileSync(state.installed, 'utf8') === 'bad-frame';
    if (managed) fs.writeFileSync(state.running, '9421', { flag: 'wx' });
    process.on('SIGTERM', () => {
        if (managed) fs.unlinkSync(state.running);
        fs.writeFileSync(state.signaled, unowned ? 'unowned' : 'owned');
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
                if (call.params.name === 'screenshot') reply({ content: [{ type: 'image', mimeType: 'image/png', data: failedFrame ? 'broken' : ${JSON.stringify(deviceFrame)} }] });
            }
        });
    }
}
`;
}

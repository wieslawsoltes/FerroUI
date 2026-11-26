import { strict as assert } from 'node:assert';
import type { DebugConfiguration } from 'vscode';
import { configureDesktopLldb } from '../src/desktopLldb';

export function verifyDesktopLldbConfiguration(): void {
    const source: DebugConfiguration = { type: 'lldb-dap', request: 'attach', name: 'Native app', env: { APP: 'value' } };
    const defaults = { PATH: 'tools', lldb_use_lldb_server: '1' };
    const desktop = { ...source };
    configureDesktopLldb(desktop, () => defaults, 'win32');
    assert.deepEqual(desktop, { ...source, debugAdapterEnv: { PATH: 'tools', LLDB_USE_LLDB_SERVER: '0' } });
    assert.deepEqual(defaults, { PATH: 'tools', lldb_use_lldb_server: '1' });

    const otherScope = { TOOL: 'original' };
    const platform: DebugConfiguration = {
        ...source, debugAdapterEnv: otherScope,
        windows: { debugAdapterEnv: ['TRACE=a=b', 'CLEAR', 'lldb_use_lldb_server=1'] },
    };
    const unexpectedEnvironment = () => { throw new Error('The adapter already has an environment'); };
    configureDesktopLldb(platform, unexpectedEnvironment, 'win32');
    assert.deepEqual(platform.windows.debugAdapterEnv, ['TRACE=a=b', 'CLEAR', 'LLDB_USE_LLDB_SERVER=0']);
    assert.deepEqual(platform.debugAdapterEnv, otherScope);

    for (const [configuration, host] of [
        [source, 'darwin'],
        [{ ...source, type: 'lldb' }, 'win32'],
        [{ ...source, request: 'launch' }, 'win32'],
        [{ ...source, windows: { debugAdapterPort: 12345 } }, 'win32'],
    ] as const) {
        const copy = { ...configuration };
        configureDesktopLldb(copy, unexpectedEnvironment, host);
        assert.deepEqual(copy, configuration);
    }
}

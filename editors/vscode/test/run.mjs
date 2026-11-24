import { runTests } from '@vscode/test-electron';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { androidManagedWorkspace } from './androidManagedWorkspace.mjs';
import { testVSCodeExecutable } from './vscodeDownload.mjs';
import { browserBuildWorkspace } from './browserBuildWorkspace.mjs';
import { browserDebugWorkspace } from './browserDebugWorkspace.mjs';

const extension = fileURLToPath(new URL('../', import.meta.url));
const sandbox = await mkdtemp(join(tmpdir(), 'ferroui-vscode-'));
const workspace = join(sandbox, 'workspace');
const profile = join(sandbox, 'profile');
const bin = resolve(process.env.FERROUI_TEST_BIN_DIR ?? join(extension, '../../target/debug'));
const executable = name => join(bin, name + (process.platform === 'win32' ? '.exe' : ''));
const simulator = process.env.FERROUI_TEST_IOS_SIMULATOR;
const simulatorDebugExtensions = process.env.FERROUI_TEST_IOS_DEBUG_EXTENSIONS;
const android = process.env.FERROUI_TEST_ANDROID_DEVICE;
const androidDebugExtensions = process.env.FERROUI_TEST_ANDROID_DEBUG_EXTENSIONS;
const androidManaged = process.env.FERROUI_TEST_ANDROID_MANAGED === '1';
const browser = process.env.FERROUI_TEST_BROWSER_URL;
const browserBuild = process.env.FERROUI_TEST_BROWSER_BUILD;
const browserDebugSite = process.env.FERROUI_TEST_BROWSER_DEBUG_SITE;
const browserDwarfExtensions = process.env.FERROUI_TEST_BROWSER_DWARF_EXTENSIONS;
try {
    let target = workspace;
    let targetSettings = {};
    let targetEnv;
    if (simulatorDebugExtensions && !simulator) throw new Error('Set FERROUI_TEST_IOS_SIMULATOR for simulator debugger acceptance.');
    if (androidDebugExtensions && !android) throw new Error('Set FERROUI_TEST_ANDROID_DEVICE for Android debugger acceptance.');
    if ([simulator, android, androidManaged, browser, browserBuild, browserDebugSite, browserDwarfExtensions].filter(Boolean).length > 1) {
        throw new Error('Choose one platform acceptance fixture per run.');
    }
    if (simulator) {
        const appPath = process.env.FERROUI_TEST_IOS_SIMULATOR_APP;
        if (process.platform !== 'darwin' || !appPath) {
            throw new Error('Set FERROUI_TEST_IOS_SIMULATOR_APP to the built Contact Review bundle on the workspace Mac.');
        }
        target = resolve(process.env.FERROUI_TEST_IOS_SIMULATOR_SOURCE ?? join(extension, '../..'));
        targetSettings = { 'ferroui.iosSimulator.apps': [{
            id: 'contact-review', label: 'Contact Review', bundleId: 'org.ferroui.simulator-inspector',
            appPath: resolve(appPath), args: ['--mcp', '--mcp-input'],
        }] };
        targetEnv = { FERROUI_TEST_IOS_SIMULATOR: simulator };
        if (simulatorDebugExtensions) {
            targetSettings['lldb-dap.executable-path'] = execFileSync('/usr/bin/xcrun', ['--find', 'lldb-dap'], { encoding: 'utf8' }).trim();
            targetSettings.launch = { configurations: [{ name: 'Attach to Contact Review', type: 'lldb-dap', request: 'attach',
                pid: '${command:ferroui.hotReload.processId}' }] };
            targetEnv.FERROUI_TEST_IOS_DEBUG = '1';
        }
    } else if (androidManaged) {
        const fixture = await androidManagedWorkspace(workspace, sandbox);
        targetSettings = fixture.settings;
        targetEnv = fixture.env;
    } else if (android) {
        const apkPath = process.env.FERROUI_TEST_ANDROID_APK;
        const adbPath = process.env.FERROUI_TEST_ANDROID_ADB;
        if (!apkPath || !adbPath) throw new Error('Set FERROUI_TEST_ANDROID_APK and FERROUI_TEST_ANDROID_ADB for the Service Review fixture.');
        target = resolve(process.env.FERROUI_TEST_ANDROID_SOURCE ?? join(extension, '../..'));
        const app = { id: 'service-review', label: 'Service Review', packageId: 'org.ferroui.androidinspector', apkPath: resolve(apkPath) };
        targetSettings = { 'ferroui.android.adbPath': resolve(adbPath), 'ferroui.android.apps': [app] };
        targetEnv = { FERROUI_TEST_ANDROID_DEVICE: android };
        if (androidDebugExtensions) {
            const symbols = process.env.FERROUI_TEST_ANDROID_DEBUG_SYMBOLS;
            if (!symbols) throw new Error('Set FERROUI_TEST_ANDROID_DEBUG_SYMBOLS to the matching unstripped libandroid_inspector.so.');
            app.symbolFiles = [resolve(symbols)];
            const adapter = process.env.FERROUI_TEST_ANDROID_LLDB_DAP ?? (process.platform === 'darwin'
                ? execFileSync('/usr/bin/xcrun', ['--find', 'lldb-dap'], { encoding: 'utf8' }).trim() : undefined);
            if (adapter) targetSettings['lldb-dap.executable-path'] = resolve(adapter);
            targetSettings.launch = { configurations: [{ name: 'Attach to Service Review', type: 'lldb-dap', request: 'attach',
                'gdb-remote-port': '${command:ferroui.android.debugPort}' }] };
            targetEnv.FERROUI_TEST_ANDROID_DEBUG = '1';
        }
    } else if (browser || browserBuild || browserDebugSite || browserDwarfExtensions) {
        const chrome = process.env.FERROUI_TEST_BROWSER_EXECUTABLE;
        const toolkit = resolve(process.env.FERROUI_TEST_BROWSER_SOURCE ?? join(extension, '../..'));
        targetSettings = { ...(chrome ? { 'ferroui.browser.executablePath': resolve(chrome) } : {}) };
        if (browserBuild || browserDwarfExtensions) {
            await browserBuildWorkspace(workspace, toolkit);
            targetSettings['ferroui.browser.apps'] = [{ id: 'dispatch-check', label: 'Dispatch Check',
                build: { application: 'dispatch-check', toolchain: toolkit, debugSymbols: Boolean(browserDwarfExtensions) } }];
            targetEnv = { CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR ?? join(sandbox, 'cargo') };
            if (browserDwarfExtensions) targetEnv.FERROUI_TEST_BROWSER_RUST_DEBUG = '1';
            else targetEnv.FERROUI_TEST_BROWSER_BUILD = browserBuild;
            const buildTimeout = process.env.FERROUI_TEST_BROWSER_BUILD_TIMEOUT_MS;
            if (buildTimeout) targetEnv.FERROUI_TEST_BROWSER_BUILD_TIMEOUT_MS = buildTimeout;
        } else if (browserDebugSite) {
            const debugToolkit = await browserDebugWorkspace(workspace, sandbox, resolve(browserDebugSite));
            targetSettings['ferroui.browser.apps'] = [{ id: 'debug-dispatch', label: 'Dispatch Desk',
                build: { application: 'live_inspector', toolchain: debugToolkit } }];
            targetEnv = { FERROUI_TEST_BROWSER_DEBUG: '1', CARGO_TARGET_DIR: join(sandbox, 'cargo') };
        } else {
            target = toolkit;
            targetSettings['ferroui.browser.apps'] = [{ id: 'dispatch-desk', label: 'Dispatch Desk', url: browser }];
            targetEnv = { FERROUI_TEST_BROWSER_URL: browser };
        }
    } else {
        await cp(join(extension, 'test/fixtures'), workspace, { recursive: true });
        const animation = join(workspace, 'animation_view.rs');
        // Seed the CRLF fixture before VS Code watches or loads its text model.
        await writeFile(animation, (await readFile(animation, 'utf8')).replace(/\r?\n/g, '\r\n'));
        await mkdir(join(workspace, 'Assets'));
        await writeFile(join(workspace, 'Assets', 'Preview Image.png'), Buffer.from(
            'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==', 'base64'));
    }
    await mkdir(join(profile, 'User'), { recursive: true });
    const extensions = simulatorDebugExtensions ?? androidDebugExtensions ?? browserDwarfExtensions;
    await writeFile(join(profile, 'User/settings.json'), JSON.stringify({
        ...targetSettings,
        'ferroui.languageServer.path': executable('ferroui-xaml-lsp'),
        'ferroui.preview.path': executable('ferroui-designer-preview'),
        'workbench.startupEditor': 'none',
        'telemetry.telemetryLevel': 'off',
        'files.autoSave': 'off',
        'files.enableTrash': false,
    }));
    await runTests({
        extensionDevelopmentPath: extension,
        extensionTestsPath: join(extension, 'dist/test.js'),
        vscodeExecutablePath: await testVSCodeExecutable(process.env.FERROUI_TEST_VSCODE, {
            version: process.env.FERROUI_TEST_VSCODE_VERSION,
            extensionDevelopmentPath: extension,
        }),
        extensionTestsEnv: targetEnv,
        launchArgs: [target, '--user-data-dir', profile, '--extensions-dir', extensions ? resolve(extensions) : join(sandbox, 'extensions'),
            '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust'],
    });
} finally {
    await rm(sandbox, { recursive: true, force: true, maxRetries: 3 });
}

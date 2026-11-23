import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode, RuntimeSource } from '../src/runtimeProtocol';
import { openEditorMcp, type McpFixture } from './mcpFixture';
import { until } from './wait';

const profile = 'service-review';
const packageId = 'org.ferroui.androidinspector';
const component = `${packageId}/org.ferroui.android.FerroMainActivity`;
const execute = promisify(execFile);

export async function verifyAndroidMcp(device: string): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const sourceUri = vscode.Uri.joinPath(folder, 'src/Android/FerroUI.Android/examples/android_inspector.rs');
    const document = await vscode.workspace.openTextDocument(sourceUri);
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('ferroui.showAppSessions');
    const adbPath = vscode.workspace.getConfiguration('ferroui.android').get<string>('adbPath')!;
    const adb = async (...args: string[]) => (await execute(adbPath, args, { timeout: 10_000 })).stdout.trim();
    const pid = () => adb('-s', device, 'shell', 'pidof', '-s', packageId).catch(() => '');
    const forwards = async () => (await adb('forward', '--list')).split('\n').filter(line => line.startsWith(device + ' ')).sort();
    const { invoke, content, close } = await openEditorMcp(40_000);
    const request = { device, profile, folder: folder.toString() };
    const settings = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = settings.inspect<boolean>('allowWrites')?.globalValue;
    const writes = (value: boolean | undefined) => settings.update('allowWrites', value, vscode.ConfigurationTarget.Global);
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    let other: string | undefined;
    try {
        other = await adb('-s', device, 'forward', '--no-rebind', 'tcp:0', 'tcp:45678');
        const baseline = await forwards();
        assert.ok(baseline.some(line => line.includes(`tcp:${other} tcp:45678`)));
        const targets = content<{ devices: { id: string; status: string }[];
            apps: { id: string; packageId: string; installsApk: boolean }[] }>(await invoke('ferroui_android_targets', { folder: request.folder }));
        assert.ok(targets.devices.some(target => target.id === device && target.status === 'device'));
        assert.ok(targets.apps.some(app => app.id === profile && app.packageId === packageId && app.installsApk));
        await writes(false);
        assert.match((await invoke('ferroui_android_run', request)).content[0]?.text ?? '', /allowWrites/);
        await writes(true);

        const started = content<RuntimeSummary>(await invoke('ferroui_android_run', request));
        assert.equal(started.kind, 'android');
        assert.match(started.runtime, /^android:/);
        assert.equal(started.android?.device, device);
        assert.equal(started.android?.launched, true);
        assert.equal(Number(await pid()), started.pid);
        assert.equal((await forwards()).length, baseline.length + 1);
        assert.equal(await adb('-s', device, 'exec-out', 'run-as', packageId, 'stat', '-c', '%a', 'files/ferroui-live-app.json'), '600');
        assert.equal(content<RuntimeSummary>(await invoke('ferroui_android_run', request)).runtime, started.runtime);
        assert.match((await invoke('ferroui_android_attach', request)).content[0]?.text ?? '', /Close the current Android session/);
        await checkNativeApp({ invoke, content }, started, document);
        await vscode.commands.executeCommand('ferroui.appSessions.stop', started.runtime);
        assert.ok(!(await sessions()).some(session => session.runtime === started.runtime));
        assert.equal(await pid(), '');
        assert.deepEqual(await forwards(), baseline);

        assert.match(await adb('-s', device, 'shell', 'am', 'start', '-W', '-n', component), /Status: ok/);
        const independent = await pid();
        assert.ok(independent);
        const attached = content<RuntimeSummary>(await invoke('ferroui_android_attach', request));
        assert.equal(attached.android?.launched, false);
        assert.equal(attached.pid, Number(independent));
        assert.notEqual(attached.runtime, started.runtime);
        assert.notEqual(attached.generation, started.generation);
        assert.match((await invoke('ferroui_runtime_detach', { runtime: attached.runtime })).content[0]?.text ?? '', /Android session/);
        content(await invoke('ferroui_android_stop', { runtime: attached.runtime }));
        assert.equal(await pid(), independent, 'Disconnect preserves the independently launched app');
        assert.deepEqual(await forwards(), baseline);

        await vscode.commands.executeCommand('ferroui.android.attach', request);
        const reopened = (await sessions()).find(session => session.android?.device === device);
        assert.ok(reopened && reopened.pid === Number(independent));
        await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label.includes(reopened.label))));
        await adb('-s', device, 'shell', 'am', 'force-stop', packageId);
        await until(async () => !(await sessions()).some(session => session.runtime === reopened.runtime));
        await until(async () => (await forwards()).length === baseline.length);
        console.log('Android Live App launch, APK installation, native frames and input, mapped Rust source, attach and exact forward cleanup passed');
    } finally {
        try {
            await writes(true);
            for (const session of await sessions()) {
                if (session.android?.device === device && session.android.profile === profile) {
                    await invoke('ferroui_android_stop', { runtime: session.runtime });
                }
            }
        } finally {
            await adb('-s', device, 'shell', 'am', 'force-stop', packageId).catch(() => {});
            if (other) await adb('-s', device, 'forward', '--remove', `tcp:${other}`).catch(() => {});
            await writes(previous);
            close();
        }
    }
}

async function checkNativeApp({ invoke, content }: McpFixture, { runtime, generation, label }: RuntimeSummary, document: vscode.TextDocument) {
    assert.ok(generation);
    const query = (tool: string, args: object = {}) => invoke('ferroui_runtime_query', {
        runtime, expected_generation: generation, tool, arguments: args,
    });
    const result = async <T>(tool: string, args?: object) => content<{ result: T }>(await query(tool, args)).result;
    const [window] = await result<RuntimeNode[]>('list_windows');
    const [tree] = await result<RuntimeNode[]>('get_tree', { node_id: window.node_id, depth: 10 });
    assert.ok(tree.children?.length);
    const png = async () => {
        const reply = await query('screenshot', { node_id: window.node_id });
        assert.equal(content<{ generation: number }>(reply).generation, generation);
        const image = Buffer.from(reply.content.find(item => item.type === 'image')?.data ?? '', 'base64');
        assert.equal(image.subarray(1, 4).toString(), 'PNG');
        assert.ok(image.readUInt32BE(16) > 200 && image.readUInt32BE(20) > 400);
        return image;
    };
    const before = await png();
    const [button] = await result<RuntimeNode[]>('find_elements', { name: 'ReviewCase' });
    assert.equal(button?.name, 'ReviewCase');
    const compiler = await result<RuntimeSource>('get_source', { node_id: button.node_id });
    assert.ok(compiler.workspacePath?.endsWith('android_inspector.rs'));
    const { source } = content<{ source: { uri: string; position: { line: number }; canApply: boolean } }>(
        await invoke('ferroui_runtime_source', { runtime, node: button.node_id }));
    assert.ok(source);
    assert.equal(source.uri, document.uri.toString());
    assert.ok(document.lineAt(source.position.line).text.includes('ReviewCase'));
    assert.equal(source.canApply, true);
    content(await invoke('ferroui_runtime_open', { runtime }));
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label.includes(label))));
    const [heading] = await result<RuntimeNode[]>('find_elements', { text: 'Service review' });
    const { source: headingSource } = content<{ source: { version: number } }>(
        await invoke('ferroui_runtime_source', { runtime, node: heading.node_id }));
    const original = document.getText();
    try {
        content(await invoke('ferroui_runtime_apply_source', { runtime, node: heading.node_id, property: 'Text',
            value: 'Service desk', expected_generation: generation, expected_version: headingSource.version }));
        assert.ok(document.getText().includes('text: "Service desk"'));
        assert.equal(content<{ source: unknown }>(await invoke('ferroui_runtime_source', { runtime, node: heading.node_id })).source, null);
    } finally {
        if (document.getText() !== original) {
            await vscode.window.showTextDocument(document);
            await vscode.commands.executeCommand('undo');
            await until(() => document.getText() === original);
        }
    }
    content(await invoke('ferroui_runtime_call', { runtime, expected_generation: generation, tool: 'click',
        arguments: { node_id: button.node_id } }));
    await until(async () => (await result<{ matched: boolean }>('wait_for', {
        name: 'ReviewedCount', text: 'Reviewed cases: 1', timeout_ms: 300,
    })).matched);
    assert.notDeepEqual(await png(), before);
    const stale = await invoke('ferroui_runtime_query', { runtime, expected_generation: generation + 1, tool: 'list_windows' });
    assert.match(stale.content[0]?.text ?? '', /application connection changed/);
}

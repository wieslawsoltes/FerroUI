import { spawn } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { mkdir, mkdtemp, realpath, rm, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';
import type { DevelopmentBrowserApp } from './browserTargets';
import { inCargoBuildSlot } from './cargoBuildGate';
import { browserCargoTarget, type CargoTarget, type DiscoveredCargoTarget } from './cargoTarget';
import { OwnedProcess } from './ownedProcess';
import type { ProcessOutput } from './reloadLog';
import type { ProjectMetadata } from './reloadProcess';
import { fileKey } from './workspacePaths';

export interface BrowserBuildTarget {
    script: string;
    toolkit: string;
    workspace: string;
    cargo: DiscoveredCargoTarget;
    hostPage: string;
    workspaceInputs: string[];
}

export interface BuiltBrowserSite {
    site: string;
    manifests: string[];
    metadata: ProjectMetadata;
    dispose(): Promise<void>;
}

export async function resolveBrowserBuild(app: DevelopmentBrowserApp, signal: AbortSignal): Promise<BrowserBuildTarget> {
    const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.parse(app.folder, true));
    if (folder?.uri.scheme !== 'file' || folder.uri.toString() !== app.folder) throw new Error('Open the browser app’s workspace folder.');
    const { application, toolchain, manifestPath, features } = app.build;
    const toolkit = await realpath(path.resolve(folder.uri.fsPath, toolchain));
    const script = path.join(toolkit, 'scripts', 'build-browser.sh');
    if (!await stat(script).then(info => info.isFile(), () => false)) throw new Error('Choose a FerroUI browser toolchain with scripts/build-browser.sh.');
    const example = path.join(toolkit, 'src', 'Browser', 'FerroUI.Browser', 'examples', application);
    const exampleWorkspace = await stat(example).then(info => info.isDirectory(), () => false) ? toolkit : undefined;
    const cargo = await browserCargoTarget(folder, {
        manifestPath, package: application, binary: application, label: app.label, features,
    }, exampleWorkspace, signal);
    const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
    return { script, toolkit, cargo, workspace,
        hostPage: fileKey(path.join(cargo.kind === 'example' ? example : path.dirname(cargo.manifest), 'wwwroot')),
        workspaceInputs: ['Cargo.lock', '.cargo/config', '.cargo/config.toml'].map(file => fileKey(path.join(workspace, file))) };
}

export async function buildBrowserSite(target: BrowserBuildTarget, app: DevelopmentBrowserApp, output: ProcessOutput,
    signal: AbortSignal, starting: () => void): Promise<BuiltBrowserSite> {
    // The toolchain also emits a shared, validated JavaScript module. See the Cargo build slot
    // across the whole read of those artifacts and across profiles sharing this toolchain.
    const toolkitSlot = fileKey(path.join(target.toolkit, 'src', 'Browser', 'FerroUI.Browser', 'webapp'));
    const [first, second] = [...new Set([toolkitSlot, fileKey(target.cargo.targetDirectory)])].sort();
    const run = () => { starting(); return assemble(target, app, output, signal); };
    return inCargoBuildSlot(first, signal, () => second ? inCargoBuildSlot(second, signal, run) : run());
}

async function assemble(target: BrowserBuildTarget, app: DevelopmentBrowserApp, output: ProcessOutput,
    signal: AbortSignal): Promise<BuiltBrowserSite> {
    await mkdir(target.cargo.targetDirectory, { recursive: true });
    const directory = await mkdtemp(path.join(target.cargo.targetDirectory, '.ferroui-browser-'));
    const site = path.join(directory, 'site');
    const messages = path.join(directory, 'cargo.jsonl');
    const dispose = () => rm(directory, { recursive: true, force: true, maxRetries: 3 });
    try {
        const { application, features, threads, debugSymbols } = app.build;
        const args = [target.script, application, debugSymbols ? '--debug-symbols' : '--debug',
            '--out', site, '--reuse-webapp', '--editor-messages', messages];
        if (features.length) args.push('--features', features.join(','));
        if (threads) args.push('--threads');
        signal.throwIfAborted();
        const child = spawn('bash', args, {
            cwd: target.workspace, windowsHide: true, detached: true, stdio: ['ignore', 'pipe', 'pipe'],
            env: { ...process.env, FERROUI_BROWSER_WORKSPACE: target.workspace, CARGO_TARGET_DIR: target.cargo.targetDirectory },
        });
        const compiler = new OwnedProcess(child);
        const abort = () => { void compiler.stop(); };
        signal.addEventListener('abort', abort, { once: true });
        for (const stream of [child.stdout, child.stderr]) {
            createInterface({ input: stream }).on('line', line => output.appendLine(line));
        }
        let exit: number | null;
        try { exit = await compiler.closed; }
        finally { signal.removeEventListener('abort', abort); }
        signal.throwIfAborted();
        if (exit !== 0) throw new Error(`The browser build failed${exit === null ? '' : ` with exit code ${exit}`}.`);
        if (!await stat(path.join(site, 'index.html')).then(info => info.isFile(), () => false)) throw new Error('The browser build did not produce index.html.');
        const report = await readReport(messages, target.cargo);
        await rm(messages);
        return { site, dispose, ...report };
    } catch (error) {
        await dispose();
        throw error;
    }
}

async function readReport(file: string, target: CargoTarget) {
    const manifests = new Set<string>();
    const directories = new Set<string>();
    let features = target.features;
    const input = createReadStream(file);
    const lines = createInterface({ input });
    try {
        for await (const line of lines) {
            if (!line.startsWith('{')) continue;
            const message = JSON.parse(line);
            if (message.reason === 'compiler-artifact' && typeof message.manifest_path === 'string' && path.isAbsolute(message.manifest_path)) {
                manifests.add(message.manifest_path);
                if (message.package_id === target.packageId && message.target?.name === target.binary && Array.isArray(message.features)) {
                    features = message.features;
                }
            }
            if (message.reason === 'build-script-executed' && typeof message.out_dir === 'string' && path.isAbsolute(message.out_dir)) directories.add(message.out_dir);
        }
    } finally { lines.close(); input.destroy(); }
    return { manifests: [...manifests], metadata: { manifest: target.manifest, directories: [...directories], package: target.packageName, features } };
}

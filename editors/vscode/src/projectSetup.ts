import { realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { discoverCargoTargets, type CargoSelection, type CargoTarget } from './cargoTarget';
import type { ProjectInventory } from './projectInventory';
import { ProjectCargoTasks } from './projectCargoTasks';
import type { ProjectTemplatePlan, ProjectTemplateRequest } from './projectSetupProtocol';
import { errorMessage } from './protocol';
import { choose } from './studioContext';
import { isInside, localFolders, workspaceFolders } from './workspacePaths';

const scheme = 'ferroui-project-review';
const title = 'New FerroUI Project';
const request = () => AbortSignal.timeout(20_000);

export class ProjectSetup implements vscode.Disposable {
    readonly cargoTasks = new ProjectCargoTasks();
    private readonly previews = new Map<string, string>();
    private readonly commands: vscode.Disposable[];

    constructor(private readonly inventory: ProjectInventory) {
        this.commands = [
            vscode.workspace.registerTextDocumentContentProvider(scheme, {
                provideTextDocumentContent: uri => this.previews.get(uri.toString()) ?? '',
            }),
            vscode.workspace.onDidCloseTextDocument(document => this.previews.delete(document.uri.toString())),
            vscode.commands.registerCommand('ferroui.project.newProject', () => this.run(() => this.create())),
            vscode.commands.registerCommand('ferroui.project.checkToolchain', () => this.run(() => this.showToolchain())),
        ];
    }

    catalog(signal: AbortSignal) {
        return this.inventory.templateCatalog(signal);
    }

    async agentTemplate(input: ProjectTemplateRequest & { folder?: string; review?: string }, signal: AbortSignal) {
        trusted();
        const folder = chooseWorkspaceRoot(input.folder);
        const parent = await realpath(folder.uri.fsPath);
        const checkout = input.checkout === undefined ? undefined : await scopedCheckout(parent, input.checkout);
        const { template, name, review } = input;
        const configuration = { template, name, checkout };
        if (review === undefined) return this.inventory.templatePlan(parent, configuration, signal);
        return this.inventory.templateCreate(parent, configuration, review, signal);
    }

    async agentToolchain(input: CargoSelection & { target?: string }, signal: AbortSignal) {
        trusted();
        const folder = chooseWorkspaceRoot(input.folder);
        const selected = input.manifest_path !== undefined || input.binary !== undefined || input.package !== undefined;
        const options = selected ? await discoverCargoTargets({ ...input, folder: folder.uri.toString() }) : [];
        if (selected && options.length !== 1) throw new Error('Choose one Cargo manifest, package, and binary.');
        return this.checkToolchain(folder, options[0], input.target, signal);
    }

    async agentRunCargo(input: CargoSelection, signal: AbortSignal) {
        trusted();
        const folder = chooseWorkspaceRoot(input.folder);
        const options = await discoverCargoTargets({ ...input, folder: folder.uri.toString() });
        if (options.length !== 1) throw new Error('Choose one Cargo manifest, package, and binary.');
        return this.cargoTasks.run(options[0], signal);
    }

    async showToolchain(target?: CargoTarget): Promise<void> {
        trusted();
        const active = vscode.window.activeTextEditor?.document.uri;
        const owner = active && vscode.workspace.getWorkspaceFolder(active);
        const root = target?.folder ?? (owner?.uri.scheme === 'file' ? owner : localFolders().length === 1
            ? localFolders()[0] : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Choose a Cargo workspace folder' }));
        if (!root || root.uri.scheme !== 'file') return;
        const report = await vscode.window.withProgress({ location: vscode.ProgressLocation.Window, title: 'Checking the FerroUI toolchain' },
            () => this.checkToolchain(root, target, undefined, request()));
        const check = (label: string, passed: boolean, detail?: string | null) => ({
            label: `$(${passed ? 'pass' : 'warning'}) ${label}`,
            detail: detail ?? undefined,
            alwaysShow: true,
            open: undefined as string | undefined,
        });
        const entries = [
            check('Cargo', !!report.cargo.version, report.cargo.version ?? report.cargo.error),
            check('Rust compiler', !!report.rustc.version && report.meetsMinimum !== false,
                [report.rustc.version ?? report.rustc.error, `Requires Rust ${report.minimumRust} or later`].filter(Boolean).join(' · ')),
            check('Rust target', report.targetInstalled === true,
                report.targetError ?? (report.target ? `${report.target} · ${report.targetInstalled ? 'Rust libraries installed' : 'Rust libraries missing'}` : 'Target unavailable')),
        ];
        if (target) entries.push({
            label: '$(go-to-file) Open Cargo.toml', detail: `${target.packageName} · ${target.binary}${target.features.length ? ' · ' + target.features.join(', ') : ''}`,
            alwaysShow: true, open: target.manifest,
        });
        const selected = await choose('FerroUI Project Toolchain', entries);
        if (selected?.open) await vscode.commands.executeCommand('vscode.open', vscode.Uri.file(selected.open));
    }

    private async checkToolchain(folder: vscode.WorkspaceFolder, target: CargoTarget | undefined, triple: string | undefined, signal: AbortSignal) {
        const cargo = target?.cargo ?? vscode.workspace.getConfiguration('ferroui.hotReload', folder.uri).get<string>('cargoPath');
        const report = await this.inventory.toolchain({
            directory: target ? path.dirname(target.manifest) : folder.uri.fsPath,
            cargo: cargo || undefined, minimumRust: target?.rustVersion, target: triple,
        }, signal);
        return { ...report, binary: target?.binary, package: target?.packageName,
            manifest: target?.manifest, features: target?.features ?? [] };
    }

    private async create(): Promise<void> {
        trusted();
        const catalog = await this.catalog(request());
        const template = await choose(title, catalog.templates.map(option => ({
            ...option, label: `$(${option.id === 'desktopRsx' ? 'code' : 'layout'}) ${option.label}`, detail: option.description,
        })));
        if (!template) return;
        const [parent] = await vscode.window.showOpenDialog({
            title: 'Choose Project Location', defaultUri: localFolders()[0]?.uri,
            canSelectFolders: true, canSelectFiles: false, canSelectMany: false, openLabel: 'Choose Location',
        }) ?? [];
        if (!parent) return;
        if (parent.scheme !== 'file') throw new Error('Choose a folder on the workspace host.');
        const name = await vscode.window.showInputBox({ title, value: 'task-workbench',
            prompt: 'Name the Cargo package and its new folder',
            validateInput: value => /^[a-z][a-z0-9_-]{0,63}$/.test(value) ? undefined : 'Use lowercase letters, digits, hyphens or underscores.' });
        if (!name) return;
        const dependency = await choose('FerroUI Dependency', [
            { label: `Published FerroUI ${catalog.publishedVersion}`, description: 'crates.io', checkout: false },
            { label: 'FerroUI source checkout', description: 'Local Cargo path dependencies', checkout: true },
        ]);
        if (!dependency) return;
        let checkout: string | undefined;
        if (dependency.checkout) {
            const [folder] = await vscode.window.showOpenDialog({ title: 'Choose the FerroUI Checkout',
                canSelectFolders: true, canSelectFiles: false, canSelectMany: false, openLabel: 'Use Checkout' }) ?? [];
            if (!folder) return;
            if (folder.scheme !== 'file') throw new Error('Choose a FerroUI checkout on the workspace host.');
            checkout = folder.fsPath;
        }
        const configuration = { template: template.id, name, checkout };
        const plan = await this.inventory.templatePlan(parent.fsPath, configuration, request());
        if (!await this.review(plan)) return;
        const project = await this.inventory.templateCreate(parent.fsPath, configuration, plan.review, request());
        const entry = vscode.Uri.file(project.entry);
        if (vscode.workspace.getWorkspaceFolder(entry)) {
            await vscode.commands.executeCommand('ferroui.openDesigner', entry);
        } else {
            await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(project.root), { forceNewWindow: true });
        }
    }

    private async review(plan: ProjectTemplatePlan): Promise<boolean> {
        const files = plan.files.map(file => ({ label: '$(file) ' + file.path, file }));
        const create = { label: '$(new-folder) Create Project', description: path.basename(plan.root),
            detail: vscode.workspace.asRelativePath(vscode.Uri.file(plan.root), true) };
        const choices: Array<vscode.QuickPickItem & { file?: ProjectTemplatePlan['files'][number] }> = [
            create, { label: 'Generated Files', kind: vscode.QuickPickItemKind.Separator }, ...files,
        ];
        while (true) {
            const choice = await choose('Review New FerroUI Project', choices);
            if (choice === undefined || choice === create) return choice === create;
            if (!choice.file) continue;
            const uri = vscode.Uri.from({ scheme, path: '/' + choice.file.path, query: plan.review });
            this.previews.set(uri.toString(), choice.file.source);
            const document = await vscode.workspace.openTextDocument(uri);
            await vscode.window.showTextDocument(document, { preview: true });
        }
    }

    private async run(action: () => Promise<void>): Promise<void> {
        try { await action(); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void {
        this.cargoTasks.dispose();
        this.commands.forEach(command => command.dispose());
        this.previews.clear();
    }
}

function trusted(): void {
    if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before setting up a FerroUI project.');
}

function chooseWorkspaceRoot(reference?: string): vscode.WorkspaceFolder {
    const folders = workspaceFolders(reference);
    if (folders.length !== 1) throw new Error('Choose one folder from ferroui_workspace_files.');
    return folders[0];
}

async function scopedCheckout(root: string, input: string): Promise<string> {
    const checkout = path.resolve(root, input);
    if (path.isAbsolute(input) || !isInside(root, checkout)) throw new Error('Choose a checkout relative to the selected workspace folder.');
    const physical = await realpath(checkout);
    if (!isInside(root, physical)) throw new Error('The checkout must resolve inside the selected workspace folder.');
    return physical;
}

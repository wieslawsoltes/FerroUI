import { realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { DictionaryPlan, DictionaryRequest, DictionaryTarget } from './dictionaryProtocol';
import { DictionaryReview } from './dictionaryReview';
import { exactRange } from './textDocument';
import { newWorkspaceFile } from './workspacePaths';
import { xamlLanguage, type XamlServer } from './xamlServer';

export interface DictionarySite {
    document: vscode.TextDocument;
    request: DictionaryRequest;
    validate?: () => void;
}

export class DictionaryExtraction implements vscode.Disposable {
    private readonly review = new DictionaryReview();

    constructor(private readonly server: XamlServer) {}

    async openReview(site: DictionarySite): Promise<void> {
        const target = await this.target(site);
        const fileName = await vscode.window.showInputBox({
            title: 'Extract Resource Dictionary', prompt: 'New XAML file beside the current view',
            value: target.suggestedFile,
            validateInput: name => /^[^/\\]+\.xaml$/i.test(name.trim()) ? undefined : 'Enter a sibling .xaml file name.',
        });
        if (fileName === undefined) return;
        const plan = await this.plan(site, fileName.trim());
        const source = site.document.getText();
        const change = singleChange(site, plan);
        const start = site.document.offsetAt(change.range.start);
        const end = site.document.offsetAt(change.range.end);
        await this.review.open(path.basename(site.document.fileName), source, source.slice(0, start) + change.newText + source.slice(end), plan,
            validate => this.apply(site, plan.file.fileName, plan.review, undefined, validate));
    }

    async inspect(site: DictionarySite, fileName: string | undefined, signal: AbortSignal): Promise<DictionaryPlan | DictionaryTarget> {
        const result = fileName === undefined ? await this.target(site) : await this.plan(site, fileName);
        signal.throwIfAborted();
        return result;
    }

    async apply(site: DictionarySite, fileName: string, review: string, signal?: AbortSignal, validate?: () => void): Promise<DictionaryPlan> {
        const plan = await this.plan(site, fileName);
        if (plan.review !== review) throw new Error('The extraction changed. Review the current patch before applying it.');
        const folder = requireCurrent(site);
        const destination = await newWorkspaceFile(path.join(path.dirname(site.document.uri.fsPath), fileName), folder.uri.toString());
        const expected = vscode.Uri.parse(plan.file.uri, true).fsPath;
        const parent = await realpath(path.dirname(destination.fsPath));
        if (fileName !== plan.file.fileName || path.relative(path.join(parent, fileName), expected)) {
            throw new Error('The source directory changed. Review the extraction again.');
        }
        requireCurrent(site);
        validate?.();
        signal?.throwIfAborted();
        const change = singleChange(site, plan);
        const edit = new vscode.WorkspaceEdit();
        // Create first so a destination collision leaves the authored buffer intact.
        edit.createFile(destination, { contents: Buffer.from(plan.file.source) });
        edit.replace(site.document.uri, change.range, change.newText);
        if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not apply the extraction. Review the source and destination before retrying.');
        return { ...plan, file: { ...plan.file, uri: destination.toString() } };
    }

    private async target(site: DictionarySite): Promise<DictionaryTarget> {
        requireCurrent(site);
        const target = await this.server.dictionaryRequest<DictionaryTarget>('dictionaryExtractionTarget', params(site));
        requireCurrent(site);
        return target;
    }

    private async plan(site: DictionarySite, fileName: string): Promise<DictionaryPlan> {
        requireCurrent(site);
        const plan = await this.server.dictionaryRequest<DictionaryPlan>('planDictionaryExtraction', { ...params(site), fileName });
        requireCurrent(site);
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

function params(site: DictionarySite) { return { uri: site.document.uri.toString(), request: site.request }; }

function requireCurrent(site: DictionarySite): vscode.WorkspaceFolder {
    const { document, request } = site;
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (!folder || !vscode.workspace.isTrusted || document.uri.scheme !== 'file' || document.languageId !== xamlLanguage) {
        throw new Error('Open an authored XAML view in a trusted workspace to extract its resources.');
    }
    site.validate?.();
    if (document.isClosed || document.version !== request.version) throw new Error('The XAML changed. Prepare the extraction again.');
    return folder;
}

function singleChange(site: DictionarySite, plan: DictionaryPlan): vscode.TextEdit {
    const [edit] = plan.edit.edits;
    if (!edit || plan.edit.edits.length !== 1 || plan.edit.version !== site.request.version) {
        throw new Error('The checked extraction patch does not match the current source.');
    }
    return new vscode.TextEdit(exactRange(site.document, edit.range), edit.newText);
}

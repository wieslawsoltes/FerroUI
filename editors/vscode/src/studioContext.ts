import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { sourceColumn } from './editorHistory';
import type { EditResult, Range } from './protocol';
import { includedSourceUri, type IncludedSource } from './includedSource';
import type { IncludedTemplateSource } from './includedTemplate';
import { revision } from './reloadProcess';
import { exactRange } from './textDocument';

export class StudioContext {
    readonly selection: number[];

    constructor(readonly panel: DesignerPanel, readonly token: number, private readonly isActive: () => boolean) {
        this.selection = [...panel.selectedIds];
    }

    get node(): number | undefined { return this.selection.at(-1) ?? this.panel.nodes[0]?.id; }
    get writable(): boolean { return this.panel.language === 'xaml'; }

    ensure(): void {
        const current = this.panel.selectedIds;
        if (!this.isActive() || this.panel.currentToken !== this.token || current.length !== this.selection.length
            || current.some((id, index) => id !== this.selection[index])) {
            throw new Error('The source or selection changed. Reopen the Studio action.');
        }
    }

    async read<T>(name: string, arguments_: object = {}): Promise<T> {
        this.ensure();
        const data = await this.panel.studioQuery(name, arguments_, this.selection, AbortSignal.timeout(15_000));
        this.ensure();
        return data as T;
    }

    async edit(name: string, arguments_: object): Promise<void> {
        await this.editResult(name, arguments_);
    }

    editResult(name: string, arguments_: object): Promise<EditResult> {
        this.ensure();
        return this.panel.studioEdit(this.token, name, arguments_, this.selection, AbortSignal.timeout(15_000), () => this.ensure());
    }

    async present(name: string, arguments_: object): Promise<void> {
        this.ensure();
        await this.panel.studioPresent(this.token, name, arguments_, this.selection, AbortSignal.timeout(15_000));
    }

    async reveal(node: number): Promise<void> {
        this.ensure();
        const entry = this.panel.nodes.find(item => item.id === node);
        if (entry) await this.panel.revealSource({ session: this.panel.session, token: this.token }, entry.nameRange);
    }

    async revealRange(range: Range): Promise<void> {
        this.ensure();
        await this.panel.revealSource({ session: this.panel.session, token: this.token }, range);
    }

    async revealRustRange(range: Range, uri?: string): Promise<void> {
        if (!uri) return this.revealRange(range);
        this.ensure();
        await this.panel.revealRustModule({ session: this.panel.session, token: this.token }, uri, range,
            AbortSignal.timeout(15_000), () => this.ensure());
    }

    async revealIncludedRange(source: IncludedSource, acceptedRevision: string): Promise<void> {
        this.ensure();
        const uri = await includedSourceUri(this.panel.document.uri, source);
        const document = await vscode.workspace.openTextDocument(uri);
        this.ensure();
        const text = document.getText();
        const bom = text.startsWith('\uFEFF');
        const normalized = bom ? text.slice(1) : text;
        if (revision(Buffer.from(normalized)) !== acceptedRevision) {
            throw new Error('The included source changed. Reopen the value trace.');
        }
        const withBom = ({ line, character }: Range['start']) => ({ line, character: character + Number(bom && line === 0) });
        const selection = exactRange(document, { start: withBom(source.range.start), end: withBom(source.range.end) });
        const viewColumn = sourceColumn(this.panel.document)
            ?? (this.panel.panel.viewColumn === vscode.ViewColumn.One ? vscode.ViewColumn.Beside : vscode.ViewColumn.One);
        await vscode.window.showTextDocument(document, { selection, viewColumn, preview: false });
    }

    async revealIncludedTemplate(source: IncludedTemplateSource): Promise<void> {
        this.ensure();
        const uri = await includedSourceUri(this.panel.document.uri, source);
        const document = await vscode.workspace.openTextDocument(uri);
        this.ensure();
        const selection = exactRange(document, source.range);
        const kinds = source.property === 'Template' ? ['ControlTemplate'] : ['DataTemplate', 'TreeDataTemplate'];
        if (!kinds.includes(document.getText(selection).split(':').at(-1) ?? '')) {
            throw new Error('The template declaration changed. Reopen the template editor.');
        }
        await vscode.window.showTextDocument(document, { selection, preview: false });
    }
}

export interface StudioItem {
    id: string;
    label: string;
    description?: string;
    icon?: string;
    children?: () => Promise<StudioItem[]>;
    run?: () => Promise<void>;
}

export function action(id: string, label: string, run: () => Promise<void>, icon?: string): StudioItem {
    return { id, label, run, icon };
}

export function editText(prompt: string, value?: string, validateInput?: (input: string) => string | undefined,
    title = 'FerroUI Studio'): Thenable<string | undefined> {
    return vscode.window.showInputBox({ title, prompt, value, validateInput, ignoreFocusOut: true });
}

export async function editTime(prompt: string, maximum: number, initial: number, title?: string): Promise<number | undefined> {
    const value = await editText(`${prompt} (0–${maximum})`, String(initial), undefined, title);
    if (value === undefined) return;
    const time = value.trim() ? Number(value) : Number.NaN;
    if (!Number.isFinite(time) || time < 0 || time > maximum) throw new Error(`Enter a time from 0 to ${maximum} seconds.`);
    return time;
}

export async function choose<T extends vscode.QuickPickItem>(title: string, items: readonly T[]): Promise<T | undefined> {
    return vscode.window.showQuickPick(items, { title, matchOnDescription: true });
}

import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';

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
        this.ensure();
        if (!this.writable) throw new Error('This Studio action is available in XAML.');
        await this.panel.studioEdit(this.token, name, arguments_, this.selection, AbortSignal.timeout(15_000));
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

export function editText(prompt: string, value?: string): Thenable<string | undefined> {
    return vscode.window.showInputBox({ title: 'FerroUI Studio', prompt, value, ignoreFocusOut: true });
}

export async function choose<T extends vscode.QuickPickItem>(title: string, items: readonly T[]): Promise<T | undefined> {
    return vscode.window.showQuickPick(items, { title, matchOnDescription: true });
}

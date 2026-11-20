import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import type { ProjectDefinition, ProjectLibrary } from './projectLibrary';
import { componentKind } from './studioLibrary';
import { localFolders } from './workspacePaths';

export interface DefinitionChoice extends vscode.QuickPickItem {
    open?: () => Promise<void>;
}

export function findLibraryDefinition(library: ProjectLibrary, current: DefinitionChoice[], open: (entry: ProjectDefinition) => Promise<void>) {
    const picker = vscode.window.createQuickPick<DefinitionChoice>();
    picker.title = 'Find Definition in Designer';
    picker.placeholder = 'Search by name, type, or file';
    const folders = localFolders();
    let pending: AbortController | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const query = async (value: string, request: AbortController) => {
        const terms = value.toLowerCase().trim().split(/\s+/);
        const local = current.filter(item => terms.every(term => `${item.label} ${item.description ?? ''}`.toLowerCase().includes(term)));
        const pages = await Promise.all(folders.map(async folder => {
            const heading = { label: folder.name, kind: vscode.QuickPickItemKind.Separator };
            try {
                const page = await library.list(folder.uri.toString(), value, 0, 50, AbortSignal.any([request.signal, AbortSignal.timeout(15_000)]));
                if (!page.entries.length) return [];
                const results: DefinitionChoice[] = page.entries.map(entry => ({
                    label: entry.definition.name,
                    description: `${componentKind(entry.definition.kind)} · ${entry.path}`,
                    detail: entry.definition.targetType,
                    alwaysShow: true,
                    open: () => open(entry),
                }));
                if (page.total > results.length) results.push({ label: 'Type more to narrow the results', alwaysShow: true });
                return [heading, ...results];
            } catch (error) {
                return [heading, { label: errorMessage(error), alwaysShow: true }];
            }
        }));
        if (request.signal.aborted) return;
        const items = [
            ...(local.length ? [{ label: 'Current document', kind: vscode.QuickPickItemKind.Separator }, ...local] : []),
            ...pages.flat(),
        ];
        picker.items = items.length ? items : [{ label: 'No matching definitions', alwaysShow: true }];
        picker.busy = false;
    };
    const schedule = () => {
        clearTimeout(timer);
        pending?.abort();
        const request = new AbortController();
        pending = request;
        const value = picker.value;
        picker.busy = true;
        picker.items = [];
        timer = setTimeout(() => { void query(value, request); }, 125);
    };
    return new Promise<DefinitionChoice | undefined>(resolve => {
        let selected: DefinitionChoice | undefined;
        const subscriptions = [
            picker.onDidChangeValue(schedule),
            picker.onDidAccept(() => {
                const target = picker.selectedItems[0];
                if (target?.open) {
                    selected = target;
                    picker.hide();
                }
            }),
            picker.onDidHide(() => {
                clearTimeout(timer);
                pending?.abort();
                subscriptions.forEach(subscription => subscription.dispose());
                picker.dispose();
                resolve(selected);
            }),
        ];
        schedule();
        picker.show();
    });
}

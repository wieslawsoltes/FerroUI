import * as vscode from 'vscode';
import type { Range } from './protocol';

export interface DesignTraceSource {
    range: Range;
    text: string | null;
}

type TraceState = 'effective' | 'base' | 'overridden' | 'inactive' | 'unset';

export interface PropertyTrace<T> {
    property: string;
    effective: string;
    authored?: T | null;
    resource?: T | null;
    contributors: { state: TraceState; label: string; priority: string; value: string | null; source: T | null; binding: boolean }[];
}

interface TraceItem<T> extends vscode.QuickPickItem {
    source?: T;
}

const states: Record<TraceState, [icon: string, label: string]> = {
    effective: ['check', 'Effective'],
    base: ['debug-pause', 'Animation base'],
    overridden: ['layers', 'Overridden'],
    inactive: ['circle-slash', 'Inactive'],
    unset: ['circle-outline', 'Unset'],
};

export async function showPropertyTrace<T>(
    trace: PropertyTrace<T>,
    reveal: (source: T) => Promise<void>,
    sourceText: (source: T) => string | null = () => null,
): Promise<void> {
    const items: TraceItem<T>[] = trace.contributors.map(entry => {
        const [icon, state] = states[entry.state] ?? ['layers', 'Contributor'];
        const source = entry.source ?? undefined;
        return {
            label: `$(${icon}) ${entry.label}`,
            description: entry.value === null || entry.value === '' ? state : `${state} · ${entry.value}`,
            detail: [entry.priority, entry.binding ? 'Binding or resource' : undefined, source ? sourceText(source) : undefined]
                .filter(Boolean).join(' · '),
            source,
            buttons: source ? [{ iconPath: new vscode.ThemeIcon('go-to-file'), tooltip: 'Reveal in source' }] : undefined,
        };
    });
    if (trace.authored) {
        items.unshift({ label: '$(code) Authored value', description: sourceText(trace.authored) ?? 'Property element',
            source: trace.authored, buttons: [{ iconPath: new vscode.ThemeIcon('go-to-file'), tooltip: 'Reveal in source' }] });
    }
    if (trace.resource) {
        items.push({ label: '$(library) Referenced resource', source: trace.resource,
            buttons: [{ iconPath: new vscode.ThemeIcon('go-to-file'), tooltip: 'Reveal in source' }] });
    }
    const source = await new Promise<T | undefined>(resolve => {
        const picker = vscode.window.createQuickPick<TraceItem<T>>();
        picker.title = `${trace.property} · ${trace.effective}`;
        picker.placeholder = 'Inspect value contributors or select a source location';
        picker.matchOnDescription = true;
        picker.matchOnDetail = true;
        picker.items = items;
        let target: T | undefined;
        const choose = (item?: TraceItem<T>) => {
            if (item?.source === undefined) return;
            target = item.source;
            picker.hide();
        };
        picker.onDidAccept(() => choose(picker.selectedItems[0]));
        picker.onDidTriggerItemButton(({ item }) => choose(item));
        picker.onDidHide(() => { picker.dispose(); resolve(target); });
        picker.show();
    });
    if (source !== undefined) await reveal(source);
}

import * as vscode from 'vscode';
import type { Range } from './protocol';
import type { StageAccessibleNode, StageChecks } from './stagesProtocol';
import { choose } from './studioContext';

type CheckItem = vscode.QuickPickItem & { range?: Range | null };

export async function pickStageCheck(name: string, { findings, accessibility }: StageChecks): Promise<Range | undefined> {
    const issues = findings.map<CheckItem>(finding => ({
        label: `$(${finding.code === 'ineffectiveLayoutValue' ? 'info' : 'warning'}) ${finding.summary ?? finding.message}`,
        description: finding.name ?? finding.control,
        detail: finding.summary ? finding.message : undefined,
        range: finding.range,
    }));
    if (!issues.length && !accessibility) {
        void vscode.window.showInformationMessage('No findings in this stage.');
        return;
    }
    const items: CheckItem[] = [separator('Findings'), ...(issues.length ? issues : [{ label: '$(check) No findings' }])];
    if (accessibility) {
        items.push(separator('Tab order'), ...accessibility.tab_order.map((row, index) => accessible(row, `${index + 1}. `)));
        if (!accessibility.tab_order.length) items.push({ label: 'No Tab stops' });
        items.push(separator('Accessibility tree'), ...accessibility.tree.map(row => accessible(row, '\u2002'.repeat(Math.min(row.depth, 12)))));
        if (accessibility.truncated) items.push({ label: '$(info) Preview exceeded the inspection limit' });
    }
    return (await choose(`Checks · ${name}`, items))?.range ?? undefined;
}

function separator(label: string): CheckItem {
    return { label, kind: vscode.QuickPickItemKind.Separator };
}

function accessible(row: StageAccessibleNode, prefix: string): CheckItem {
    return {
        label: `${prefix}${row.name || row.control}`,
        description: [row.role, row.enabled ? undefined : 'Disabled', row.offscreen ? 'Offscreen' : undefined].filter(Boolean).join(' · '),
        range: row.range,
    };
}

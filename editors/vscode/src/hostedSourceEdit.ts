import type * as vscode from 'vscode';
import type { HostedSourcePatch } from './hostedAnimationProtocol';
import { applyTextEdits } from './textDocument';

export async function applyHostedSourcePatches(document: vscode.TextDocument, version: number, patches: HostedSourcePatch[]): Promise<void> {
    const source = document.getText();
    let previous = 0;
    const edits = patches.map(({ start, end, expected, replacement }) => {
        if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < previous || end < start
            || end > source.length || source.slice(start, end) !== expected) {
            throw new Error('The App Preview patch no longer matches the source.');
        }
        previous = end;
        const from = document.positionAt(start);
        const to = document.positionAt(end);
        if (document.offsetAt(from) !== start || document.offsetAt(to) !== end) {
            throw new Error('The App Preview patch has an invalid editor range.');
        }
        return { range: { start: from, end: to }, newText: replacement };
    });
    await applyTextEdits(document, version, edits);
}

import * as vscode from 'vscode';
import type { HostedAnimationProgram, HostedKeyframePlan } from './hostedAnimationProtocol';
import { applyTextEdits } from './textDocument';

export async function chooseHostedKeyframe(program: HostedAnimationProgram) {
    const authoring = program.authoring;
    if (!authoring) throw new Error('Choose an animation with a registered project codec.');
    const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 7 });
    return vscode.window.showQuickPick(authoring.tracks.flatMap(track => track.keyframes.map(keyframe => ({
        label: track.property,
        description: `${seconds.format(keyframe.time)} s`,
        detail: keyframe.value_text,
        target: track.target,
        keyframe,
    }))), { title: `Edit keyframe · ${program.name}`, matchOnDescription: true, matchOnDetail: true });
}

export async function applyHostedKeyframePatch(document: vscode.TextDocument, version: number, patch: NonNullable<HostedKeyframePlan['patch']>) {
    const source = document.getText();
    const { start, end, expected, replacement } = patch;
    if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0 || end < start
        || end > source.length || source.slice(start, end) !== expected) {
        throw new Error('The animation patch no longer matches the source.');
    }
    const from = document.positionAt(start);
    const to = document.positionAt(end);
    if (document.offsetAt(from) !== start || document.offsetAt(to) !== end) {
        throw new Error('The animation patch has an invalid editor range.');
    }
    await applyTextEdits(document, version, [{ range: { start: from, end: to }, newText: replacement }]);
}

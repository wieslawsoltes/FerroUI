import * as vscode from 'vscode';
import type { HostedAnimationProgram } from './hostedAnimationProtocol';

export async function chooseHostedKeyframe(program: HostedAnimationProgram) {
    const authoring = program.authoring;
    if (!authoring) throw new Error('Choose an editable animation in the current application preview.');
    const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 7 });
    return vscode.window.showQuickPick(authoring.tracks.flatMap(track => track.keyframes.map(keyframe => ({
        label: track.property,
        description: `${seconds.format(keyframe.time)} s`,
        detail: keyframe.value_text,
        target: track.target,
        keyframe,
    }))), { title: `Edit keyframe · ${program.name}`, matchOnDescription: true, matchOnDetail: true });
}

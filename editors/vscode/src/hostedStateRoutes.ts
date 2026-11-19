import * as vscode from 'vscode';
import type { HostedAnimationProgram, HostedStateDraft } from './hostedAnimationProtocol';

export async function editHostedStateRoute(program: HostedAnimationProgram): Promise<HostedStateDraft | undefined> {
    const route = program.stateRoute;
    if (!route) throw new Error('Choose a mapped Rust state route.');
    const title = program.name;
    const duration = await vscode.window.showInputBox({
        title, prompt: 'Duration in milliseconds', value: String(Math.round(program.duration * 1000)),
        validateInput: value => /^\d+$/.test(value.trim()) && Number.isSafeInteger(Number(value)) ? undefined : 'Enter a whole number of milliseconds.',
    });
    if (duration === undefined) return;
    const easing = await vscode.window.showInputBox({
        title, prompt: 'Easing name or four Bézier coordinates', value: route.easing,
    });
    if (easing !== undefined) return { program: program.id, duration: Number(duration) / 1000, easing };
}

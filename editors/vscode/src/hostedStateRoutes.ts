import type { HostedAnimationProgram, HostedStateDraft } from './hostedAnimationProtocol';
import { editText, editTime } from './studioContext';

export async function editHostedStateRoute(program: HostedAnimationProgram, rust: boolean): Promise<HostedStateDraft | undefined> {
    const route = program.stateRoute;
    if (!route || route.endpoint) throw new Error('Choose an authored state transition.');
    const title = program.name;
    const milliseconds = rust ? await editText('Duration in milliseconds', String(Math.round(program.duration * 1000)),
        value => /^\d+$/.test(value.trim()) && Number.isSafeInteger(Number(value)) ? undefined : 'Enter a whole number of milliseconds.', title) : undefined;
    const duration = rust ? milliseconds === undefined ? undefined : Number(milliseconds) / 1000
        : await editTime('Duration in seconds', 600, program.duration, title);
    if (duration === undefined) return;
    const easing = await editText('Transition easing', route.easing, undefined, title);
    if (easing !== undefined) return { program: program.id, duration, easing };
}

export type RuntimeKind = 'managed' | 'attached' | 'simulator';
export type AppRuntime = `${RuntimeKind}:${string}`;
export type RuntimeTarget = 'managed' | 'attached' | AppRuntime;

export interface IosRuntimeTarget {
    device: string;
    name: string;
    system: string;
    profile: string;
    bundleId: string;
}

export function runtimeKind(target: RuntimeTarget): RuntimeKind {
    if (target === 'managed' || target.startsWith('managed:')) return 'managed';
    return target.startsWith('simulator:') ? 'simulator' : 'attached';
}

export const runtimeArgument = { type: 'string', maxLength: 64 };

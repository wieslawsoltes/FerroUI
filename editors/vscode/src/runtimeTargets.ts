export type RuntimeKind = 'managed' | 'attached';
export type AppRuntime = `${RuntimeKind}:${string}`;
export type RuntimeTarget = RuntimeKind | AppRuntime;

export function runtimeKind(target: RuntimeTarget): RuntimeKind {
    return target === 'managed' || target.startsWith('managed:') ? 'managed' : 'attached';
}

export const runtimeArgument = { type: 'string', maxLength: 64 };

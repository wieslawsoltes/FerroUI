export type RuntimeKind = 'managed' | 'attached' | 'simulator' | 'android' | 'browser';
export type AppRuntime = `${RuntimeKind}:${string}`;
export type RuntimeTarget = 'managed' | 'attached' | AppRuntime;

export interface IosRuntimeTarget {
    device: string;
    name: string;
    system: string;
    profile: string;
    bundleId: string;
}

export interface AndroidRuntimeTarget {
    device: string;
    name: string;
    emulator: boolean;
    profile: string;
    packageId: string;
    launched: boolean;
    pid: number;
}

export interface BrowserRuntimeTarget {
    folder: string;
    profile: string;
    url: string;
    launched: boolean;
}

export type PlatformRuntime = { kind: 'simulator'; simulator: IosRuntimeTarget }
    | { kind: 'android'; android: AndroidRuntimeTarget } | { kind: 'browser'; browser: BrowserRuntimeTarget };

export function runtimeKind(target: RuntimeTarget): RuntimeKind {
    if (target === 'managed' || target.startsWith('managed:')) return 'managed';
    if (target.startsWith('android:')) return 'android';
    if (target.startsWith('browser:')) return 'browser';
    return target.startsWith('simulator:') ? 'simulator' : 'attached';
}

export const runtimeArgument = { type: 'string', maxLength: 64 };

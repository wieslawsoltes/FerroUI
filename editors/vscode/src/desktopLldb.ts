import type { DebugConfiguration } from 'vscode';

const useLldbServer = 'LLDB_USE_LLDB_SERVER';

export function configureDesktopLldb(configuration: DebugConfiguration, workspaceEnvironment: () => unknown,
    host: NodeJS.Platform = process.platform): void {
    if (host !== 'win32') return;
    const override: unknown = configuration.windows;
    const windows = override && typeof override === 'object' && !Array.isArray(override)
        ? override as Record<string, unknown> : undefined;
    const effective = { ...configuration, ...windows };
    if (effective.type !== 'lldb-dap' || effective.request !== 'attach' || effective.debugAdapterPort) return;
    const environment: unknown = effective.debugAdapterEnv || workspaceEnvironment() || {};
    if (typeof environment !== 'object' || !environment) return;

    // The Windows lldb-server backend can end the attached process even after
    // acknowledging a non-terminating detach. Select the local process plugin.
    const target = windows ?? configuration;
    const isBackend = (name: string) => name.toUpperCase() === useLldbServer;
    target.debugAdapterEnv = Array.isArray(environment)
        ? [...environment.filter(entry => typeof entry !== 'string' || !isBackend(entry.split('=', 1)[0])),
            `${useLldbServer}=0`]
        : { ...Object.fromEntries(Object.entries(environment).filter(([name]) => !isBackend(name))), [useLldbServer]: '0' };
}

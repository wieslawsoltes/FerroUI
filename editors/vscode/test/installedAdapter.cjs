const vscode = require('vscode');

exports.activate = context => {
    context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory('lldb-dap', {
        createDebugAdapterDescriptor(session, executable) {
            if (!executable) throw new Error('The installed test is missing its native LLDB adapter.');
            const configured = session.configuration.debugAdapterEnv;
            const env = Array.isArray(configured) ? Object.fromEntries(configured.map(entry => {
                const separator = entry.indexOf('=');
                return separator < 0 ? [entry, ''] : [entry.slice(0, separator), entry.slice(separator + 1)];
            })) : configured;
            return new vscode.DebugAdapterExecutable(executable.command, executable.args,
                { ...executable.options, env: { ...executable.options?.env, ...env } });
        },
    }));
};

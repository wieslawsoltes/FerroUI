import { createHash } from 'node:crypto';
import type { RuntimeClient } from './runtimeClient';
import type { RuntimeSource } from './runtimeProtocol';

interface RuntimeTemplate {
    member: string;
    column?: { index: number; header: string | null; expander_inner: boolean };
    source: RuntimeSource | null;
}

export async function readRuntimeTemplates(client: RuntimeClient, node: number, signal?: AbortSignal) {
    if (!await client.supports('get_template_sources')) throw new Error('Rebuild the app with runtime template inspection support.');
    const templates = await client.call<RuntimeTemplate[]>('get_template_sources', { node_id: node }, signal);
    return templates.map(template => {
        const { column, member, source } = template;
        const identity = JSON.stringify([
            member, column?.index, column?.expander_inner,
            source?.uri ?? (source?.workspacePath && [source.workspacePath, source.manifestPath]), source?.line, source?.column, source?.revision,
        ]);
        return { ...template, id: createHash('sha256').update(identity).digest('hex') };
    });
}

export function runtimeTemplateLabel(template: RuntimeTemplate) {
    return template.member.replace(/[A-Z]/g, (letter, offset) => offset ? ` ${letter.toLowerCase()}` : letter);
}

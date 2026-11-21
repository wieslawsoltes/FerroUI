import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedStyles } from './hostedStyles';
import type { HostedStyleCatalog, HostedStyleCommand, HostedStyleDefinition, HostedStyleResult, HostedStyleTarget } from './hostedStylesProtocol';
import { choose } from './studioContext';
import { chooseNewSetter, chooseNewStyle, chooseStyleResource } from './stylePrompts';

export async function runHostedStyleAction(styles: HostedStyles, frame: HostedAnimationFrame,
    command: HostedStyleCommand, current: () => HostedAnimationFrame): Promise<HostedStyleResult | undefined> {
    const signal = () => AbortSignal.timeout(15_000);
    const latest = () => {
        const fresh = current();
        if (fresh.revision !== frame.revision) throw new Error('The App Preview source changed while choosing a style action.');
        return fresh;
    };
    const assign = async (target: HostedStyleTarget, property: string) => {
        const { resources } = await styles.resources(latest(), target, property, signal());
        const selected = await chooseStyleResource(property, resources);
        if (selected) await styles.edit(latest(), target, { action: 'resource', property, key: selected.resource.key, dynamic: selected.dynamic }, true, signal());
    };
    switch (command.action) {
        case 'files': case 'catalog': case 'read': return styles.read(frame, command, signal());
        case 'locate': await styles.sources.locate(frame, command.address, command.range, signal()); return;
        case 'edit': await styles.edit(frame, command.target, command.edit, true, signal()); return;
        case 'create': {
            const { catalog } = await styles.read(frame, { action: 'catalog', address: command.address }, signal()) as HostedStyleCatalog;
            const kinds: { label: string; definitionKind: 'style' | 'theme' }[] = [];
            if (catalog.style_hosts.length) kinds.push({ label: 'Style', definitionKind: 'style' });
            if (catalog.palette_scopes.length) kinds.push({ label: 'Control theme', definitionKind: 'theme' });
            if (!kinds.length) throw new Error('Open a file with an authored style or resource host.');
            const kind = kinds.length === 1 ? kinds[0] : await choose('New App Preview definition', kinds);
            const result = kind && await chooseNewStyle(catalog, kind.definitionKind);
            if (result) await styles.edit(latest(), { address: command.address, node: result.node }, result.edit, true, signal());
            return;
        }
        case 'addSetter': {
            const { definition } = await styles.read(frame, { action: 'read', target: command.target }, signal()) as HostedStyleDefinition;
            const setter = await chooseNewSetter(definition.available);
            if (setter?.kind === 'resource') await assign(command.target, setter.property);
            if (setter?.kind === 'literal') await styles.edit(latest(), command.target, { action: 'setter', property: setter.property, value: setter.value }, true, signal());
            return;
        }
        case 'chooseResource': await assign(command.target, command.property);
    }
}

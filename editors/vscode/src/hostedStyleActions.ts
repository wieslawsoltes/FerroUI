import type { HostedAnimationFrame } from './hostedAnimationProtocol';
import type { HostedStyles } from './hostedStyles';
import type { HostedStyleCatalog, HostedStyleCommand, HostedStyleDefinition, HostedStyleResult, HostedStyleTarget } from './hostedStylesProtocol';
import { choose, editText } from './studioContext';
import type { RustStyleEdit } from './stylesProtocol';
import { chooseNewSetter, chooseNewStyle, chooseRustColorResource, chooseStyleIssue, chooseStyleResource, promptRustColorResource } from './stylePrompts';

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
    const rustEdit = (target: HostedStyleTarget, edit: RustStyleEdit) =>
        styles.edit(latest(), target, { action: 'rust', edit }, true, signal());
    switch (command.action) {
        case 'files': case 'catalog': case 'read': return styles.read(frame, command, signal());
        case 'locate': await styles.sources.locate(frame, command.address, command.range, signal()); return;
        case 'edit': await styles.edit(frame, command.target, command.edit, true, signal()); return;
        case 'issues': {
            const { catalog } = await styles.read(frame, { action: 'catalog', address: command.address }, signal()) as HostedStyleCatalog;
            const range = await chooseStyleIssue(catalog.issues ?? []);
            if (range) await styles.sources.locate(latest(), command.address, range, signal());
            return;
        }
        case 'create': {
            const { catalog } = await styles.read(frame, { action: 'catalog', address: command.address }, signal()) as HostedStyleCatalog;
            const kinds: { label: string; definitionKind: 'style' | 'theme' }[] = [];
            if (catalog.style_hosts.length) kinds.push({ label: 'Style', definitionKind: 'style' });
            if (catalog.palette_scopes.length) kinds.push({ label: 'Control theme', definitionKind: 'theme' });
            if (!kinds.length) throw new Error('Open a file with an authored style or resource host.');
            const kind = kinds.length === 1 ? kinds[0] : await choose('New App Preview definition', kinds);
            const result = kind && await chooseNewStyle(catalog, kind.definitionKind);
            if (result) {
                const target = { address: command.address, node: result.node };
                if (catalog.source_kind === 'rust' && result.edit.action === 'create') {
                    await rustEdit(target, { ...result.edit, owner: target.node });
                } else {
                    await styles.edit(latest(), target, result.edit, true, signal());
                }
            }
            return;
        }
        case 'addSetter': {
            const { definition } = await styles.read(frame, { action: 'read', target: command.target }, signal()) as HostedStyleDefinition;
            const setter = await chooseNewSetter(definition.available);
            if (setter?.kind === 'resource') {
                if (setter.getter) {
                    const choice = await chooseRustColorResource(setter.property, definition.resource_choices ?? []);
                    if (choice) await rustEdit(command.target, { action: 'set_resource', style: command.target.node, getter: setter.getter, key: choice.key });
                } else await assign(command.target, setter.property);
            }
            if (setter?.kind === 'literal') {
                if (setter.getter) await rustEdit(command.target, { action: 'add_setter', style: command.target.node, getter: setter.getter, value: setter.value });
                else await styles.edit(latest(), command.target, { action: 'setter', property: setter.property, value: setter.value }, true, signal());
            }
            return;
        }
        case 'addNested': {
            const selector = await editText('Selector', '^:pointerover', undefined, 'New Nested Rust Style');
            if (selector !== undefined) await rustEdit(command.target, { action: 'create_nested', style: command.target.node, selector });
            return;
        }
        case 'addColorResource': {
            const input = await promptRustColorResource();
            if (input) await rustEdit(command.target, { action: 'add_color_resource', style: command.target.node, ...input });
            return;
        }
        case 'chooseResource': {
            const { definition } = await styles.read(frame, { action: 'read', target: command.target }, signal()) as HostedStyleDefinition;
            if (definition.resource_choices) {
                const choice = await chooseRustColorResource(command.property, definition.resource_choices);
                if (choice) await rustEdit(command.target, { action: 'set_resource', style: command.target.node, setter: command.setter, key: choice.key });
            } else await assign(command.target, command.property);
        }
    }
}

import { strict as assert } from 'node:assert';
import { mkdtemp, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import type { BuiltProject } from '../src/builtProjects';
import type { HostedAuthoredPlan } from '../src/hostedSources';
import type { HostedFrame, HostedSource } from '../src/hostedProtocol';
import type { HostedStyleCatalog, HostedStyleDefinition, HostedStyleEdit, HostedStyleFiles, HostedStyleResources, HostedStyleTarget } from '../src/hostedStylesProtocol';
import type { NativePreview } from '../src/nativePreview';
import { authoredStyles, styleView, writeHostedStyleFixture } from './hostedStyleFixture';

export async function verifyHostedStyles(preview: NativePreview, update: (revision: number, xaml: string) => HostedSource) {
    const directory = await mkdtemp(join(tmpdir(), 'ferroui-hosted-styles-'));
    try {
        const root = await realpath(directory);
        await writeHostedStyleFixture(root);
        const manifest = join(root, 'Cargo.toml');
        await writeFile(manifest, '[package]\nname = "todo-workbench"\nversion = "0.0.0"\n');
        const project: BuiltProject = { workspaceRoot: root, metadata: { manifest, directories: [root], package: 'todo-workbench' } };
        const uri = pathToFileURL(join(root, 'Review.xaml')).toString();
        const styles = pathToFileURL(join(root, 'Styles', 'ReviewStyle.xaml')).toString();
        const render = async (revision: number, draft?: string) => {
            const request = update(revision, styleView);
            request.projectPath = '/Review.xaml';
            request.document = await preview.inProject({ ...request.document, uri }, project);
            const result = await preview.renderHostedWorkspace(request, { root, localSources: draft ? [{ uri: styles, source: draft }] : [] });
            assert.equal(result.error, null);
            assert.ok(result.frame?.stylesAvailable);
            return result.frame;
        };
        const initial = await render(3000);
        const { documents } = await preview.hostedStyles<HostedStyleFiles>(initial, { action: 'files' });
        const included = documents.find(file => file.uri === styles);
        assert.ok(included);
        const address = included.address;
        const { catalog } = await preview.hostedStyles<HostedStyleCatalog>(initial, { action: 'catalog', address });
        const style = catalog.definitions.find(entry => entry.kind === 'Style');
        const theme = catalog.definitions.find(entry => entry.kind === 'ControlTheme');
        assert.ok(style && theme);
        const target = { address, node: style.node };
        const themeTarget = { address, node: theme.node };
        const read = async (frame: HostedFrame, target: HostedStyleTarget) =>
            (await preview.hostedStyles<HostedStyleDefinition>(frame, { action: 'read', target })).definition;
        const plan = (frame: HostedFrame, target: HostedStyleTarget, edit: HostedStyleEdit, source = authoredStyles) =>
            preview.hostedStyles<HostedAuthoredPlan>(frame, { action: 'plan', target, edit, source });
        const getDraft = (plan: HostedAuthoredPlan, source = authoredStyles) => {
            assert.equal(plan.uri, styles);
            return plan.patches.reduceRight((text, patch) => {
                assert.equal(text.slice(patch.start, patch.end), patch.expected);
                return text.slice(0, patch.start) + patch.replacement + text.slice(patch.end);
            }, source);
        };

        const definition = await read(initial, target);
        assert.equal(definition.setters.find(setter => setter.property === 'Opacity')?.field.kind, 'number');
        assert.ok(definition.available.some(setter => setter.property === 'FontWeight'));
        await assert.rejects(plan(initial, target, { action: 'setter', property: 'Opacity', value: 'invalid' }));
        await assert.rejects(plan(initial, target, { action: 'selector', value: 'missing|PreviewBadge' }), /namespace/);
        const typed = getDraft(await plan(initial, target, { action: 'setter', property: 'Opacity', value: '0.64' }));
        assert.equal(typed, authoredStyles.replace('Value="1"', 'Value="0.64"'));
        const updated = await render(3001, typed);
        assert.notEqual(updated.image.png, initial.image.png);
        await assert.rejects(plan(initial, target, { action: 'remove' }), /current app-hosted preview frame/);
        await assert.rejects(plan(updated, target, { action: 'remove' }), /buffer changed/);
        const { resources } = await preview.hostedStyles<HostedStyleResources>(updated, { action: 'resources', target, property: 'Foreground' });
        assert.ok(resources.some(resource => resource.key === 'ReviewAccent'));
        assert.ok(resources.some(resource => resource.key === 'ReviewAccentAlias' && resource.type_name === 'SolidColorBrush'));
        const accented = getDraft(await plan(updated, target, { action: 'resource', property: 'Foreground', key: 'ReviewAccentAlias', dynamic: false }, typed), typed);
        assert.equal(accented, typed.replace('Value="#FF233F67"', `Value="{StaticResource 'ReviewAccentAlias'}"`));
        const colored = await render(3002, accented);
        assert.notEqual(colored.image.png, updated.image.png);

        const controlTheme = await read(colored, themeTarget);
        assert.equal(controlTheme.base, 'StandardMessage');
        assert.deepEqual(controlTheme.base_choices?.map(choice => choice.node), ['ApprovedMessage', 'StandardMessage']);
        await assert.rejects(plan(colored, themeTarget, { action: 'base', key: 'ReviewMessage' }, accented), /cycle/i);
        await assert.rejects(plan(colored, themeTarget, { action: 'base', key: 'WrongControl' }, accented), /ancestor/i);
        const based = getDraft(await plan(colored, themeTarget, { action: 'base', key: 'ApprovedMessage' }, accented), accented);
        assert.equal(based, accented.replace('StaticResource StandardMessage', "StaticResource 'ApprovedMessage'"));
        const approved = await render(3003, based);
        assert.notEqual(approved.image.png, colored.image.png);

        const short = getDraft(await plan(approved, target, { action: 'selector', value: 'app|PreviewBadge.missing' }, based), based);
        assert.notEqual((await render(3004, short)).image.png, approved.image.png);
        const restored = await render(3005);
        assert.equal(restored.image.png, initial.image.png);
        const owner = catalog.style_hosts.find(host => host.type_name === 'Styles');
        const dictionary = catalog.palette_scopes.find(scope => scope.label === 'ResourceDictionary');
        assert.ok(owner && dictionary);
        const selector = 'app|PreviewBadge:disabled';
        const inserted = getDraft(await plan(restored, { address, node: owner.node }, { action: 'create', selector }));
        const newStyle = await render(3006, inserted);
        const next = await preview.hostedStyles<HostedStyleCatalog>(newStyle, { action: 'catalog', address });
        const added = next.catalog.definitions.find(entry => entry.target === selector);
        assert.ok(added);
        const removed = getDraft(await plan(newStyle, { address, node: added.node }, { action: 'remove' }, inserted), inserted);
        assert.ok(!removed.includes(selector));
        const clean = await render(3007, removed);
        const withTheme = getDraft(await plan(clean, { address, node: dictionary.node }, {
            action: 'create_theme', key: 'NewReviewTheme', target_type: 'TextBlock',
        }, removed), removed);
        assert.match(withTheme, /NewReviewTheme/);
        assert.equal((await render(3008, withTheme)).image.png, initial.image.png);
    } finally { await rm(directory, { recursive: true, force: true }); }
}

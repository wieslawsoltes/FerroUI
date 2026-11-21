import { editText, type StudioContext } from './studioContext';
import { chooseNewSetter, chooseNewStyle } from './stylePrompts';
import type { RustStyleEdit, StylesAction, StylesContent } from './stylesProtocol';

export const RSX_STYLES = 'designer_rsx_styles';
export const RSX_STYLE_EDIT = 'designer_rsx_style_edit';

const editActions = new Set(['new', 'selector', 'setter', 'add-setter', 'add-nested-style', 'remove-style']);

export function isRustStyleAction(action: StylesAction): boolean { return editActions.has(action.type); }

export async function editRustStyle(context: StudioContext, content: StylesContent, action: StylesAction): Promise<string | undefined> {
    const edit = (args: RustStyleEdit) => context.edit(RSX_STYLE_EDIT, args);
    if (action.type === 'new') {
        const result = await chooseNewStyle(content.catalog, 'style', context.panel.selectedNode?.label);
        if (result?.edit.action !== 'create') return;
        await edit({ ...result.edit, owner: result.node });
        return result.edit.selector.trim();
    }
    const definition = content.definition;
    if (!definition) return;
    const style = definition.node;
    switch (action.type) {
        case 'selector':
            await edit({ action: 'selector', style, selector: action.value });
            return action.value.trim();
        case 'setter':
            await edit({ action: 'setter', style, setter: action.node, value: action.value });
            break;
        case 'add-setter': {
            const value = await chooseNewSetter(definition.available);
            if (value?.kind === 'literal' && value.getter) {
                await edit({ action: 'add_setter', style, getter: value.getter, value: value.value });
            }
            break;
        }
        case 'add-nested-style': {
            const selector = await editText('Selector', '^:pointerover', undefined, 'New Nested Rust Style');
            if (selector === undefined) break;
            await edit({ action: 'create_nested', style, selector });
            return selector.trim();
        }
        case 'remove-style':
            await edit({ action: 'remove', style });
            break;
    }
}

import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, viewportTheme, withViewportTheme } from './protocol';
import { addPaletteColor, addStyleSetter, assignStyleResource, createStyle } from './stylesActions';
import { chooseStyleIssue } from './stylePrompts';
import { choose, StudioContext } from './studioContext';
import { previewDefinition, showResourceReferences } from './studioLibrary';
import { editRustStyle, isRustStyleAction, RSX_STYLES } from './rsxStyleActions';
import type { FromStyles, PaletteSheet, StyleCatalog, StyleDefinition, StyleEditorLocation, StyleScope, StyleSummary, StylesAction, StylesPage, StylesState, ToStyles } from './stylesProtocol';
import { stylesHtml } from './view';

export const STYLES_VIEW = 'ferroui.styles';

interface Selection {
    epoch: number;
    definition?: StyleSummary;
    scope?: StyleScope;
    nextName?: string;
    requestedDefinition?: number;
    paletteFocus?: StylesState['paletteFocus'];
    consumer?: WeakRef<DesignerPanel>;
    page?: StylesState['page'];
    compare?: boolean;
}

export class StylesView {
    readonly view: DockView<StylesState, FromStyles, ToStyles>;
    private readonly selections = new WeakMap<DesignerPanel, Selection>();
    private panel?: DesignerPanel;
    private file = '';
    private key = '';
    private stamp = 0;
    private state: StylesState = { stamp: 0 };

    constructor(root: vscode.Uri, visible: () => void, private readonly returnTo: (panel: DesignerPanel) => Promise<void>) {
        this.view = new DockView(root, stylesHtml, () => {
            if (!this.view.visible) this.key = '';
            visible();
        }, message => {
            if (message.type === 'action') void this.receive(message.stamp, message.action);
        });
    }

    openDefinition(panel: DesignerPanel, node: number): void {
        const selection = this.selection(panel);
        selection.epoch++;
        selection.requestedDefinition = node;
        selection.page = 'styles';
        selection.compare = undefined;
        selection.paletteFocus = undefined;
        this.refresh(panel);
    }

    openSource(panel: DesignerPanel, editor: StyleEditorLocation, consumer: DesignerPanel): void {
        const selection = this.selection(panel);
        selection.epoch++;
        selection.consumer = new WeakRef(consumer);
        selection.page = editor.page;
        selection.compare = undefined;
        selection.requestedDefinition = editor.page === 'styles' ? editor.node : undefined;
        selection.paletteFocus = editor.page === 'palette' ? editor : undefined;
        this.refresh(panel);
    }

    showPage(panel: DesignerPanel, page: StylesPage): void {
        const selection = this.selection(panel);
        selection.epoch++;
        selection.page = page === 'comparison' ? 'palette' : page;
        selection.compare = page === 'comparison';
        selection.paletteFocus = undefined;
        this.refresh(panel);
    }

    update(panel: DesignerPanel | undefined, file: string): void {
        const changedPanel = panel !== this.panel;
        if (changedPanel) this.key = '';
        this.panel = panel;
        this.file = file;
        if (!this.view.visible) return;
        const token = panel?.currentToken;
        const available = panel && token !== undefined && !panel.status;
        const key = panel ? `${panel.session}:${token}:${panel.status ?? ''}:${panel.selectedIds.join(',')}` : 'empty';
        if (key === this.key) return;
        this.key = key;
        const stamp = ++this.stamp;
        if (!available || !(panel.language === 'xaml' ? panel.supportsStyleComposition : panel.supportsRustStyles)) {
            const message = !panel ? 'Open an XAML or RSX design.' : !available
                ? panel.status ?? 'Rendering the current design…' : 'Use a FerroUI helper with style composition support.';
            this.publish({ stamp, message, busy: Boolean(panel && !available),
                content: changedPanel ? undefined : this.state.content });
            return;
        }
        const selection = this.selection(panel);
        const context = new StudioContext(panel, token, () => this.panel === panel && this.stamp === stamp && this.view.visible);
        this.publish({ stamp, busy: true, theme: viewportTheme(panel.dimensions), content: changedPanel ? undefined : this.state.content });
        void this.read(context, stamp, selection);
    }

    private selection(panel: DesignerPanel): Selection {
        let value = this.selections.get(panel);
        if (!value) { value = { epoch: 0 }; this.selections.set(panel, value); }
        return value;
    }

    private async read(context: StudioContext, stamp: number, selection: Selection): Promise<void> {
        try {
            const panel = context.panel;
            const rust = panel.language === 'rust';
            const catalog = rust ? await context.read<StyleCatalog>(RSX_STYLES)
                : await context.read<StyleCatalog>('designer_style_catalog', context.node === undefined ? {} : { target: context.node });
            const named = selection.nextName ? catalog.definitions.filter(entry => entry.name === selection.nextName) : [];
            const definition = catalog.definitions.find(entry => entry.node === selection.requestedDefinition)
                ?? (named.length === 1 ? named[0] : undefined)
                ?? matching(catalog.definitions, selection.definition, (left, right) =>
                    left.name === right.name && left.kind === right.kind && left.scope === right.scope)
                ?? (!rust ? catalog.definitions.find(entry => entry.node === context.node) : undefined) ?? catalog.definitions[0];
            const scope = catalog.palette_scopes.find(scope => scope.node === selection.paletteFocus?.owner)
                ?? matching(catalog.palette_scopes, selection.scope, (left, right) => left.label === right.label)
                ?? catalog.palette_scopes.find(scope => scope.node === catalog.palette_owner) ?? catalog.palette_scopes[0];
            const [style, palette] = await Promise.all([
                definition && context.read<StyleDefinition>(rust ? RSX_STYLES : 'designer_style_definition', { node: definition.node }),
                scope && context.read<PaletteSheet>('designer_palette', { owner: scope.node }),
            ]);
            context.ensure();
            selection.definition = definition;
            selection.scope = scope;
            const page = selection.page;
            const compare = selection.compare;
            const paletteFocus = selection.paletteFocus;
            const consumer = selection.consumer?.deref();
            selection.nextName = undefined;
            selection.requestedDefinition = undefined;
            selection.paletteFocus = undefined;
            selection.page = undefined;
            selection.compare = undefined;
            this.publish({ stamp, content: { catalog, definition: style, palette }, theme: viewportTheme(panel.dimensions), preview: !rust, page, compare, paletteFocus,
                consumer: consumer && !consumer.isDisposed ? vscode.workspace.asRelativePath(consumer.document.uri) : undefined });
        } catch (error) {
            if (this.stamp === stamp) {
                this.publish({ ...this.state, busy: false, message: errorMessage(error) });
            }
        }
    }

    private async receive(stamp: number, action: StylesAction): Promise<void> {
        const panel = this.panel;
        const token = panel?.currentToken;
        const content = this.state.content;
        if (!panel || token === undefined || !content || this.state.busy || stamp !== this.stamp || !this.view.visible) return;
        const selection = this.selection(panel);
        const epoch = selection.epoch;
        // Applying a source edit temporarily marks the dock busy. Only changes
        // to the user's context retire the action, so it can survive its own busy state.
        const current = () => this.panel === panel && epoch === selection.epoch && this.view.visible;
        const context = new StudioContext(panel, token, current);
        try {
            context.ensure();
            if (panel.language === 'rust' && isRustStyleAction(action)) {
                const name = await editRustStyle(context, content, action);
                if (name) { selection.nextName = name; this.refresh(panel); }
                return;
            }
            const definition = content.definition;
            const palette = content.palette;
            switch (action.type) {
                case 'included-sources': {
                    const palettes = action.page === 'palette';
                    const choices = (content.catalog.included_sources ?? [])
                        .filter(choice => (choice.source.kind === 'palette') === palettes)
                        .map(choice => ({ ...choice, description: vscode.workspace.asRelativePath(vscode.Uri.parse(choice.source.uri)) }));
                    const chosen = await choose(palettes ? 'Open included palette' : 'Open included style or control theme', choices);
                    if (chosen) await context.present('designer_open_style_source', { source: chosen.source });
                    return;
                }
                case 'color-source': {
                    const color = palette?.comparison.find(row => row.key === action.key)?.[action.theme];
                    if (color?.source) await context.present('designer_open_style_source', { source: color.source });
                    else if (color?.range) await context.revealRange(color.range);
                    return;
                }
                case 'return-to-consumer': {
                    const consumer = selection.consumer?.deref();
                    if (!consumer || consumer.isDisposed) throw new Error('The consuming design is closed.');
                    await this.returnTo(consumer);
                    return;
                }
                case 'definition':
                    selection.epoch++;
                    selection.requestedDefinition = action.node;
                    this.refresh(panel);
                    return;
                case 'scope':
                    selection.epoch++;
                    selection.scope = content.catalog.palette_scopes.find(scope => scope.node === action.node);
                    this.refresh(panel);
                    return;
                case 'reveal': await context.revealRange(action.range); return;
                case 'style-issues': {
                    const range = await chooseStyleIssue(content.catalog.issues ?? []);
                    if (range) await context.revealRange(range);
                    return;
                }
                case 'add-nested-style': case 'add-color-resource': case 'rust-resource-color': return;
                case 'references': await showResourceReferences(context, action.node); return;
                case 'preview-theme':
                    if (action.theme === 'high_contrast' && !panel.supportsHighContrastTheme) return;
                    if (viewportTheme(panel.dimensions) !== action.theme) panel.configure(withViewportTheme(panel.dimensions, action.theme));
                    return;
                case 'new': {
                    const name = await createStyle(context, content.catalog, action.kind);
                    if (name) { selection.nextName = name; this.refresh(panel); }
                    return;
                }
                case 'palette-color':
                    if (!palette?.layers.some(layer => layer.colors.some(color => color.node === action.node && color.editable))) {
                        throw new Error('Choose a literal in the current palette.');
                    }
                    await context.edit('designer_palette_edit', { action: 'set', node: action.node, color: action.color });
                    return;
                case 'palette-add':
                    if (palette) await addPaletteColor(context, palette, action.layer);
                    return;
                default:
                    if (definition) await this.definitionAction(context, definition, selection, action);
            }
        } catch (error) {
            if (current()) this.view.post({ type: 'error', stamp: this.stamp, message: errorMessage(error) });
        }
    }

    private async definitionAction(context: StudioContext, sheet: StyleDefinition, selection: Selection,
        action: Extract<StylesAction, { type: 'selector' | 'base' | 'setter' | 'setter-resource' | 'add-setter' | 'preview-definition' | 'remove-style' }>): Promise<void> {
        const node = sheet.node;
        const edit = (args: object) => context.edit('designer_style_edit', args);
        switch (action.type) {
            case 'preview-definition': await previewDefinition(context, node); break;
            case 'add-setter': await addStyleSetter(context, sheet); break;
            case 'setter-resource': await assignStyleResource(context, node, action.property); break;
            case 'selector':
                await edit({ action: 'selector', node, selector: action.value });
                selection.nextName = action.value.trim();
                this.refresh(context.panel);
                break;
            case 'base': await edit({ action: 'base', node, base: action.base }); break;
            case 'setter': await edit({ action: 'setter', owner: node, property: action.property, value: action.value }); break;
            case 'remove-style': await edit({ action: 'remove', node }); break;
        }
    }

    private refresh(panel: DesignerPanel): void {
        if (this.panel !== panel) return;
        this.key = '';
        this.update(panel, this.file);
    }

    private publish(state: StylesState): void {
        this.state = state;
        this.view.show(state, this.file);
    }
}

function matching<T extends { node: number }>(items: T[], current: T | undefined, match: (item: T, saved: T) => boolean): T | undefined {
    if (current === undefined) return;
    const candidates = items.filter(item => match(item, current));
    return candidates.find(item => item.node === current.node) ?? (candidates.length === 1 ? candidates[0] : undefined);
}

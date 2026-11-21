import { canvasThemeLabels, type CanvasTheme, type Range } from './protocol';
import type { IncludedSource } from './includedSource';

export const paletteLayerLabels = { shared: 'Shared', default: 'Default', ...canvasThemeLabels } as const;
export type PaletteLayer = keyof typeof paletteLayerLabels;
export const contrastProviderLabels = { winui: 'WinUI', material: 'Material', macos: 'macOS' } as const;
export type ContrastProvider = keyof typeof contrastProviderLabels;
export type PaletteColorKind = 'color' | 'solid_color_brush';
export type StylesPage = 'styles' | 'palette' | 'comparison';
export type PaletteEdit = { action: 'set'; node: number; color: string }
    | { action: 'add'; owner: number; layer: PaletteLayer; key: string; kind: PaletteColorKind; color: string; provider?: ContrastProvider };

export type StyleCreation = { action: 'create'; selector: string }
    | { action: 'create_theme'; key: string; target_type: string };
export type RustColorMode = 'literal' | 'declaration' | 'local';
export type RustStyleEdit = { action: 'create'; owner: number; selector: string }
    | { action: 'create_nested' | 'selector'; style: number; selector: string }
    | { action: 'setter'; style: number; setter: number; value: string | null }
    | { action: 'add_setter'; style: number; getter: string; value: string }
    | { action: 'set_resource'; style: number; key: string } & ({ setter: number } | { getter: string })
    | { action: 'add_color_resource'; style: number; key: string; value: string; layer?: PaletteLayer }
    | { action: 'color_resource'; style: number; resource: number; value: string; mode?: RustColorMode }
    | { action: 'remove'; style: number };

export interface StyleScope { node: number; label: string }
export interface StyleHost extends StyleScope { type_name: string; nested: boolean }
export interface StyleSummary { node: number; name: string; kind: 'Style' | 'ControlTheme'; scope?: string; target?: string }
export interface StyleSource extends IncludedSource { kind: 'style' | 'control_theme' | 'palette' | 'color' }
export interface StyleSourceChoice { label: string; source: StyleSource }
export interface StyleNavigation { source: StyleSource; consumer: string }
export type StyleEditorLocation = { page: 'styles'; node: number }
    | { page: 'palette'; owner: number; layer: PaletteLayer; color: number | null };
export interface LocatedStyleSource { editor: StyleEditorLocation; range: Range }
export interface StyleCatalog {
    source_kind?: 'rust';
    definitions: StyleSummary[];
    style_hosts: StyleHost[];
    palette_scopes: StyleScope[];
    palette_owner?: number;
    included_sources?: StyleSourceChoice[];
    theme?: string;
    issues?: { reason: string; range: Range }[];
}
export interface SetterField { kind: 'text' | 'number' | 'color' | 'boolean' | 'choice'; options: string[]; resources: boolean; type_name?: string; owner?: string }
export interface StyleSetter { property: string; field: SetterField; getter?: string }
export interface AuthoredSetter extends StyleSetter {
    node: number;
    value?: string;
    range: Range;
    object: boolean;
    expression: boolean;
    editable: boolean;
    can_remove?: boolean;
    resource?: { key: string; declaration?: Range | null } | null;
}
export interface RustColorChoice { key: string; color?: string | null }
export interface RustColorResource extends RustColorChoice { node: number; range: Range; layer?: PaletteLayer | null; shared_constant?: string | null }
export interface StyleDefinition<Base extends string | number = number> {
    node: number;
    kind: StyleSummary['kind'];
    name: string;
    target?: string;
    range: Range;
    identity: { kind: 'selector' | 'base'; value?: string; editable: boolean };
    base?: Base;
    base_choices?: { node: Base; name: string; type_name?: string }[];
    setters: AuthoredSetter[];
    available: StyleSetter[];
    can_remove: boolean;
    nested_allowed?: boolean;
    color_resources?: RustColorResource[];
    resource_choices?: RustColorChoice[];
    resource_comparison?: PaletteComparisonRow[];
}
export interface PaletteColor {
    node: number;
    key: string;
    kind: PaletteColorKind;
    authored?: string;
    color?: string;
    editable: boolean;
    uses: number;
    range: Range;
}
export interface PaletteColumn { layer: PaletteLayer; declaration?: number | null; unavailable?: string; colors: PaletteColor[] }
export interface ResolvedColor {
    node?: number;
    value?: string | null;
    scope?: string | null;
    source?: StyleSource;
    sourceAddress?: string;
    range?: Range;
    path?: { key: string; reference?: 'StaticResource' | 'DynamicResource' }[];
    issue?: string;
}
export type PaletteComparisonRow = { key: string } & Partial<Record<CanvasTheme, ResolvedColor | null>>;
export interface PaletteSheet {
    owner: number;
    layers: PaletteColumn[];
    comparison: PaletteComparisonRow[];
    contrastPreview?: boolean;
    contrastProviders?: ContrastProvider[];
}
export interface StylesContent { catalog: StyleCatalog; definition?: StyleDefinition; palette?: PaletteSheet }
export interface StylesState {
    stamp: number;
    content?: StylesContent;
    theme?: CanvasTheme;
    preview?: boolean;
    busy?: boolean;
    message?: string;
    page?: 'styles' | 'palette';
    compare?: boolean;
    paletteFocus?: Extract<StyleEditorLocation, { page: 'palette' }>;
    consumer?: string;
}
export type StylesAction<Base extends string | number = number> =
    | { type: 'definition' | 'scope' | 'references'; node: number }
    | { type: 'new'; kind: 'style' | 'theme' }
    | { type: 'included-sources'; page: 'styles' | 'palette' }
    | { type: 'color-source'; key: string; theme: CanvasTheme }
    | { type: 'return-to-consumer' }
    | { type: 'selector'; value: string }
    | { type: 'base'; base: Base | null }
    | { type: 'setter'; property: string; value: string | null; node: number }
    | { type: 'setter-resource'; property: string; node: number }
    | { type: 'add-color-resource'; layer: PaletteLayer }
    | { type: 'rust-resource-color'; node: number; color: string }
    | { type: 'rust-shared-color'; node: number }
    | { type: 'add-setter' | 'preview-definition' | 'remove-style' }
    | { type: 'add-nested-style' | 'style-issues' }
    | { type: 'palette-add'; layer: PaletteLayer }
    | { type: 'palette-color'; node: number; color: string }
    | { type: 'preview-theme'; theme: CanvasTheme }
    | { type: 'reveal'; range: Range };
export type FromStyles = { type: 'ready' } | { type: 'action'; stamp: number; action: StylesAction };
export type ToStyles = { type: 'error'; stamp: number; message: string };

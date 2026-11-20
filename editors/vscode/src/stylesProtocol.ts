import type { Range } from './protocol';

export type PaletteLayer = 'shared' | 'default' | 'light' | 'dark';
export type PaletteColorKind = 'color' | 'solid_color_brush';

export interface StyleScope { node: number; label: string }
export interface StyleHost extends StyleScope { type_name: string; nested: boolean }
export interface StyleSummary { node: number; name: string; kind: 'Style' | 'ControlTheme'; scope?: string; target?: string }
export interface StyleCatalog {
    definitions: StyleSummary[];
    style_hosts: StyleHost[];
    palette_scopes: StyleScope[];
    palette_owner?: number;
    theme?: string;
}
export interface SetterField { kind: 'text' | 'number' | 'color' | 'boolean' | 'choice'; options: string[]; resources: boolean; type_name?: string }
export interface StyleSetter { property: string; field: SetterField }
export interface AuthoredSetter extends StyleSetter {
    node: number;
    value?: string;
    range: Range;
    object: boolean;
    expression: boolean;
    editable: boolean;
}
export interface StyleDefinition {
    node: number;
    kind: StyleSummary['kind'];
    name: string;
    target?: string;
    range: Range;
    identity: { kind: 'selector' | 'base'; value?: string; editable: boolean };
    base?: number;
    base_choices?: { node: number; name: string; type_name?: string }[];
    setters: AuthoredSetter[];
    available: StyleSetter[];
    can_remove: boolean;
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
export interface PaletteColumn { layer: PaletteLayer; declaration?: number; unavailable?: string; colors: PaletteColor[] }
export interface ResolvedColor { node: number; value?: string; scope?: string }
export interface PaletteSheet {
    owner: number;
    layers: PaletteColumn[];
    comparison: { key: string; light?: ResolvedColor; dark?: ResolvedColor }[];
}
export interface StylesContent { catalog: StyleCatalog; definition?: StyleDefinition; palette?: PaletteSheet }
export interface StylesState {
    stamp: number;
    content?: StylesContent;
    dark?: boolean;
    preview?: boolean;
    busy?: boolean;
    message?: string;
    page?: 'styles' | 'palette';
}
export type StylesAction =
    | { type: 'definition' | 'scope' | 'references'; node: number }
    | { type: 'new'; kind: 'style' | 'theme' }
    | { type: 'selector'; value: string }
    | { type: 'base'; base: number | null }
    | { type: 'setter'; property: string; value: string | null }
    | { type: 'setter-resource'; property: string }
    | { type: 'add-setter' | 'preview-definition' | 'remove-style' }
    | { type: 'palette-add'; layer: PaletteLayer }
    | { type: 'palette-color'; node: number; color: string }
    | { type: 'preview-theme'; dark: boolean }
    | { type: 'reveal'; range: Range };
export type FromStyles = { type: 'ready' } | { type: 'action'; stamp: number; action: StylesAction };
export type ToStyles = { type: 'error'; stamp: number; message: string };

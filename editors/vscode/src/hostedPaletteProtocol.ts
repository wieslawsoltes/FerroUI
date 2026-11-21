import type { HostedAnimationFrame, HostedSourcePlan } from './hostedAnimationProtocol';
import type { Range } from './protocol';
import type { PaletteEdit, PaletteLayer, PaletteSheet } from './stylesProtocol';

export interface HostedPaletteTarget { address: string; owner: number }
export interface HostedPaletteDocument { address: string; uri: string; scopes: { node: number; label: string }[] }
export interface HostedPaletteCatalog { documents: HostedPaletteDocument[] }
export interface HostedPaletteReport { uri: string; palette: PaletteSheet }
export interface HostedPalettePlan extends HostedSourcePlan { uri: string }
export interface HostedPaletteLocation { uri: string; range: Range }
export interface HostedPalettePresentation { target?: HostedPaletteTarget; layer?: PaletteLayer; compare?: boolean }

type PaletteRead = { action: 'catalog' } | { action: 'read'; target: HostedPaletteTarget };
type PaletteLocate = { action: 'locate'; address: string; range: Range };
type PaletteReferences = { action: 'references'; target: HostedPaletteTarget; node: number };
export type NativePaletteRequest = PaletteRead | PaletteReferences
    | { action: 'source'; address: string }
    | (PaletteLocate & { source: string })
    | { action: 'plan'; target: HostedPaletteTarget; edit: PaletteEdit; source: string };

export type HostedPaletteAction = PaletteRead | PaletteLocate | PaletteReferences
    | { action: 'edit'; target: HostedPaletteTarget; edit: PaletteEdit }
    | { action: 'add'; target: HostedPaletteTarget; layer: PaletteLayer };
export type FromHostedPalette = { type: 'palette'; request: number; frame: HostedAnimationFrame; command: HostedPaletteAction };
export type ToHostedPalette =
    | { type: 'paletteReply'; request: number; frame: HostedAnimationFrame; result?: HostedPaletteCatalog | HostedPaletteReport; error?: string }
    | { type: 'openPalette'; revision: number; presentation?: HostedPalettePresentation };

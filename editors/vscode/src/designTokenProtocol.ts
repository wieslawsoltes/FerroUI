export type TokenDirection = 'import' | 'export';
export interface TokenTransition { property: string; namespace?: string; class?: string }
export interface TokenMapping { key?: string; token?: string; kind?: string; xamlType?: string; transition?: TokenTransition }
export interface TokenOptions { mappings: Record<string, TokenMapping>; pixelsPerRem?: number }
export interface TokenRequest { direction: TokenDirection; source: string; options?: TokenOptions }
export interface TokenConversion { token: string; key: string; kind: string; xamlType: string; value: string; note?: string; transition?: TokenTransition }
export interface TokenIssue { path: string; message: string; kind?: string }
export interface TokenPlan { source: string; mappings: TokenConversion[]; issues: TokenIssue[] }
export interface TokenReviewState {
    type: 'review'; direction: TokenDirection; sourceName: string; mappingRows: TokenConversion[]; issues: TokenIssue[];
    mappingRevision: number; ready: boolean; busy: boolean; current: boolean; problem?: string;
}

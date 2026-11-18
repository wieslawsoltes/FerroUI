import type { Range } from './protocol';

export interface RsxProperty { setter: string; xamlName: string; valueType: string }
export interface RsxField {
    name: string;
    range: Range;
    property: RsxProperty | null;
    literal: boolean;
    shorthand: boolean;
    attached: boolean;
}
export interface RsxControl { class: string; name: Range; fields: RsxField[] }
export interface RsxLanguageDocument { nodes: RsxControl[]; properties: Record<string, RsxProperty[]> }

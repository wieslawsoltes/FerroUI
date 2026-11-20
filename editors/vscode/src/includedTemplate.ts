import type { IncludedSource } from './includedSource';

export interface IncludedTemplateSource extends IncludedSource {
    key: string;
    property?: 'Template';
}

export interface IncludedTemplatePreview {
    source: IncludedTemplateSource;
    sample: unknown;
    consumer: string;
    target_type?: string;
}

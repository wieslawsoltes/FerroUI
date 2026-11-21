import { integer, tool } from './agentSchema';

const session = { session: integer(1) };
const target = { ...session, expected_generation: integer(1), expected_revision: integer(1), expected_sequence: integer(1),
    node: integer(0) };

export const hostedTemplateCatalog = {
    designer_hosted_templates: tool('List source-backed control templates used when the app accepted this XAML preview. Use the generation, frame, and owner returned here for template actions.',
        session, Object.keys(session)),
    designer_hosted_template_source: tool('Open the template definition for the reported owner. Pass the catalog owner as node.',
        target, Object.keys(target), true, false),
    designer_hosted_template_copy_draft: tool('Preview the checked source patch for an inherited control template and its dependencies. Pass the catalog owner as node.',
        target, Object.keys(target)),
    designer_hosted_template_copy: tool('Install a private copy of an inherited control template into the previewed XAML buffer in one Undo step. Pass the catalog owner as node.',
        target, Object.keys(target), true, false),
};

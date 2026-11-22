import { choice, integer, tool } from './agentSchema';

const session = { session: integer(1) };
const target = { ...session, expected_generation: integer(1), expected_revision: integer(1), expected_sequence: integer(1),
    node: integer(0) };

export const hostedTemplateCatalog = {
    designer_hosted_templates: tool('List templates used by the app when it accepted this XAML or Rust preview. Use the returned generation, frame, owner and member for template actions.',
        session, Object.keys(session)),
    designer_hosted_template_source: tool('Open a template definition, its Rust assignment, or its Rust definition in Design. Pass the catalog owner as node and the member when reported.',
        { ...target,
            member: choice('ItemTemplate', 'SelectionBoxItemTemplate', 'MenuItemTemplate', 'CellTemplate', 'CellEditingTemplate', 'Template'),
            template_destination: choice('definition', 'assignment', 'design'),
        }, Object.keys(target), true, false),
    designer_hosted_template_copy_draft: tool('Preview the checked source patch for an inherited control template and its dependencies. Pass the catalog owner as node.',
        target, Object.keys(target)),
    designer_hosted_template_copy: tool('Install a private copy of an inherited control template into the previewed XAML buffer in one Undo step. Pass the catalog owner as node.',
        target, Object.keys(target), true, false),
};

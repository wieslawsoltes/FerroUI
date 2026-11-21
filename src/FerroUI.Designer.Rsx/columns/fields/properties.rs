use super::{Property, ValueCodec};
use ValueCodec::*;
use ferroui_designer_tools::ColumnKind;

const fn field(name: &'static str, setter: &'static str, encoding: ValueCodec) -> Property {
    Property {
        name,
        setter,
        encoding,
    }
}

const COMMON: &[Property] = &[
    field("Header", "header", Text),
    field("Width", "width", Length),
    field("CanUserResize", "can_user_resize", Boolean),
];

const TABLE: &[Property] = &[
    field("IsVisible", "is_visible", Boolean),
    field(
        "HorizontalContentAlignment",
        "horizontal_content_alignment",
        Choice("ferroui_base::layout::HorizontalAlignment"),
    ),
    field("CellTemplate", "cell_template", Expression),
    field("HeaderTemplate", "header_template", Expression),
    field("CellTheme", "cell_theme", Expression),
    field("HeaderTheme", "header_theme", Expression),
];

const TREE: &[Property] = &[
    field("MinWidth", "min_width", Length),
    field("MaxWidth", "max_width", Length),
    field("CanUserSortColumn", "can_user_sort_column", Boolean),
    field("AllowTriStateSorting", "allow_tri_state_sorting", Boolean),
];

const BINDING: &[Property] = &[
    field("Binding", "binding_path", BindingPath),
    field("Binding", "binding", Expression),
];

const TEXT: &[Property] = &[
    field("IsReadOnly", "is_read_only", Boolean),
    field("StringFormat", "string_format", Text),
    field(
        "TextAlignment",
        "text_alignment",
        Choice("ferroui_base::media::TextAlignment"),
    ),
    field(
        "TextWrapping",
        "text_wrapping",
        Choice("ferroui_base::media::TextWrapping"),
    ),
    field("IsTextSearchEnabled", "is_text_search_enabled", Boolean),
    field("TextTrimming", "text_trimming", Expression),
];

const CHECKBOX: &[Property] = &[
    field("IsReadOnly", "is_read_only", Boolean),
    field("IsThreeState", "is_three_state", Boolean),
];

const TEMPLATE: &[Property] = &[
    field("CellTemplate", "cell_template", Expression),
    field("CellEditingTemplate", "cell_editing_template", Expression),
    field(
        "CellTemplateResourceKey",
        "cell_template_resource_key",
        Expression,
    ),
    field(
        "CellEditingTemplateResourceKey",
        "cell_editing_template_resource_key",
        Expression,
    ),
    field("TextSearchBinding", "text_search_path", BindingPath),
    field("TextSearchBinding", "text_search_binding", Expression),
];

const EXPANDER: &[Property] = &[
    field("ChildrenBinding", "children_path", BindingPath),
    field("ChildrenBinding", "children_binding", Expression),
    field("HasChildrenBinding", "has_children_path", BindingPath),
    field("HasChildrenBinding", "has_children_binding", Expression),
    field("IsExpandedBinding", "is_expanded_path", BindingPath),
    field("IsExpandedBinding", "is_expanded_binding", Expression),
];

pub(super) fn of(kind: ColumnKind) -> impl Iterator<Item = Property> {
    let specific = match kind {
        ColumnKind::Table => TABLE,
        ColumnKind::Text => TEXT,
        ColumnKind::CheckBox => CHECKBOX,
        ColumnKind::Template => TEMPLATE,
        ColumnKind::Expander => EXPANDER,
        ColumnKind::RowHeader => &[],
    };
    let tree = if kind == ColumnKind::Table { &[] } else { TREE };
    let binding = if matches!(
        kind,
        ColumnKind::Table | ColumnKind::Text | ColumnKind::CheckBox
    ) {
        BINDING
    } else {
        &[]
    };
    COMMON
        .iter()
        .chain(tree)
        .chain(binding)
        .chain(specific)
        .copied()
}

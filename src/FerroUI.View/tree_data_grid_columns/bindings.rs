use super::TreeDataGridColumnBuilder;
use ferroui_base::{
    controls::ResourceKey,
    data::ReflectionBinding,
    media::{TextAlignment, TextTrimming, TextWrapping},
};
use ferroui_controls::{templates::IDataTemplate, AssignedBinding};
use ferroui_controls_treedatagrid::{
    TreeDataGridCheckBoxColumn, TreeDataGridHierarchicalExpanderColumn, TreeDataGridTemplateColumn,
    TreeDataGridTextColumn,
};
use std::rc::Rc;

impl TreeDataGridColumnBuilder<TreeDataGridTextColumn> {
    pub fn binding_path(self, path: impl AsRef<str>) -> Self {
        self.binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn binding(self, binding: AssignedBinding) -> Self {
        self.0.set_binding(Some(binding.0));
        self
    }

    pub fn is_read_only(self, read_only: bool) -> Self {
        self.0.set_is_read_only(read_only);
        self
    }

    pub fn is_text_search_enabled(self, enabled: bool) -> Self {
        self.0.set_is_text_search_enabled(enabled);
        self
    }

    pub fn string_format(self, format: impl Into<String>) -> Self {
        self.0.set_string_format(format);
        self
    }

    pub fn text_alignment(self, alignment: TextAlignment) -> Self {
        self.0.set_text_alignment(alignment);
        self
    }

    pub fn text_wrapping(self, wrapping: TextWrapping) -> Self {
        self.0.set_text_wrapping(wrapping);
        self
    }

    pub fn text_trimming(self, trimming: Rc<dyn TextTrimming>) -> Self {
        self.0.set_text_trimming(trimming);
        self
    }
}

impl TreeDataGridColumnBuilder<TreeDataGridCheckBoxColumn> {
    pub fn binding_path(self, path: impl AsRef<str>) -> Self {
        self.binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn binding(self, binding: AssignedBinding) -> Self {
        self.0.set_binding(Some(binding.0));
        self
    }

    pub fn is_read_only(self, read_only: bool) -> Self {
        self.0.set_is_read_only(read_only);
        self
    }

    pub fn is_three_state(self, enabled: bool) -> Self {
        self.0.set_is_three_state(Some(enabled));
        self
    }
}

impl TreeDataGridColumnBuilder<TreeDataGridTemplateColumn> {
    pub fn cell_template(self, template: Rc<dyn IDataTemplate>) -> Self {
        self.0.set_cell_template(Some(template));
        self
    }

    pub fn cell_editing_template(self, template: Rc<dyn IDataTemplate>) -> Self {
        self.0.set_cell_editing_template(Some(template));
        self
    }

    pub fn cell_template_resource_key(self, key: ResourceKey) -> Self {
        self.0.set_cell_template(None);
        self.0.set_cell_template_resource_key(Some(key));
        self
    }

    pub fn cell_editing_template_resource_key(self, key: ResourceKey) -> Self {
        self.0.set_cell_editing_template_resource_key(Some(key));
        self
    }

    pub fn text_search_path(self, path: impl AsRef<str>) -> Self {
        self.text_search_binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn text_search_binding(self, binding: AssignedBinding) -> Self {
        self.0.set_text_search_binding(Some(binding.0));
        self
    }
}

impl TreeDataGridColumnBuilder<TreeDataGridHierarchicalExpanderColumn> {
    pub fn children_path(self, path: impl AsRef<str>) -> Self {
        self.children_binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn children_binding(self, binding: AssignedBinding) -> Self {
        self.0.set_children_binding(Some(binding.0));
        self
    }

    pub fn has_children_path(self, path: impl AsRef<str>) -> Self {
        self.has_children_binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn has_children_binding(self, binding: AssignedBinding) -> Self {
        self.0.set_has_children_binding(Some(binding.0));
        self
    }

    pub fn is_expanded_path(self, path: impl AsRef<str>) -> Self {
        self.is_expanded_binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn is_expanded_binding(self, binding: AssignedBinding) -> Self {
        self.0.set_is_expanded_binding(Some(binding.0));
        self
    }
}

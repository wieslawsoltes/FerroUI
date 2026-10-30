// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{controls::*, model_projection};
use ferrodock_controls_proportional_stack_panel::ProportionalStackPanelSplitter;
use ferrodock_model::core::IDockable;
use ferroui_base::{
    data::{
        core::{plugins::InpcPropertyAccessor, BoxedPropertyGetter, ClrPropertyInfo, ValueType},
        BindingError, CompiledBinding, CompiledBindingPathBuilder,
    },
    BoxedValue, Ref,
};
use ferroui_controls::{
    templates::{FuncDataTemplate, IDataTemplate},
    Control, GridSplitter,
};
use std::rc::Rc;

/// Original ordered default templates used by DockControl.
pub(crate) struct DockDataTemplateHelper;
impl DockDataTemplateHelper {
    /// Creates each template on enumeration, in the original source order.
    pub(crate) fn create_default_data_templates() -> DefaultDataTemplates {
        DefaultDataTemplates
    }
}
/// Repeatable source enumerable; every iteration creates fresh templates lazily.
pub(crate) struct DefaultDataTemplates;
impl IntoIterator for DefaultDataTemplates {
    type Item = Rc<dyn IDataTemplate>;
    type IntoIter = std::iter::Map<std::ops::Range<u8>, fn(u8) -> Rc<dyn IDataTemplate>>;
    fn into_iter(self) -> Self::IntoIter {
        (0..14).map(create_template)
    }
}
impl IntoIterator for &DefaultDataTemplates {
    type Item = Rc<dyn IDataTemplate>;
    type IntoIter = <DefaultDataTemplates as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        DefaultDataTemplates.into_iter()
    }
}
fn create_template(index: u8) -> Rc<dyn IDataTemplate> {
    match index {
        0 => create_data_template(
            |m| m.as_document_content().is_some(),
            || DocumentContentControl::new().upcast(),
        ),
        1 => create_data_template(
            |m| m.as_tool_content().is_some(),
            || ToolContentControl::new().upcast(),
        ),
        2 => create_data_template(
            |m| m.as_proportional_dock_splitter().is_some(),
            || {
                let control = ProportionalStackPanelSplitter::new();
                let can_resize = binding(
                    "CanResize",
                    ValueType::of::<bool>(),
                    Rc::new(|value| {
                        let model = model_projection::dockable(Some(value))
                            .and_then(|m| m.as_proportional_dock_splitter())
                            .ok_or_else(wrong_source)?;
                        Ok(Some(Rc::new(model.can_resize())))
                    }),
                );
                control.bind_binding(
                    ProportionalStackPanelSplitter::is_resizing_enabled_property().as_property(),
                    &*can_resize,
                );
                let preview = binding(
                    "ResizePreview",
                    ValueType::of::<bool>(),
                    Rc::new(|value| {
                        let model = model_projection::dockable(Some(value))
                            .and_then(|m| m.as_proportional_dock_splitter())
                            .ok_or_else(wrong_source)?;
                        Ok(Some(Rc::new(model.resize_preview())))
                    }),
                );
                control.bind_binding(
                    ProportionalStackPanelSplitter::preview_resize_property().as_property(),
                    &*preview,
                );
                control.upcast()
            },
        ),
        3 => create_data_template(
            |m| m.as_grid_dock_splitter().is_some(),
            || {
                let control = GridSplitter::new();
                let direction = binding(
                    "ResizeDirection",
                    ValueType::of::<ferrodock_model::core::GridResizeDirection>(),
                    Rc::new(|value| {
                        let model = model_projection::dockable(Some(value))
                            .and_then(|m| m.as_grid_dock_splitter())
                            .ok_or_else(wrong_source)?;
                        Ok(Some(Rc::new(model.resize_direction())))
                    }),
                );
                control.bind_binding(
                    GridSplitter::resize_direction_property().as_property(),
                    &*direction,
                );
                control.upcast()
            },
        ),
        4 => create_data_template(
            |m| m.as_document_dock().is_some(),
            || DocumentDockControl::new().upcast(),
        ),
        5 => create_data_template(
            |m| m.as_tool_dock().is_some(),
            || ToolDockControl::new().upcast(),
        ),
        6 => create_data_template(
            |m| m.as_split_view_dock().is_some(),
            || SplitViewDockControl::new().upcast(),
        ),
        7 => create_data_template(
            |m| m.as_proportional_dock().is_some(),
            || ProportionalDockControl::new().upcast(),
        ),
        8 => create_data_template(
            |m| m.as_stack_dock().is_some(),
            || StackDockControl::new().upcast(),
        ),
        9 => create_data_template(
            |m| m.as_grid_dock().is_some(),
            || GridDockControl::new().upcast(),
        ),
        10 => create_data_template(
            |m| m.as_wrap_dock().is_some(),
            || WrapDockControl::new().upcast(),
        ),
        11 => create_data_template(
            |m| m.as_uniform_grid_dock().is_some(),
            || UniformGridDockControl::new().upcast(),
        ),
        12 => create_data_template(
            |m| m.as_dock_dock().is_some(),
            || DockDockControl::new().upcast(),
        ),
        13 => create_data_template(
            |m| m.as_root_dock().is_some(),
            || RootDockControl::new().upcast(),
        ),
        _ => unreachable!(),
    }
}

// The original reference-type generic template accepts null as well as objects
// implementing its interface. Membership uses the existing original model hooks;
// native raw model handles participate through the shared ValueTypes registrations.
fn create_data_template(
    accepts: fn(Rc<dyn IDockable>) -> bool,
    factory: fn() -> Ref<Control>,
) -> Rc<dyn IDataTemplate> {
    let matches = move |value: Option<&BoxedValue>| {
        value.is_none() || model_projection::dockable(value).is_some_and(accepts)
    };
    FuncDataTemplate::new(
        matches,
        move |value, _| {
            assert!(
                matches(value.as_ref()),
                "The data passed to the data template does not implement its model contract."
            );
            Some(factory())
        },
        false,
    )
}
fn wrong_source() -> BindingError {
    BindingError::message("The data context does not implement the splitter property contract.")
}
fn binding(name: &str, kind: ValueType, getter: BoxedPropertyGetter) -> Rc<CompiledBinding> {
    let property = Rc::new(
        ClrPropertyInfo::new(name, None, None, kind).with_boxed_accessors(Some(getter), None),
    );
    CompiledBinding::new(
        CompiledBindingPathBuilder::new()
            .property(
                property,
                Rc::new(|source, property| {
                    InpcPropertyAccessor::new(
                        source.clone(),
                        property,
                        Some(Rc::new(model_projection::notifier)),
                    )
                }),
            )
            .build(),
    )
}
#[cfg(test)]
#[path = "dock_data_template_helper_tests.rs"]
mod tests;

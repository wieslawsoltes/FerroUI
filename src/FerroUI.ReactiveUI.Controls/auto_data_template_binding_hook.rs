//! `AutoDataTemplateBindingHook` (`AutoDataTemplateBindingHook.cs`): gives an `ItemsControl`
//! bound to view models a default item template that hosts a [`ViewModelViewHost`].

use crate::{FerroObjectHandles, ViewModelViewHost};
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{FerroObject, FerroObjectExtensions, StyledElement};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::ItemsControl;
use ferroui_reactiveui::{BindingDirection, BindingHookChanges, IPropertyBindingHook, ObservedChangedMixins};
use std::any::Any;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The default data template that wraps items in a [`ViewModelViewHost`]
    /// (`DefaultItemTemplate`).
    static DEFAULT_ITEM_TEMPLATE: Rc<FuncDataTemplate> = FuncDataTemplate::new(
        |data| data.is_some(),
        |_, _| {
            let control = ViewModelViewHost::new();
            let object: &FerroObject = &control;
            let context = FerroObjectExtensions::get_observable(object, StyledElement::data_context_property());
            let _ = control.bind(ViewModelViewHost::view_model_property(), context, BindingPriority::LocalValue);
            control.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            control.set_vertical_content_alignment(VerticalAlignment::Stretch);
            Some(control.upcast())
        },
        true,
    );
}

/// A binding hook that gives an `ItemsControl` a default data template when it has no item
/// template and no data templates (`AutoDataTemplateBindingHook`).
///
/// It serves view models bound to item controls: without a template the items would not be
/// rendered as their views, so the default template shows each item with a
/// [`ViewModelViewHost`].
#[derive(Debug, Default)]
pub struct AutoDataTemplateBindingHook;

impl AutoDataTemplateBindingHook {
    /// The hook.
    pub fn new() -> Self {
        Self
    }

    /// The default data template (`DefaultItemTemplate`).
    pub fn default_item_template() -> Rc<dyn IDataTemplate> {
        DEFAULT_ITEM_TEMPLATE.with(|template| template.clone() as Rc<dyn IDataTemplate>)
    }
}

impl IPropertyBindingHook for AutoDataTemplateBindingHook {
    fn execute_hook(
        &self,
        _source: Option<&Rc<dyn Any>>,
        _target: &Rc<dyn Any>,
        _get_current_view_model_properties: &dyn Fn() -> BindingHookChanges,
        get_current_view_properties: &dyn Fn() -> BindingHookChanges,
        _direction: BindingDirection,
    ) -> bool {
        let view_properties = get_current_view_properties();
        let Some(last_view_property) = view_properties.last() else {
            return true;
        };
        let Some(items_control) = FerroObjectHandles::object_of(&*last_view_property.sender).and_then(|sender| sender.cast::<ItemsControl>()) else {
            return true;
        };

        let property_name = last_view_property.get_property_name();
        if property_name != "Items" && property_name != "ItemsSource" {
            return true;
        }

        if items_control.item_template().is_some() {
            return true;
        }

        if items_control.data_templates().count() > 0 {
            return true;
        }

        items_control.set_item_template(Some(Self::default_item_template()));
        true
    }
}

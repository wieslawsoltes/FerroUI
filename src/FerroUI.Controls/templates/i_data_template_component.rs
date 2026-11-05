// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

//! The phasing contract of a template instance and the attached property that carries it.
//!
//! WinUI declares the contract as `Microsoft.UI.Xaml.Markup.IDataTemplateComponent` and the
//! property as `XamlBindingHelper.DataTemplateComponent` (the pinned microsoft-ui-xaml checkout
//! of NOTICE.md: the `ProcessBindings` and `Recycle` members the XAML compiler generates in
//! `CodeGenerators/CSharpPagePass2.tt`). The repeater (`ferroui-controls-repeater`) drives it, and
//! the bindings of `{x:Bind}` (`ferroui-markup-xaml`) implement it; it is declared here, where both
//! can name it, and the repeater re-exports it where it was.

use crate::Control;
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroProperty};
use std::rc::Rc;

/// The bindings of a template instance, driven by the container that hosts the instance.
///
/// The container calls [`process_bindings`](Self::process_bindings) with phase `0` when it
/// prepares the element for an item (before it assigns the data context), then with each phase
/// the previous call returned while it is positive, and [`recycle`](Self::recycle) before the
/// element is reused for another item. Phase zero reads the bindings of the elements without a
/// phase; a later phase reads the bindings of the elements with that phase (`x:Phase`).
pub trait IDataTemplateComponent {
    /// Reads the bindings of `current_phase` for `data`, the item at `item_index` (`-1` when the
    /// container does not give it), and returns the next phase to process, or `-1` when there is
    /// none.
    fn process_bindings(&self, data: Option<BoxedValue>, item_index: i32, current_phase: i32) -> i32;

    /// Releases what the bindings listen to before the element is reused for another item; the
    /// elements of the later phases are hidden again until their phase.
    fn recycle(&self);
}

/// Handles compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IDataTemplateComponent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// WinUI's `XamlBindingHelper`: the attached property of the template component of an element.
///
/// The root of a template instance with `{x:Bind}` bindings carries the bindings of the instance
/// as its component; a container that finds one drives the instance through it instead of its
/// data context.
pub struct XamlBindingHelper;

ferro_static_type!(XamlBindingHelper);

ferro_properties! {
    impl XamlBindingHelper {
        /// The template component of the root of a template instance.
        pub fn data_template_component_property() -> AttachedProperty<Option<Rc<dyn IDataTemplateComponent>>> {
            FerroProperty::register_attached::<XamlBindingHelper, Control, _>("DataTemplateComponent", None)
        }
    }
}

impl XamlBindingHelper {
    /// The template component of `element`.
    pub fn get_data_template_component(element: &Control) -> Option<Rc<dyn IDataTemplateComponent>> {
        element.get_value(Self::data_template_component_property())
    }

    /// Sets the template component of `element`.
    pub fn set_data_template_component(element: &Control, component: Option<Rc<dyn IDataTemplateComponent>>) {
        element.set_value(Self::data_template_component_property(), component);
    }
}

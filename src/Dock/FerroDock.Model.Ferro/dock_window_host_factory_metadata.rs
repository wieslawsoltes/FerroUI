// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native optional host creation dispatches the original enclosing factory contract.
use ferrodock_model::core::{IDockWindow, IDockWindowHostFactory, IFactory, IHostWindow};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Descriptor carrier for the source optional full-window host factory.
#[doc(hidden)]
pub trait IDockWindowHostFactoryMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockWindowHostFactoryMetadata as "IDockWindowHostFactory" {
    namespace:"FerroDock.Model.Core",this:Rc<dyn IDockWindowHostFactory>,handles:[Rc<dyn IDockWindowHostFactory>,Option<Rc<dyn IDockWindowHostFactory>>],
    methods:[fn CreateHostWindow(Option<Rc<dyn IDockWindow>>)->Option<Rc<dyn IHostWindow>> => (|this:&Rc<dyn IDockWindowHostFactory>,window|this.create_host_window(window))],
});
pub(crate) fn register() {
    MarkupType::register(<dyn IDockWindowHostFactoryMetadata>::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IDockWindowHostFactory>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDockWindowHostFactory>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IFactory>, Rc<dyn IDockWindowHostFactory>>(|value| {
        value
            .downcast_ref::<Rc<dyn IFactory>>()?
            .clone()
            .as_dock_window_host_factory()
    });
    ferrodock_runtime::register_value_type::<Rc<dyn IDockWindowHostFactory>>();
}

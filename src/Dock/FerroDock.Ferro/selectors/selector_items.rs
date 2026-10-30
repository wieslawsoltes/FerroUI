//! Native registration for the original selector list projection.
use super::DockSelectorItem;
use crate::read_only_items::items;
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::ItemsSource;
use std::rc::Rc;
pub(crate) fn register_types() {
    ValueTypes::register_interface_object::<Rc<dyn super::IDockSelectorService>>(|service| {
        service.as_object()
    });
    ValueTypes::register_reference::<DockSelectorItem>();
    ValueTypes::register_nullable::<ReadOnlyListHandle<Rc<DockSelectorItem>>>();
    ItemsSource::register_binding_conversion_with::<ReadOnlyListHandle<Rc<DockSelectorItem>>>(
        items,
    );
    ValueTypes::register_conversion::<
        Option<ReadOnlyListHandle<Rc<DockSelectorItem>>>,
        Option<ItemsSource>,
    >(|v| Some(v.as_ref().map(items)));
}

// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{
    DefaultDockCommandBarAdapter, DockCommandBarDefinitionMetadata, DockCommandBarItemMetadata,
    DockCommandBarManager, IDockCommandBarAdapter, IDockCommandBarProviderMetadata,
};
use crate::read_only_items::items;
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{DockCommandBarDefinition, DockCommandBarItem, IDockCommandBarProvider},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
    Ref,
};
use ferroui_controls::{Control, ItemsSource};
use std::rc::Rc;
pub(crate) fn register_types() {
    MarkupType::register_all(&[
        DefaultDockCommandBarAdapter::MARKUP,
        DockCommandBarManager::MARKUP,
        <dyn IDockCommandBarAdapter>::MARKUP,
        <dyn DockCommandBarDefinitionMetadata>::MARKUP,
        <dyn DockCommandBarItemMetadata>::MARKUP,
        <dyn IDockCommandBarProviderMetadata>::MARKUP,
    ]);
    ValueTypes::register_reference::<DockCommandBarDefinition>();
    ValueTypes::register_reference::<DockCommandBarItem>();
    ValueTypes::register_reference::<DefaultDockCommandBarAdapter>();
    ValueTypes::register_reference::<DockCommandBarManager>();
    ValueTypes::register_nullable::<ReadOnlyListHandle<Ref<Control>>>();
    ValueTypes::register_nullable::<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>();
    ValueTypes::register_nullable::<ReadOnlyListHandle<Rc<DockCommandBarItem>>>();
    ValueTypes::register_nullable::<Rc<dyn IDockCommandBarAdapter>>();
    ValueTypes::register_nullable::<Rc<dyn IDockCommandBarProvider>>();
    ValueTypes::register_cast::<Rc<DefaultDockCommandBarAdapter>, Rc<dyn IDockCommandBarAdapter>>(
        |adapter| adapter.clone(),
    );
    ItemsSource::register_binding_conversion_with::<ReadOnlyListHandle<Ref<Control>>>(items);
    ValueTypes::register_conversion::<Option<ReadOnlyListHandle<Ref<Control>>>, Option<ItemsSource>>(
        |value| Some(value.as_ref().map(items)),
    );
}

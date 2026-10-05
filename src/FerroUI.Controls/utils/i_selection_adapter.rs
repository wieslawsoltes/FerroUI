// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use crate::{unbox_item, ItemsSource, SelectionChangedEventArgs};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::KeyEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroObject, Ref};
use std::rc::Rc;

/// Defines an item collection, selection members, and key handling for the
/// selection adapter contained in the drop-down portion of an
/// [`AutoCompleteBox`](crate::AutoCompleteBox) control.
///
/// A class states that it is a selection adapter by listing
/// `Rc<dyn ISelectionAdapter>` among the interfaces of its class info.
pub trait ISelectionAdapter {
    /// Gets the selected item.
    fn selected_item(&self) -> Option<BoxedValue>;

    /// Sets the selected item.
    fn set_selected_item(&self, value: Option<BoxedValue>);

    /// Occurs when the `SelectedItem` property value changes.
    fn selection_changed(&self, handler: Rc<dyn Fn(&SelectionChangedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Gets a collection that is used to generate content for the
    /// selection adapter.
    fn items_source(&self) -> Option<ItemsSource>;

    /// Sets a collection that is used to generate content for the
    /// selection adapter.
    fn set_items_source(&self, value: Option<ItemsSource>);

    /// Occurs when a selected item is not cancelled and is committed as the
    /// selected item.
    fn commit(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Occurs when a selection has been cancelled.
    fn cancel(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Provides handling for the key down event that occurs when a key is
    /// pressed while the drop-down portion of the
    /// [`AutoCompleteBox`](crate::AutoCompleteBox) has focus.
    fn handle_key_down(&self, e: &KeyEventArgs);
}

/// Selection adapters are reference types: they compare by identity.
impl PartialEq for dyn ISelectionAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl dyn ISelectionAdapter {
    /// The selection adapter an object is, if its class declares the
    /// contract (the `as` cast of the reference implementation).
    pub fn from_object(object: &Ref<FerroObject>) -> Option<Rc<dyn ISelectionAdapter>> {
        let boxed: BoxedValue = Rc::new(object.clone());
        let adapter = ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn ISelectionAdapter>>())?;
        unbox_item::<Rc<dyn ISelectionAdapter>>(&Some(adapter))
    }
}

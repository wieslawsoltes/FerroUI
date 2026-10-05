// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::ISelectionAdapter;
use crate::primitives::SelectingItemsControl;
use crate::{ItemsSource, ScrollViewer, SelectionChangedEventArgs};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, PointerReleasedEventArgs};
use ferroui_base::interactivity::{RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Ref, StyledElement, Vector};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The overridable members of [`SelectingItemsControlSelectionAdapter`]:
/// the methods a class deriving from it can replace. Every method defaults
/// to the behaviour of the adapter (`base_*`), which an override calls to
/// run the base implementation.
///
/// Install the overrides with
/// [`SelectingItemsControlSelectionAdapter::with_overrides`].
pub trait SelectingItemsControlSelectionAdapterOverrides {
    /// Raises the commit event.
    fn on_commit(&self, adapter: &SelectingItemsControlSelectionAdapter) {
        adapter.base_on_commit()
    }

    /// Raises the cancel event.
    fn on_cancel(&self, adapter: &SelectingItemsControlSelectionAdapter) {
        adapter.base_on_cancel()
    }
}

/// Represents the selection adapter contained in the drop-down portion of
/// an [`AutoCompleteBox`](crate::AutoCompleteBox) control.
pub struct SelectingItemsControlSelectionAdapter {
    weak_self: Weak<SelectingItemsControlSelectionAdapter>,
    overrides: Option<Rc<dyn SelectingItemsControlSelectionAdapterOverrides>>,

    /// The selecting items control instance.
    selector: RefCell<Option<Ref<SelectingItemsControl>>>,

    /// The handlers of the selection changed and pointer released events
    /// of the selector.
    selector_handlers: Cell<Option<(RoutedEventHandlerToken, RoutedEventHandlerToken)>>,

    /// A value indicating whether the selection change event should not be
    /// fired.
    ignoring_selection_changed: Cell<bool>,

    selection_changed: HandlerList<dyn Fn(&SelectionChangedEventArgs)>,
    commit: HandlerList<dyn Fn(&RoutedEventArgs)>,
    cancel: HandlerList<dyn Fn(&RoutedEventArgs)>,
}

impl SelectingItemsControlSelectionAdapter {
    /// Initializes a new instance of the
    /// [`SelectingItemsControlSelectionAdapter`] class.
    pub fn new() -> Rc<Self> {
        Self::create(None, None)
    }

    /// Initializes a new instance of the
    /// [`SelectingItemsControlSelectionAdapter`] class with the specified
    /// [`SelectingItemsControl`] control.
    pub fn with_selector(selector: Ref<SelectingItemsControl>) -> Rc<Self> {
        Self::create(Some(selector), None)
    }

    /// Creates an adapter whose overridable members are replaced by
    /// `overrides`: the form a class deriving from the adapter takes.
    pub fn with_overrides(
        selector: Option<Ref<SelectingItemsControl>>,
        overrides: Rc<dyn SelectingItemsControlSelectionAdapterOverrides>,
    ) -> Rc<Self> {
        Self::create(selector, Some(overrides))
    }

    fn create(
        selector: Option<Ref<SelectingItemsControl>>,
        overrides: Option<Rc<dyn SelectingItemsControlSelectionAdapterOverrides>>,
    ) -> Rc<Self> {
        let adapter = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            overrides,
            selector: RefCell::new(None),
            selector_handlers: Cell::new(None),
            ignoring_selection_changed: Cell::new(false),
            selection_changed: HandlerList::new(),
            commit: HandlerList::new(),
            cancel: HandlerList::new(),
        });
        if selector.is_some() {
            adapter.set_selector_control(selector);
        }
        adapter
    }

    /// Gets the underlying [`SelectingItemsControl`] control.
    pub fn selector_control(&self) -> Option<Ref<SelectingItemsControl>> {
        self.selector.borrow().clone()
    }

    /// Sets the underlying [`SelectingItemsControl`] control.
    pub fn set_selector_control(&self, value: Option<Ref<SelectingItemsControl>>) {
        let old = self.selector.borrow().clone();
        if let (Some(old), Some((selection_changed, pointer_released))) = (old, self.selector_handlers.take()) {
            old.remove_handler(SelectingItemsControl::selection_changed_event(), selection_changed);
            old.remove_handler(InputElement::pointer_released_event(), pointer_released);
        }

        *self.selector.borrow_mut() = value.clone();

        if let Some(selector) = value {
            // The selector is kept alive by this adapter: its handlers
            // refer back to the adapter weakly.
            let weak = self.weak_self.clone();
            let selection_changed = selector.selection_changed(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_selection_changed(e);
                }
            });
            let weak = self.weak_self.clone();
            let pointer_released =
                selector.add_handler(InputElement::pointer_released_event(), move |_, e: &PointerReleasedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_selector_pointer_released(e);
                    }
                });
            self.selector_handlers.set(Some((selection_changed, pointer_released)));
        }
    }

    /// If the control contains a scroll viewer, this will reset the viewer
    /// to be scrolled to the top.
    fn reset_scroll_viewer(&self) {
        fn first_scroll_viewer(element: &StyledElement) -> Option<Ref<ScrollViewer>> {
            let children = element.logical_children().snapshot();
            for child in children.iter() {
                if let Some(scroll_viewer) = child.clone().cast::<ScrollViewer>() {
                    return Some(scroll_viewer);
                }
                if let Some(scroll_viewer) = first_scroll_viewer(child) {
                    return Some(scroll_viewer);
                }
            }
            None
        }

        if let Some(selector) = self.selector_control() {
            if let Some(scroll_viewer) = first_scroll_viewer(&selector) {
                scroll_viewer.set_offset(Vector::new(0.0, 0.0));
            }
        }
    }

    /// Handles the pointer released event on the selector control, used to
    /// commit the selection of the adapter when the left button was
    /// released.
    fn on_selector_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if e.initial_press_mouse_button() == MouseButton::Left {
            self.on_commit();
        }
    }

    /// Handles the selection changed event on the selector control.
    fn on_selection_changed(&self, e: &SelectionChangedEventArgs) {
        if self.ignoring_selection_changed.get() {
            return;
        }

        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(e);
        }
    }

    /// Increments the selected index property of the selector control.
    pub fn selected_index_increment(&self) {
        if let Some(selector) = self.selector_control() {
            let next = selector.selected_index() + 1;
            selector.set_selected_index(if next >= selector.item_count() { -1 } else { next });
        }
    }

    /// Decrements the selected index property of the selector control.
    pub fn selected_index_decrement(&self) {
        if let Some(selector) = self.selector_control() {
            let index = selector.selected_index();
            if index >= 0 {
                selector.set_selected_index(index - 1);
            } else if index == -1 {
                selector.set_selected_index(selector.item_count() - 1);
            }
        }
    }

    /// Raises the commit event. Overridable: see
    /// [`SelectingItemsControlSelectionAdapterOverrides::on_commit`].
    pub fn on_commit(&self) {
        match &self.overrides {
            Some(overrides) => overrides.on_commit(self),
            None => self.base_on_commit(),
        }
    }

    /// The implementation of [`on_commit`](Self::on_commit) of this class,
    /// for overrides.
    pub fn base_on_commit(&self) {
        self.on_commit_with(&RoutedEventArgs::new());
    }

    /// Fires the commit event.
    fn on_commit_with(&self, e: &RoutedEventArgs) {
        for (_, handler) in self.commit.snapshot().iter() {
            handler(e);
        }

        self.after_adapter_action();
    }

    /// Raises the cancel event. Overridable: see
    /// [`SelectingItemsControlSelectionAdapterOverrides::on_cancel`].
    pub fn on_cancel(&self) {
        match &self.overrides {
            Some(overrides) => overrides.on_cancel(self),
            None => self.base_on_cancel(),
        }
    }

    /// The implementation of [`on_cancel`](Self::on_cancel) of this class,
    /// for overrides.
    pub fn base_on_cancel(&self) {
        self.on_cancel_with(&RoutedEventArgs::new());
    }

    /// Fires the cancel event.
    fn on_cancel_with(&self, e: &RoutedEventArgs) {
        for (_, handler) in self.cancel.snapshot().iter() {
            handler(e);
        }

        self.after_adapter_action();
    }

    /// Changes the selected item after an adapter action is complete.
    fn after_adapter_action(&self) {
        self.ignoring_selection_changed.set(true);
        if let Some(selector) = self.selector_control() {
            selector.set_selected_item(None);
            selector.set_selected_index(-1);
        }
        self.ignoring_selection_changed.set(false);
    }

    /// The handle removing a handler from one of the events.
    fn subscription<F: ?Sized + 'static>(
        &self,
        list: fn(&Self) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = list(self).add(handler);
        let weak = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                list(&this).remove(token);
            }
        })
    }
}

impl ISelectionAdapter for SelectingItemsControlSelectionAdapter {
    fn selected_item(&self) -> Option<BoxedValue> {
        self.selector_control().and_then(|selector| selector.selected_item())
    }

    fn set_selected_item(&self, value: Option<BoxedValue>) {
        self.ignoring_selection_changed.set(true);
        let is_null = value.is_none();
        if let Some(selector) = self.selector_control() {
            selector.set_selected_item(value);
        }

        // Attempt to reset the scroll viewer's position.
        if is_null {
            self.reset_scroll_viewer();
        }

        self.ignoring_selection_changed.set(false);
    }

    fn selection_changed(&self, handler: Rc<dyn Fn(&SelectionChangedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscription(|this| &this.selection_changed, handler)
    }

    fn items_source(&self) -> Option<ItemsSource> {
        self.selector_control().and_then(|selector| selector.items_source())
    }

    fn set_items_source(&self, value: Option<ItemsSource>) {
        if let Some(selector) = self.selector_control() {
            selector.set_items_source(value);
        }
    }

    fn commit(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscription(|this| &this.commit, handler)
    }

    fn cancel(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscription(|this| &this.cancel, handler)
    }

    fn handle_key_down(&self, e: &KeyEventArgs) {
        match e.key {
            Key::Enter => {
                self.on_commit();
                e.set_handled(true);
            }
            Key::Up => {
                self.selected_index_decrement();
                e.set_handled(true);
            }
            Key::Down => {
                if !e.key_modifiers.contains(KeyModifiers::ALT) {
                    self.selected_index_increment();
                    e.set_handled(true);
                }
            }
            Key::Escape => {
                self.on_cancel();
                e.set_handled(true);
            }
            _ => {}
        }
    }
}

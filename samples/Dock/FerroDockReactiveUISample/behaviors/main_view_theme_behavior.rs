// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! The original InitializeThemes hookup, moved to a behavior for passive views.
use crate::{views::MainView, App};
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::{
    controls::NameScopeExtensions, ferro_properties, reactive::IDisposable, AttachedProperty,
    BoxedValue, FerroProperty,
};
use ferroui_controls::{
    Button, ComboBox, IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource,
    ItemsView, TypedItems,
};
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use std::{any::Any, cell::RefCell, rc::Rc};
/// Initializes source theme controls after compiled markup has finished.
pub struct MainViewThemeBehavior;
ferroui_base::ferro_static_type!(MainViewThemeBehavior);
ferroui_base::ferro_markup_type!(static MainViewThemeBehavior{type_info:MainViewThemeBehavior});
ferro_properties! {impl MainViewThemeBehavior{
    /// Enables the source constructor's InitializeThemes behavior.
    pub fn initialize_themes_property()->AttachedProperty<bool>{FerroProperty::register_attached::<MainViewThemeBehavior,MainView,_>("InitializeThemes",false)}
}}
impl MainViewThemeBehavior {
    fn static_constructor() {
        Self::initialize_themes_property()
            .changed()
            .add_class_handler::<MainView>(|view, change| {
                if change.get_new_value::<bool>() {
                    if view.is_initialized() {
                        Self::initialize(view);
                    } else {
                        let weak = view.to_ref().downgrade();
                        let pending: Rc<RefCell<Option<Rc<dyn IDisposable>>>> =
                            Rc::new(RefCell::new(None));
                        let token = pending.clone();
                        let subscription = view.initialized(move || {
                            if let Some(subscription) = token.borrow_mut().take() {
                                subscription.dispose();
                            }
                            if let Some(view) = weak.upgrade() {
                                Self::initialize(&view);
                            }
                        });
                        *pending.borrow_mut() = Some(subscription);
                    }
                }
            });
    }
    fn initialize(view: &MainView) {
        let Some(manager) = App::theme_manager() else {
            return;
        };
        let model = App::create_theme_view_model(manager.clone());
        if let Some(theme) = NameScopeExtensions::find::<Button>(view, "ThemeButton") {
            theme.set_command(Some(ReactiveCommandAdapter::command_of(
                model.toggle_theme(),
            )));
        }
        if let Some(preset) = NameScopeExtensions::find::<ComboBox>(view, "PresetComboBox") {
            preset.set_items_source(Some(ItemsSource::new(Rc::new(PresetNamesItems(
                manager.preset_names(),
            )))));
            preset.set_selected_index(manager.current_preset_index());
            let owner = preset.clone();
            let command = model.select_preset();
            preset.selection_changed(move |_, _| {
                let index = owner.selected_index();
                command.execute(Some(&index));
            });
        }
        if let Some(tabs) = NameScopeExtensions::find::<ComboBox>(view, "TabThemeComboBox") {
            tabs.set_items_source(Some(ItemsSource::from_strs(["Fluent", "Browser"])));
            tabs.set_selected_index(model.selected_tab_theme());
            let weak = tabs.downgrade();
            let command = model.select_tab_theme();
            tabs.selection_changed(move |_, _| {
                if let Some(tabs) = weak.upgrade() {
                    command.execute(Some(&tabs.selected_index()));
                }
            });
        }
    }
}
// Reads the same source list and forwards optional source notifications without
// copying its names or inventing observable-list behavior for an ordinary array.
struct PresetNamesItems(ReadOnlyListHandle<String>);
impl IItemsList for PresetNamesItems {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.0.get(index)))
    }
    fn identity(&self) -> Option<usize> {
        Some(self.0.identity())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn is_notifying(&self) -> bool {
        self.0.as_notify_collection_changed().is_some()
    }
    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        self.0.add_collection_changed(Rc::new(move |event| {
            let new_items = TypedItems(event.new_items);
            let old_items = TypedItems(event.old_items);
            handler(&ItemsChangedEventArgs {
                action: event.action,
                new_items: ItemsView::from_typed(&new_items),
                old_items: ItemsView::from_typed(&old_items),
                new_starting_index: event.new_starting_index,
                old_starting_index: event.old_starting_index,
            });
        }))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }
}

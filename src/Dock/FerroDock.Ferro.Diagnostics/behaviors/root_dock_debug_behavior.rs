// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original inspector subscriptions and selection highlighting, outside passive view code.
use crate::{
    controls::RootDockDebug, helpers::selection_overlay_helper::SelectionOverlayHelper,
    view_models::EventLogView,
};
use ferrodock_ferro::diagnostics::DockDiagnosticEvents;
use ferroui_base::{
    controls::NameScopeExtensions, data::BindingPriority, ferro_properties, AttachedProperty,
    FerroProperty, Ref,
};
use ferroui_controls::{primitives::SelectingItemsControl, TreeView};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{cell::RefCell, rc::Rc};
/// Attaches the original source inspector behavior to its compiled view.
pub struct RootDockDebugBehavior;
ferroui_base::ferro_static_type!(RootDockDebugBehavior);
ferroui_base::ferro_markup_type!(static RootDockDebugBehavior{namespace:"FerroDock.Ferro.Diagnostics.Behaviors",type_info:RootDockDebugBehavior});
ferro_properties! {impl RootDockDebugBehavior {
 /// Enables the original constructor subscriptions during component loading.
 pub fn is_enabled_property()->AttachedProperty<bool>{FerroProperty::register_attached::<RootDockDebugBehavior,RootDockDebug,_>("IsEnabled",false)}
}}
impl RootDockDebugBehavior {
    fn static_constructor() {
        Self::is_enabled_property()
            .changed()
            .add_class_handler::<RootDockDebug>(|view, event| {
                if !event.get_new_value::<bool>() {
                    return;
                }
                // These source constructor subscriptions must exist before callers
                // assign DataContext. Native Initialized waits for a logical root;
                // selection-name lookup itself remains deferred until visual attach.
                Self::initialize(view.to_ref());
            });
    }
    fn initialize(view: Ref<RootDockDebug>) {
        let owner = view.clone();
        view.data_context_changed(move || {
            let factory = dockable(owner.data_context().as_ref()).and_then(|model| model.factory());
            owner.presentation().set_factory(factory);
        });
        let owner = view.clone();
        view.presentation()
            .property_changed()
            .add(Rc::new(move |name| {
                if name == "Filter" {
                    owner.raise_property_changed(
                        RootDockDebug::filter_property().as_property(),
                        None,
                        &owner.filter(),
                        BindingPriority::LocalValue,
                        true,
                    );
                }
                if name == "FilteredEvents" {
                    owner.raise_property_changed(
                        RootDockDebug::filtered_events_property().as_property(),
                        None,
                        &owner.filtered_events(),
                        BindingPriority::LocalValue,
                        true,
                    );
                }
            }));
        let state = Rc::new(SelectionState {
            view: view.clone(),
            tree: RefCell::new(None),
            selection: RefCell::new(None),
            request: RefCell::new(None),
            overlay: RefCell::new(None),
        });
        let attached = state.clone();
        view.attached_to_visual_tree(move |_| attached.attach());
        let detached = state.clone();
        view.detached_from_visual_tree(move |_| detached.detach());
        if view.is_attached_to_visual_tree() {
            state.attach();
        }
    }
}
struct SelectionState {
    view: Ref<RootDockDebug>,
    tree: RefCell<Option<Ref<TreeView>>>,
    selection: RefCell<Option<ferroui_base::interactivity::RoutedEventHandlerToken>>,
    request: RefCell<Option<u64>>,
    overlay: RefCell<Option<Rc<SelectionOverlayHelper>>>,
}
impl SelectionState {
    fn attach(self: &Rc<Self>) {
        let tree = NameScopeExtensions::find::<TreeView>(&self.view, "Visible");
        self.tree.replace(tree.clone());
        if let Some(tree) = tree {
            let owner = self.clone();
            let token = tree.selection_changed(move |_, _| owner.highlight());
            self.selection.replace(Some(token));
        }
        let owner = self.clone();
        let token =
            DockDiagnosticEvents::select_dockable_requested().add(Rc::new(move |dockable| {
                let Some(dockable) = dockable else {
                    return;
                };
                let tree = owner.tree.borrow().clone();
                if let Some(tree) = tree {
                    tree.set_selected_item(Some(Rc::new(dockable.clone())));
                }
            }));
        self.request.replace(Some(token));
    }
    fn detach(&self) {
        let tree = self.tree.borrow().clone();
        let selection = *self.selection.borrow();
        if let (Some(tree), Some(selection)) = (tree, selection) {
            tree.remove_handler(SelectingItemsControl::selection_changed_event(), selection);
        }
        self.selection.replace(None);
        self.tree.replace(None);
        let overlay = self.overlay.borrow().clone();
        if let Some(overlay) = overlay {
            overlay.remove_overlay();
        }
        self.overlay.replace(None);
        if let Some(token) = self.request.replace(None) {
            DockDiagnosticEvents::select_dockable_requested().remove(token);
        }
    }
    fn highlight(&self) {
        let tree = self.tree.borrow().clone();
        let model = tree.and_then(|tree| dockable(tree.selected_item().as_ref()));
        let pair = model.and_then(|model| {
            model
                .factory()
                .and_then(|factory| SelectionOverlayHelper::try_get_control(&*factory, &model))
        });
        if let Some((root, Some(control))) = pair {
            let current = self.overlay.borrow().clone();
            let overlay = current.unwrap_or_else(|| {
                let overlay = SelectionOverlayHelper::new();
                self.overlay.replace(Some(overlay.clone()));
                overlay
            });
            overlay.attach_overlay(root);
            overlay.highlight(Some(control));
        } else {
            let overlay = self.overlay.borrow().clone();
            if let Some(overlay) = overlay {
                overlay.highlight(None);
            }
        }
    }
}
struct LogItems(EventLogView);
impl ferroui_controls::IItemsList for LogItems {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, index: usize) -> Option<ferroui_base::BoxedValue> {
        Some(Rc::new(self.0.get(index)))
    }
    fn identity(&self) -> Option<usize> {
        Some(self.0.identity())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn is_notifying(&self) -> bool {
        self.0.is_unfiltered()
    }
    fn add_collection_changed(
        &self,
        handler: Rc<ferroui_controls::ItemsChangedHandler>,
    ) -> Option<u64> {
        if !self.0.is_unfiltered() {
            return None;
        }
        Some(
            self.0
                .events()
                .add_collection_changed(Rc::new(move |event| {
                    let new_items = ferroui_controls::TypedItems(event.new_items);
                    let old_items = ferroui_controls::TypedItems(event.old_items);
                    handler(&ferroui_controls::ItemsChangedEventArgs {
                        action: event.action,
                        new_items: ferroui_controls::ItemsView::from_typed(&new_items),
                        old_items: ferroui_controls::ItemsView::from_typed(&old_items),
                        new_starting_index: event.new_starting_index,
                        old_starting_index: event.old_starting_index,
                    });
                })),
        )
    }
    fn remove_collection_changed(&self, token: u64) {
        self.0.events().remove_collection_changed(token);
    }
}
// The source ItemsSourceView materializes a non-IList enumerable. Retain
// the actual observable collection for Events, and materialize only the source
// Where query at the native item-source boundary (the public query stays lazy).
fn items(view: &EventLogView) -> ferroui_controls::ItemsSource {
    if view.is_unfiltered() {
        ferroui_controls::ItemsSource::new(Rc::new(LogItems(view.clone())))
    } else {
        ferroui_controls::ItemsSource::from_values(view.iter())
    }
}
pub(crate) fn register_items() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::ItemsSource;
    ValueTypes::register_conversion::<EventLogView, ItemsSource>(|view| Some(items(view)));
    ValueTypes::register_conversion::<EventLogView, Option<ItemsSource>>(|view| {
        Some(Some(items(view)))
    });
}

fn dockable(
    value: Option<&ferroui_base::BoxedValue>,
) -> Option<Rc<dyn ferrodock_model::core::IDockable>> {
    ferroui_base::metadata::from_markup_value(&value.cloned())
}

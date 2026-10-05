use super::ItemsPresenter;
use crate::items_source::ItemsChangedEventArgs;
use crate::utils::CollectionUtils;
use crate::{Control, Controls, ItemsControl};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{ferro_property, AttachedProperty, BoxedValue, FerroProperty, Ref, WeakRef};
use std::cell::Cell;
use std::rc::Rc;

/// Generates containers for [`ItemsPresenter`]s that have non-virtualizing
/// panels.
pub(crate) struct PanelContainerGenerator {
    presenter: WeakRef<ItemsPresenter>,
    items_changed_token: Cell<u64>,
}

ferroui_base::ferro_static_type!(PanelContainerGenerator);

ferroui_base::ferro_properties! { impl PanelContainerGenerator {
    ferro_property!(
        fn item_is_own_container_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<PanelContainerGenerator, Control, _>("ItemIsOwnContainer", false)
        }
    );
} }

impl PanelContainerGenerator {
    pub fn new(presenter: &ItemsPresenter) -> Rc<Self> {
        let items_control = presenter.items_control();
        debug_assert!(items_control.is_some());
        debug_assert!(presenter.panel().is_some());

        let this = Rc::new(Self { presenter: presenter.to_ref().downgrade(), items_changed_token: Cell::new(0) });

        if let Some(items_control) = items_control {
            let weak = Rc::downgrade(&this);
            let token =
                items_control.items_view().add_post_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_items_changed(e);
                    }
                }));
            this.items_changed_token.set(token);
        }

        this.on_items_changed(&CollectionUtils::RESET_EVENT_ARGS);
        this
    }

    pub fn dispose(&self) {
        let Some(presenter) = self.presenter.upgrade() else { return };

        if let Some(items_control) = presenter.items_control() {
            items_control.items_view().remove_post_collection_changed(self.items_changed_token.get());
            self.clear_items_control_logical_children();
        }

        if let Some(panel) = presenter.panel() {
            panel.children().clear();
        }
    }

    pub fn refresh(&self) {
        self.on_items_changed(&CollectionUtils::RESET_EVENT_ARGS);
    }

    fn on_items_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        let Some(presenter) = self.presenter.upgrade() else { return };
        let (Some(panel), Some(items_control)) = (presenter.panel(), presenter.items_control()) else { return };

        let generator = items_control.item_container_generator();
        let children = panel.children();

        let add = |index: i32, items: &mut dyn Iterator<Item = Option<BoxedValue>>| {
            let mut i = index;
            for item in items {
                Self::insert_container(&items_control, &children, &item, i);
                i += 1;
            }

            let child_count = children.count() as i32;
            let delta = i - index;

            while i < child_count {
                generator.item_container_index_changed(&children.get(i as usize), i - delta, i);
                i += 1;
            }
        };

        let remove = |index: i32, count: i32| {
            for i in 0..count {
                let c = children.get((index + i) as usize);

                items_control.remove_logical_child(&c);

                if !c.is_set(Self::item_is_own_container_property().as_property()) {
                    generator.clear_item_container(&c);
                }
            }

            children.remove_range(index as usize, count as usize);

            let child_count = children.count() as i32;

            for i in index..child_count {
                generator.item_container_index_changed(&children.get(i as usize), i + count, i);
            }
        };

        let reset = || {
            self.clear_items_control_logical_children();
            children.clear();
            add(0, &mut items_control.items_view().iter());
        };

        match e.action {
            NotifyCollectionChangedAction::Add => add(e.new_starting_index, &mut e.new_items.iter()),
            NotifyCollectionChangedAction::Remove => remove(e.old_starting_index, e.old_items.len() as i32),
            NotifyCollectionChangedAction::Replace => {
                if e.old_starting_index < 0 {
                    reset();
                } else {
                    remove(e.old_starting_index, e.old_items.len() as i32);
                    add(e.new_starting_index, &mut e.new_items.iter());
                }
            }
            NotifyCollectionChangedAction::Move => {
                if e.old_starting_index < 0 {
                    reset();
                } else {
                    remove(e.old_starting_index, e.old_items.len() as i32);
                    let mut insert_index = e.new_starting_index;

                    if e.new_starting_index > e.old_starting_index {
                        insert_index -= e.old_items.len() as i32 - 1;
                    }

                    add(insert_index, &mut e.new_items.iter());
                }
            }
            NotifyCollectionChangedAction::Reset => reset(),
        }
    }

    fn insert_container(items_control: &Ref<ItemsControl>, children: &Controls, item: &Option<BoxedValue>, index: i32) {
        let generator = items_control.item_container_generator();

        let (needs_container, recycle_key) = generator.needs_container(item, index);
        let container = if needs_container {
            generator.create_container(item, index, recycle_key)
        } else {
            let container = item.as_ref().and_then(Control::from_boxed).expect("the item is its own container");
            container.set_value(Self::item_is_own_container_property(), true);
            container
        };

        generator.prepare_item_container(&container, item, index);
        items_control.add_logical_child(&container);
        children.insert(index as usize, &container);
        generator.item_container_prepared(&container, item, index);
    }

    fn clear_items_control_logical_children(&self) {
        let Some(presenter) = self.presenter.upgrade() else { return };
        let (Some(panel), Some(items_control)) = (presenter.panel(), presenter.items_control()) else { return };

        let generator = items_control.item_container_generator();

        for c in panel.children().snapshot().iter() {
            items_control.remove_logical_child(c);

            if !c.is_set(Self::item_is_own_container_property().as_property()) {
                generator.clear_item_container(c);
            }
        }
    }
}

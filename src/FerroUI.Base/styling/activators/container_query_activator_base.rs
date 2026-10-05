use super::{StyleActivator, StyleActivatorBase};
use crate::layout::Layoutable;
use crate::reactive::IDisposable;
use crate::styling::{Container, ContainerSizing, VisualQueryProvider};
use crate::{Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The state shared by the activators of size queries: the visual being
/// styled and the container whose size it currently tracks.
pub(crate) struct ContainerQueryActivatorBase {
    pub base: StyleActivatorBase,
    visual: WeakRef<Visual>,
    container_name: Option<String>,
    current_screen_size_provider: RefCell<Option<WeakRef<Layoutable>>>,
    provider_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    visual_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ContainerQueryActivatorBase {
    pub fn new(visual: &Visual, container_name: Option<&str>) -> Self {
        Self {
            base: StyleActivatorBase::new(),
            visual: visual.to_ref().downgrade(),
            container_name: container_name.map(str::to_string),
            current_screen_size_provider: RefCell::new(None),
            provider_subscriptions: RefCell::new(Vec::new()),
            visual_subscriptions: RefCell::new(Vec::new()),
        }
    }

    /// The container whose size is currently tracked.
    pub fn current_container(&self) -> Option<Ref<Layoutable>> {
        let current = self.current_screen_size_provider.borrow();
        current.as_ref().and_then(WeakRef::upgrade)
    }

    /// The query provider of the current container, if the container can be
    /// queried for the size selected by `accepts`.
    pub fn current_query_provider(&self, accepts: impl FnOnce(ContainerSizing) -> bool) -> Option<VisualQueryProvider> {
        let container = self.current_container()?;
        let sizing = Container::get_sizing(&container);
        let query_provider = Container::get_query_provider(&container)?;
        accepts(sizing).then_some(query_provider)
    }

    fn de_initialize_screen_size_provider(&self) {
        let has_provider = self.current_container().is_some_and(|c| Container::get_query_provider(&c).is_some());
        if has_provider {
            let subscriptions = std::mem::take(&mut *self.provider_subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
            *self.current_screen_size_provider.borrow_mut() = None;
        }
    }
}

/// Implemented by the activators of size queries.
pub(crate) trait ContainerQueryActivator: StyleActivator + Sized {
    fn container_base(&self) -> &ContainerQueryActivatorBase;

    fn weak(&self) -> Weak<Self>;

    fn initialize_screen_size_provider(&self) {
        let this = self.container_base();

        if this.current_screen_size_provider.borrow().is_none() {
            let container = this.visual.upgrade().and_then(|v| get_container(&v, this.container_name.as_deref()));
            if let Some(container) = container {
                if let Some(provider) = Container::get_query_provider(&container) {
                    *this.current_screen_size_provider.borrow_mut() = Some(container.downgrade());

                    let reevaluate = |weak: Weak<Self>| -> Rc<dyn Fn()> {
                        Rc::new(move || {
                            if let Some(this) = weak.upgrade() {
                                this.reevaluate_is_active();
                            }
                        })
                    };
                    let subscriptions = vec![
                        provider.width_changed(reevaluate(self.weak())),
                        provider.height_changed(reevaluate(self.weak())),
                    ];
                    *this.provider_subscriptions.borrow_mut() = subscriptions;
                }
            }
        }

        self.reevaluate_is_active();
    }

    fn initialize_container_query(&self) {
        self.initialize_screen_size_provider();

        let this = self.container_base();
        if let Some(visual) = this.visual.upgrade() {
            let weak = self.weak();
            let attached = visual.attached_to_visual_tree(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.initialize_screen_size_provider();
                }
            });
            let weak = self.weak();
            let detached = visual.detached_from_visual_tree(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.container_base().de_initialize_screen_size_provider();
                }
            });
            *this.visual_subscriptions.borrow_mut() = vec![attached, detached];
        }
    }

    fn deinitialize_container_query(&self) {
        let this = self.container_base();
        let subscriptions = std::mem::take(&mut *this.visual_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        this.de_initialize_screen_size_provider();
    }
}

/// Finds the container of a visual: the nearest visual ancestor that is a
/// container (when no name is given) or that has the given container name.
pub(crate) fn get_container(visual: &Visual, container_name: Option<&str>) -> Option<Ref<Layoutable>> {
    let mut current = visual.visual_parent();
    while let Some(ancestor) = current {
        if let Some(layoutable) = ancestor.cast::<Layoutable>() {
            let matches = match container_name {
                None => Container::get_sizing(&layoutable) != ContainerSizing::Normal,
                Some(name) => Container::get_name(&layoutable).as_deref() == Some(name),
            };
            if matches {
                return Some(layoutable);
            }
        }
        current = ancestor.visual_parent();
    }
    None
}

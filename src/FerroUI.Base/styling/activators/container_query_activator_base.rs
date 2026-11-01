use super::{StyleActivator, StyleActivatorBase};
use crate::layout::Layoutable;
use crate::reactive::IDisposable;
use crate::styling::{Container, ContainerSizing, VisualQueryProvider};
use crate::{Ref, StyledElement, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The state shared by the activators of size queries: the element being
/// styled, the visual whose ancestors are searched for its container (the
/// element itself, or for an element that is not a visual its first visual
/// logical ancestor) and the container whose size it currently tracks.
pub(crate) struct ContainerQueryActivatorBase {
    pub base: StyleActivatorBase,
    target: WeakRef<StyledElement>,
    container_name: Option<String>,
    visual: RefCell<Option<WeakRef<Visual>>>,
    current_screen_size_provider: RefCell<Option<WeakRef<Layoutable>>>,
    provider_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    visual_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    target_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ContainerQueryActivatorBase {
    pub fn new(target: &StyledElement, container_name: Option<&str>) -> Self {
        Self {
            base: StyleActivatorBase::new(),
            target: target.to_ref().downgrade(),
            container_name: container_name.map(str::to_string),
            visual: RefCell::new(None),
            current_screen_size_provider: RefCell::new(None),
            provider_subscriptions: RefCell::new(Vec::new()),
            visual_subscriptions: RefCell::new(Vec::new()),
            target_subscriptions: RefCell::new(Vec::new()),
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

    /// Whether the target is not a visual, so that the activator follows its
    /// logical parent to find the visual to search from.
    fn target_is_not_visual(&self) -> bool {
        self.target.upgrade().is_some_and(|target| !target.is::<Visual>())
    }
}

/// Implemented by the activators of size queries.
pub(crate) trait ContainerQueryActivator: StyleActivator + Sized {
    fn container_base(&self) -> &ContainerQueryActivatorBase;

    fn weak(&self) -> Weak<Self>;

    fn initialize_screen_size_provider(&self) {
        let this = self.container_base();

        if this.current_screen_size_provider.borrow().is_none() {
            let container = this.target.upgrade().and_then(|t| get_container(&t, this.container_name.as_deref()));
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

    /// Changes the visual whose attachment to the visual tree the activator
    /// follows: the container being tracked is released and the attachment
    /// handlers move from the old visual to the new one.
    fn set_visual(&self, visual: Option<Ref<Visual>>) {
        let this = self.container_base();

        let unchanged = match (&*this.visual.borrow(), &visual) {
            (None, None) => true,
            (Some(current), Some(visual)) => current.points_to(visual),
            _ => false,
        };
        if unchanged {
            return;
        }

        this.de_initialize_screen_size_provider();

        let subscriptions = std::mem::take(&mut *this.visual_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }

        *this.visual.borrow_mut() = visual.as_ref().map(Ref::downgrade);

        if let Some(visual) = visual {
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
            *this.visual_subscriptions.borrow_mut() = vec![attached.detach(), detached.detach()];
        }
    }

    fn initialize_container_query(&self) {
        let this = self.container_base();
        let target = this.target.upgrade();

        if this.target_is_not_visual() {
            if let Some(target) = &target {
                let weak = self.weak();
                let attached = target.attached_to_logical_tree(move |_| {
                    if let Some(this) = weak.upgrade() {
                        let visual = this.container_base().target.upgrade().and_then(|t| get_visual(&t));
                        this.set_visual(visual);
                        this.initialize_screen_size_provider();
                    }
                });
                let weak = self.weak();
                let detached = target.detached_from_logical_tree(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.set_visual(None);
                        this.reevaluate_is_active();
                    }
                });
                *this.target_subscriptions.borrow_mut() = vec![attached.detach(), detached.detach()];
            }
        }

        self.set_visual(target.as_deref().and_then(get_visual));
        self.initialize_screen_size_provider();
    }

    fn deinitialize_container_query(&self) {
        let this = self.container_base();
        let subscriptions = std::mem::take(&mut *this.target_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }

        self.set_visual(None);
    }
}

/// Finds the container of an element: the nearest visual ancestor that is a
/// container (when no name is given) or that has the given container name.
/// For an element that is not a visual the search starts at its first visual
/// logical ancestor, which can itself be the container.
pub(crate) fn get_container(target: &StyledElement, container_name: Option<&str>) -> Option<Ref<Layoutable>> {
    let visual = get_visual(target)?;

    // The first visual logical ancestor can itself be the container for a non-visual target.
    let mut current = if target.is::<Visual>() { visual.visual_parent() } else { Some(visual) };
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

/// The element itself if it is a visual, otherwise its first visual logical
/// ancestor.
fn get_visual(target: &StyledElement) -> Option<Ref<Visual>> {
    target.to_ref().cast::<Visual>().or_else(|| target.find_logical_ancestor_of_type::<Visual>(false))
}

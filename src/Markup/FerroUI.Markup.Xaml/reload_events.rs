//! Development ownership of routed subscriptions installed by XAML.
//! Application calls to Interactive::add_handler remain outside this ownership.
use crate::{xaml_il::runtime::XamlIlContext, FromXamlObject};
use ferroui_base::{
    interactivity::{Interactive, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies},
    metadata::{service, IServiceProvider, MarkupDelegate, MarkupValue},
    BoxedValue, FerroObject, Ref, WeakRef,
};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

ferroui_base::ferro_thread_local! {
    static CURRENT: RefCell<Vec<Weak<EventGeneration>>> = const { RefCell::new(Vec::new()) };
}
/// One population's subscriptions, retained by its contexts and callbacks. The
/// generation never retains its owner or event targets. Retired deferred content
/// cannot install new subscriptions even if application code keeps its factory.
pub struct EventGeneration {
    owner: WeakRef<FerroObject>,
    active: Cell<bool>,
    retired: Cell<bool>,
    subscriptions: RefCell<Vec<Weak<Subscription>>>,
}
impl EventGeneration {
    fn new(owner: &Ref<FerroObject>, active: bool) -> Rc<Self> {
        Rc::new(Self {
            owner: owner.downgrade(),
            active: Cell::new(active),
            retired: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
        })
    }
    fn owns(&self, root: &Ref<FerroObject>) -> bool {
        self.owner.upgrade().is_some_and(|owner| owner.ptr_eq(root))
    }
    pub fn current(root: &Ref<FerroObject>) -> Rc<Self> {
        CURRENT.with(|current| {
            let mut entries = current.borrow_mut();
            entries.retain(|entry| {
                entry
                    .upgrade()
                    .is_some_and(|generation| generation.owner.upgrade().is_some())
            });
            if let Some(generation) = entries
                .iter()
                .filter_map(Weak::upgrade)
                .find(|generation| generation.owns(root))
            {
                return generation;
            }
            let generation = Self::new(root, true);
            entries.push(Rc::downgrade(&generation));
            generation
        })
    }
    fn publish(self: &Rc<Self>) {
        CURRENT.with(|current| {
            let mut entries = current.borrow_mut();
            entries.retain(|entry| {
                entry.upgrade().is_some_and(|generation| {
                    generation
                        .owner
                        .upgrade()
                        .is_some_and(|root| !self.owns(&root))
                })
            });
            entries.push(Rc::downgrade(self));
        });
        self.active.set(true);
    }
    fn subscriptions(&self) -> Vec<Rc<Subscription>> {
        let mut entries = self.subscriptions.borrow_mut();
        let live = entries.iter().filter_map(Weak::upgrade).collect();
        entries.retain(|entry| entry.strong_count() > 0);
        live
    }
    fn retire(&self) {
        self.active.set(false);
        self.retired.set(true);
        for subscription in self.subscriptions() {
            subscription.detach();
        }
    }
}
struct Subscription {
    generation: Rc<EventGeneration>,
    target: RefCell<WeakRef<Interactive>>,
    event: RoutedEvent,
    handler: MarkupDelegate,
    routes: RoutingStrategies,
    handled: bool,
    token: Cell<Option<RoutedEventHandlerToken>>,
}
impl Subscription {
    fn attach(self: &Rc<Self>, target: &Ref<Interactive>) {
        self.detach();
        *self.target.borrow_mut() = target.downgrade();
        let state = self.clone();
        let handler = MarkupDelegate::new(move |arguments| {
            if state.generation.active.get() && !state.generation.retired.get() {
                state.handler.invoke(arguments)
            } else {
                None
            }
        });
        self.token.set(Some(target.add_handler_untyped(
            &self.event,
            handler,
            self.routes,
            self.handled,
        )));
    }
    fn detach(&self) {
        let token = self.token.take();
        let target = self.target.borrow().upgrade();
        if let (Some(target), Some(token)) = (target, token) {
            target.remove_handler(&self.event, token);
        }
    }
}
/// Called only by compiled/runtime XAML event setters, with the document context.
pub fn add_handler(
    context: &XamlIlContext,
    target: &Ref<Interactive>,
    event: &RoutedEvent,
    handler: MarkupDelegate,
    routes: RoutingStrategies,
    handled: bool,
) {
    let Some(generation) = context.extension::<EventGeneration>() else {
        target.add_handler_untyped(event, handler, routes, handled);
        return;
    };
    if generation.retired.get() {
        return;
    }
    let subscription = Rc::new(Subscription {
        generation: generation.clone(),
        target: RefCell::new(target.downgrade()),
        event: event.clone(),
        handler,
        routes,
        handled,
        token: Cell::new(None),
    });
    generation
        .subscriptions
        .borrow_mut()
        .push(Rc::downgrade(&subscription));
    subscription.attach(target);
}
/// Initialize once, before deferred factories capture the context. Rebasing a
/// candidate root must keep its generation, not adopt the old live generation.
pub(crate) fn initialize(context: &XamlIlContext, root: &MarkupValue) {
    if context.extension::<EventGeneration>().is_some() {
        return;
    }
    let Some(root) = root.as_ref().and_then(Ref::<FerroObject>::from_xaml_object) else {
        return;
    };
    let parent = context.parent_service_provider();
    let generation = parent
        .as_ref()
        .and_then(|parent| {
            let code: Option<Rc<ReloadCodeBehind>> = parent.get_service_of();
            code.filter(|code| code.root_for(&root).is_some())
                .map(|code| code.generation.clone())
                .or_else(|| {
                    parent
                        .get_service_of::<Rc<EventGeneration>>()
                        .filter(|generation| generation.owns(&root))
                })
        })
        .unwrap_or_else(|| EventGeneration::current(&root));
    context.set_extension(generation);
}
/// The preserved instance used for code-behind method resolution. It is separate
/// from the detached population target; neither constructors nor live setters run.
pub struct ReloadCodeBehind {
    root: WeakRef<FerroObject>,
    candidate: WeakRef<FerroObject>,
    generation: Rc<EventGeneration>,
}
impl ReloadCodeBehind {
    pub fn new(root: &Ref<FerroObject>, candidate: &Ref<FerroObject>) -> Rc<Self> {
        Rc::new(Self {
            root: root.downgrade(),
            candidate: candidate.downgrade(),
            generation: EventGeneration::new(root, false),
        })
    }
    pub fn root_for(&self, object: &Ref<FerroObject>) -> Option<Ref<FerroObject>> {
        let root = self.root.upgrade()?;
        if object.ptr_eq(&root)
            || self
                .candidate
                .upgrade()
                .is_some_and(|candidate| candidate.ptr_eq(object))
        {
            Some(root)
        } else {
            None
        }
    }
    pub fn services(
        self: &Rc<Self>,
        fallback: Rc<dyn IServiceProvider>,
    ) -> Rc<dyn IServiceProvider> {
        Rc::new(Services {
            code: self.clone(),
            fallback,
        })
    }
    pub fn prepare(self: &Rc<Self>, old: Rc<EventGeneration>) -> Result<HandlerChange, String> {
        let root = self
            .root
            .upgrade()
            .and_then(|root| root.cast::<Interactive>())
            .ok_or("Component closed")?;
        let candidate = self.candidate.upgrade().ok_or("Candidate closed")?;
        let root_handlers: Vec<_> = self
            .generation
            .subscriptions()
            .into_iter()
            .filter(|subscription| {
                subscription
                    .target
                    .borrow()
                    .upgrade()
                    .is_some_and(|target| target.upcast::<FerroObject>().ptr_eq(&candidate))
            })
            .collect();
        for subscription in &root_handlers {
            subscription.detach();
        }
        Ok(HandlerChange {
            root: root.downgrade(),
            old,
            new: self.generation.clone(),
            root_handlers,
            started: false,
            accepted: false,
        })
    }
}
struct Services {
    code: Rc<ReloadCodeBehind>,
    fallback: Rc<dyn IServiceProvider>,
}
impl IServiceProvider for Services {
    fn get_service(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        service(kind, || self.code.clone()).or_else(|| self.fallback.get_service(kind))
    }
}
/// Translate only this component's root; nested component contexts keep their own
/// roots and subscriptions. Deferred contexts inherit the same weak service.
pub fn code_behind(provider: &dyn IServiceProvider, root: &MarkupValue) -> Option<BoxedValue> {
    let code: Rc<ReloadCodeBehind> = provider.get_service_of()?;
    let root = Ref::<FerroObject>::from_xaml_object(root.as_ref()?)?;
    code.root_for(&root).map(|root| Rc::new(root) as BoxedValue)
}
/// Reversible root-handler transfer. New callbacks remain inactive through commit;
/// finish activates them and retires the old generation after the entire revision.
pub struct HandlerChange {
    root: WeakRef<Interactive>,
    old: Rc<EventGeneration>,
    new: Rc<EventGeneration>,
    root_handlers: Vec<Rc<Subscription>>,
    started: bool,
    accepted: bool,
}
impl HandlerChange {
    pub fn commit(&mut self) -> Result<(), String> {
        let root = self.root.upgrade().ok_or("Component closed")?;
        self.old.active.set(false);
        self.started = true;
        for subscription in &self.root_handlers {
            subscription.attach(&root);
        }
        Ok(())
    }
    pub fn rollback(&mut self) {
        if self.started {
            for subscription in &self.root_handlers {
                subscription.detach();
            }
            self.old.active.set(true);
            self.started = false;
        }
    }
    pub fn finish(&mut self) -> Rc<EventGeneration> {
        self.old.retire();
        self.new.publish();
        self.accepted = true;
        self.new.clone()
    }
}
impl Drop for HandlerChange {
    fn drop(&mut self) {
        if !self.accepted {
            self.new.retire();
        }
    }
}

/// Only weak component identities and the subscription generation cross a
/// deferred declaration boundary. Do not retain the declaring service provider:
/// it can own the root that owns this template.
#[derive(Clone, Default)]
pub(crate) struct DeferredEventContext {
    generation: Option<Rc<EventGeneration>>,
    code: Option<Rc<ReloadCodeBehind>>,
}
impl DeferredEventContext {
    pub(crate) fn capture(provider: &dyn IServiceProvider) -> Self {
        Self {
            generation: provider.get_service_of(),
            code: provider.get_service_of(),
        }
    }
    pub(crate) fn get_service(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.generation
            .as_ref()
            .and_then(|value| service(kind, || value.clone()))
            .or_else(|| {
                self.code
                    .as_ref()
                    .and_then(|value| service(kind, || value.clone()))
            })
    }
}

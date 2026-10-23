//! Data templates whose items are components ([`template`]).

use crate::component_host::{ComponentHost, HostPool};
use crate::element::{rerender, Element, IntoElement};
use crate::hooks::{use_data_context, use_hook};
use crate::reactive::Owner;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::Rc;

/// A data template whose item is a component: for the `ItemTemplate` of an
/// items control or the `ContentTemplate` of a content control.
///
/// ```ignore
/// list_box()
///     .items_source(people)
///     .item_template(template(|person: Person| rsx! { TextBlock { text: person.name } }))
/// ```
///
/// Each item container gets a [`ComponentHost`] whose component runs `view`
/// with the item (the `DataContext` of the container). The template matches
/// data of type `T`. What `view` makes sees the contexts of the owner that was
/// current when the template was made.
///
/// The template recycles. A host that leaves the visual tree (a virtualizing
/// panel took its container away, a list removed its item) keeps its
/// component mounted, with its last item, and waits in a pool of the
/// template. The next item that needs a host takes one from the pool: its
/// component runs again with the new item and the reconciler patches its
/// controls, so scrolling a long list makes no new controls once the pool has
/// the hosts of a screen. The pool ends its hosts when the template is
/// dropped.
pub fn template<T, E>(view: impl Fn(T) -> E + 'static) -> Rc<dyn IDataTemplate>
where
    T: Clone + 'static,
    E: IntoElement,
{
    template_of("ComponentHost", view)
}

/// [`template`] whose hosts the inspector calls `origin`.
pub(crate) fn template_of<T, E>(origin: &'static str, view: impl Fn(T) -> E + 'static) -> Rc<dyn IDataTemplate>
where
    T: Clone + 'static,
    E: IntoElement,
{
    let view = Rc::new(view);
    let item_view: Rc<dyn Fn() -> Element> = Rc::new(move || {
        let view = view.clone();
        rerender(move || {
            // A host waiting in the pool has no data context: it keeps
            // showing its last item until it is given the next one.
            let last = use_hook(|| Rc::new(RefCell::new(None::<T>)));
            let item = use_data_context::<T>();
            if let Some(item) = &item {
                *last.borrow_mut() = Some(item.clone());
            }
            match item.or_else(|| last.borrow().clone()) {
                Some(item) => view(item).into_element(),
                None => Element::Empty,
            }
        })
    });
    Rc::new(ComponentTemplate {
        matches: |data| {
            data.is_some_and(|data| {
                let value: &dyn ferroui_base::AnyValue = &**data;
                value.downcast_ref::<T>().is_some()
            })
        },
        view: item_view,
        owner: Owner::current(),
        pool: Rc::new(RefCell::new(Vec::new())),
        origin,
    })
}

struct ComponentTemplate {
    matches: fn(Option<&BoxedValue>) -> bool,
    view: Rc<dyn Fn() -> Element>,
    owner: Option<Owner>,
    pool: Rc<HostPool>,
    origin: &'static str,
}

impl Drop for ComponentTemplate {
    fn drop(&mut self) {
        let hosts = std::mem::take(&mut *self.pool.borrow_mut());
        for host in hosts {
            host.unmount();
        }
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ComponentTemplate {
    fn build(&self, _data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        let pooled = self.pool.borrow_mut().pop();
        if let Some(host) = pooled {
            return Some(host.upcast());
        }
        let host = ComponentHost::new();
        host.set_origin(self.origin);
        host.set_pool(&self.pool);
        host.set_parent_owner(self.owner);
        let view = self.view.clone();
        host.set_view(move || view());
        Some(host.upcast())
    }
}

impl IDataTemplate for ComponentTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        (self.matches)(data)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for ComponentTemplate {
    fn build_with_existing(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        // The data context of a kept host is set by the caller: its
        // component runs again with the new item.
        match existing {
            Some(existing) if existing.cast::<ComponentHost>().is_some() => Some(existing),
            _ => self.build(&data.cloned()),
        }
    }
}

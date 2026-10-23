//! Port of `Diagnostics/Controls/TopLevelGroup.cs`.

use crate::diagnostics::IDevToolsTopLevelGroup;
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt, Ref};
use ferroui_controls::TopLevel;
use std::rc::Rc;

/// The object DevTools shows at the root of its trees for a group of top levels: the top levels
/// of the group, and the notifications of the top levels that join and leave it.
#[repr(C)]
pub struct TopLevelGroup {
    base: FerroObject,
    group: Rc<dyn IDevToolsTopLevelGroup>,
    added: HandlerList<dyn Fn(&Ref<TopLevel>)>,
    removed: HandlerList<dyn Fn(&Ref<TopLevel>)>,
}

ferro_class!(TopLevelGroup: FerroObject);
ferro_class_info!(TopLevelGroup {});

impl FerroObjectImpl for TopLevelGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        if let Some(incc) = this.group.observable_items() {
            let weak = this.to_ref().downgrade();
            incc.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<'_, Ref<TopLevel>>| {
                let Some(this) = weak.upgrade() else {
                    return;
                };

                for old_item in args.old_items {
                    for (_, handler) in this.removed.snapshot().iter() {
                        handler(old_item);
                    }
                }

                for new_item in args.new_items {
                    for (_, handler) in this.added.snapshot().iter() {
                        handler(new_item);
                    }
                }
            }));
        }
    }
}

impl TopLevelGroup {
    pub fn construct(group: Rc<dyn IDevToolsTopLevelGroup>) -> Self {
        Self { base: FerroObject::construct(), group, added: HandlerList::new(), removed: HandlerList::new() }
    }

    /// C# `new TopLevelGroup(group)`.
    pub fn new(group: Rc<dyn IDevToolsTopLevelGroup>) -> Ref<Self> {
        instantiate(Self::construct(group))
    }

    pub fn group(&self) -> Rc<dyn IDevToolsTopLevelGroup> {
        self.group.clone()
    }

    pub fn items(&self) -> Vec<Ref<TopLevel>> {
        self.group.items()
    }

    /// Raised when a top level joins the group. Disposing the returned handle unsubscribes.
    pub fn added(&self, handler: impl Fn(&Ref<TopLevel>) + 'static) -> Rc<dyn IDisposable> {
        let token = self.added.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.added.remove(token);
            }
        })
    }

    /// Raised when a top level leaves the group. Disposing the returned handle unsubscribes.
    pub fn removed(&self, handler: impl Fn(&Ref<TopLevel>) + 'static) -> Rc<dyn IDisposable> {
        let token = self.removed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.removed.remove(token);
            }
        })
    }
}

use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::popup_impl::PopupImpl;
use crate::top_level_impl::{
    impl_top_level_contract, MacOSTopLevelHandle, TopLevelEvents, TopLevelImpl, TopLevelParent,
};
use ferroui_controls::platform::{IPopupImpl, ITopLevelImpl};
use ferroui_microcom::ComPtr;
use std::rc::{Rc, Weak};

/// A top-level without a window of its own: a view that a host embeds.
pub struct EmbeddableTopLevelImpl {
    weak_self: Weak<EmbeddableTopLevelImpl>,
    base: Rc<TopLevelImpl>,
}

impl EmbeddableTopLevelImpl {
    pub(crate) fn new(factory: ComPtr<IFerroNativeFactory>) -> Rc<EmbeddableTopLevelImpl> {
        let this = Rc::new_cyclic(|weak_self| EmbeddableTopLevelImpl {
            weak_self: weak_self.clone(),
            base: TopLevelImpl::new(factory.clone()),
        });

        let e = IFrnTopLevelEvents::from_impl(TopLevelEvents(this.clone()));
        let native = factory.create_top_level(Some(&e)).check().expect("the native top-level");
        this.base.init(MacOSTopLevelHandle::from_top_level(native));
        this
    }
}

impl TopLevelParent for EmbeddableTopLevelImpl {
    fn top_level(&self) -> &Rc<TopLevelImpl> {
        &self.base
    }

    fn create_popup_core(&self) -> Option<Rc<dyn IPopupImpl>> {
        let this: Rc<dyn ITopLevelImpl> = self.weak_self.upgrade()?;
        Some(PopupImpl::new(self.base.factory().clone(), this))
    }
}

impl_top_level_contract!(EmbeddableTopLevelImpl {});

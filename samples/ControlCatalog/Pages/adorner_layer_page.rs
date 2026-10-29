//! Port of `Pages/AdornerLayerPage.xaml.cs`: the class of the document
//! `Pages/AdornerLayerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Button, ContentPage, Control};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct AdornerLayerPage {
    base: ContentPage,
    adorner: RefCell<Option<Ref<Control>>>,
}

content_page_class!(AdornerLayerPage);
ferro_class_info!(AdornerLayerPage {
    new: AdornerLayerPage::new,
    markup: {
        methods: [
            fn RemoveAdorner_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AdornerLayerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.remove_adorner_on_click(&sender, e.as_routed_event_args())
                },
            fn AddAdorner_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AdornerLayerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_adorner_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(AdornerLayerPage, "/Pages/AdornerLayerPage.xaml");

impl AdornerLayerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), adorner: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn adorner_button(&self) -> Ref<Button> {
        self.get_control::<Button>("AdornerButton")
    }

    fn remove_adorner_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let adorner_button = self.adorner_button();
        let adorner = AdornerLayer::get_adorner(&adorner_button);
        if let Some(adorner) = adorner {
            *self.adorner.borrow_mut() = Some(adorner);
        }
        AdornerLayer::set_adorner(&adorner_button, None::<Ref<Control>>);
    }

    fn add_adorner_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            AdornerLayer::set_adorner(&self.adorner_button(), adorner);
        }
    }
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
mod reload {
    use super::*;
    use ferroui_base::{controls::NameScopeRef, StyledElement, WeakRef};
    use ferroui_hot_reload::{ComponentRefresh, PreparedReload, ReloadError};

    struct Refresh(WeakRef<AdornerLayerPage>);
    struct Change {
        page: Ref<AdornerLayerPage>,
        before: Option<Ref<Control>>,
    }
    impl ComponentRefresh for Refresh {
        fn prepare(&self, _: &Ref<StyledElement>, scope: Option<&NameScopeRef>) -> Result<Box<dyn PreparedReload>, ReloadError> {
            let page = self.0.upgrade().ok_or_else(|| ReloadError::Failed("AdornerLayerPage closed".into()))?;
            if scope.and_then(|scope| scope.find("AdornerButton")).and_then(|control| control.cast::<Button>()).is_none() {
                return Err(ReloadError::Failed("AdornerButton must remain a Button".into()));
            }
            let before = page.adorner.borrow().clone();
            Ok(Box::new(Change { page, before }))
        }
    }
    impl PreparedReload for Change {
        fn commit(&mut self) -> Result<(), ReloadError> {
            // The new markup owns the new adorner; a previously removed adorner
            // belongs to the retired child graph and must not be attached to it.
            self.page.adorner.borrow_mut().take();
            Ok(())
        }
        fn rollback(&mut self) { *self.page.adorner.borrow_mut() = self.before.clone(); }
        fn finish(&mut self) { self.before.take(); }
    }
    impl AdornerLayerPage {
        pub(crate) fn reload_refresh(&self) -> Rc<dyn ComponentRefresh> {
            Rc::new(Refresh(self.to_ref().downgrade()))
        }
    }
}

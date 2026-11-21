// Copyright (c) 2026 Wiesław Šoltés. MIT. Native view of the original ToolsPage.
use crate::{
    demo_settings::DemoSettings,
    office_view_models::{ToolsAction, ToolsViewModel},
};
use ferroribbon::controls::{RibbonItemInvokedEventArgs, RibbonToggleButton, RibbonToolBar};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, UserControl, primitives::TemplatedControlImpl,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct ToolsPage {
    base: UserControl,
    model: OnceCell<Rc<ToolsViewModel>>,
    settings: OnceCell<Rc<DemoSettings>>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
}
ferro_class!(ToolsPage:UserControl);
ferro_class_info!(ToolsPage{new:ToolsPage::new,markup:{namespace:"FerroRibbon.Sample.Pages"}});
ferro_impl_classes!(ToolsPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ToolsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            settings: OnceCell::new(),
            cleanup: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("Tools populate override");
        } else {
            crate::generated::compiled_tools_page::populate(None, &this)
                .expect("compiled original ToolsPage");
        }
        let weak = Rc::downgrade(&settings);
        let model = ToolsViewModel::new(Rc::new(move |line| {
            weak.upgrade()
                .ok_or("Tools settings owner is gone")?
                .write(Some(line))
                .map_err(|e| e.to_string())
        }));
        assert!(this.model.set(model.clone()).is_ok());
        assert!(this.settings.set(settings).is_ok());
        this.set_data_context(Some(model));
        // Subscribe to exactly the constructor's snapshot, matching source OfType.
        for button in this
            .tools()
            .items()
            .iter()
            .flatten()
            .filter_map(|c| c.cast::<RibbonToggleButton>())
        {
            let weak = this.downgrade();
            let checked = MarkupDelegate::new(move |args| {
                if let Some(page) = weak.upgrade() {
                    let button = from_markup_value::<Ref<RibbonToggleButton>>(&args[0])
                        .expect("source Checked sender");
                    let context = button
                        .tag()
                        .and_then(|t| t.downcast_ref::<String>().cloned());
                    let label = Rc::new(move || button.label());
                    page.view_model()
                        .action()
                        .execute(Some(&Some(ToolsAction::Checked { context, label })));
                }
                None
            });
            button.add_checked(Some(checked.clone()));
            let weak = button.downgrade();
            this.cleanup.borrow_mut().push(Box::new(move || {
                if let Some(button) = weak.upgrade() {
                    button.remove_checked(Some(checked));
                }
            }));
        }
        let weak = this.downgrade();
        let invoked = MarkupDelegate::new(move |args| {
            let e = from_markup_value::<Rc<RibbonItemInvokedEventArgs>>(&args[1])
                .expect("Tools invocation");
            if let Some(page) = weak.upgrade() {
                let label = super::cad_session::ribbon_item(&e.item)
                    .and_then(|item| item.label())
                    .map(|s| s.to_utf8().expect("native tool label"));
                page.view_model()
                    .action()
                    .execute(Some(&Some(ToolsAction::Invoked { label })));
            }
            None
        });
        this.tools().add_item_invoked(Some(invoked.clone()));
        let weak = this.tools().downgrade();
        this.cleanup.borrow_mut().push(Box::new(move || {
            if let Some(toolbar) = weak.upgrade() {
                toolbar.remove_item_invoked(Some(invoked));
            }
        }));
        this
    }
    pub fn tools(&self) -> Ref<RibbonToolBar> {
        self.named::<RibbonToolBar>("Tools")
    }
    pub fn view_model(&self) -> Rc<ToolsViewModel> {
        self.model.get().expect("Tools initialized").clone()
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Missing Tools element {name}"))
    }
}
impl Drop for ToolsPage {
    fn drop(&mut self) {
        for cleanup in self.cleanup.get_mut().drain(..) {
            cleanup();
        }
    }
}

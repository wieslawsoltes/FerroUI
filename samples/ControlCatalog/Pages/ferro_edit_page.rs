//! Hosts the shared editor sample inside the catalog.
use crate::markup::{content_page_class, xaml_class};
use editor_demo::EditorDemo;
use ferroui_base::{Ref, ferro_class_info, instantiate};
use ferroui_controls::{Border, ContentPage, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct FerroEditPage {
    base: ContentPage,
    demo: RefCell<Option<Rc<EditorDemo>>>,
}

content_page_class!(FerroEditPage);
ferro_class_info!(FerroEditPage {
    new: FerroEditPage::new
});
xaml_class!(FerroEditPage, "/Pages/FerroEditPage.xaml");

impl FerroEditPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            demo: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        #[cfg(not(target_arch = "wasm32"))]
        this.show_editor();
        #[cfg(target_arch = "wasm32")]
        {
            use ferroui_edit::textmate::{browser_oniguruma_ready, initialize_browser_oniguruma};

            if browser_oniguruma_ready() {
                this.show_editor();
            } else {
                this.show_status("Loading syntax highlighting…");
                let page = this.downgrade();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = initialize_browser_oniguruma().await;
                    if let Some(page) = page.upgrade() {
                        match result {
                            Ok(()) => page.show_editor(),
                            Err(error) => page.show_status(&format!(
                                "Syntax highlighting could not load. {error}"
                            )),
                        }
                    }
                });
            }
        }
        this
    }

    fn show_editor(&self) {
        match EditorDemo::new(false) {
            Ok(demo) => {
                self.get_control::<Border>("DemoHost")
                    .set_child(Some(demo.view().upcast()));
                *self.demo.borrow_mut() = Some(demo);
            }
            Err(error) => self.show_status(&format!("FerroEdit could not load. {error}")),
        }
    }

    fn show_status(&self, message: &str) {
        let status = TextBlock::new();
        status.set_text(Some(message));
        self.get_control::<Border>("DemoHost")
            .set_child(Some(status.upcast()));
    }
}

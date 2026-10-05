//! Port of `Pages/TabbedPage/TabbedPageProgrammaticPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageProgrammaticPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, ContentPage, Page, PageSelectionChangedEventArgs, TabbedPage, TextBlock, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `(page as ContentPage)?.Header?.ToString() ?? "—"`.
fn header_text(page: Option<Ref<Page>>) -> String {
    page.and_then(|page| page.cast::<ContentPage>())
        .and_then(|page| value_text(&page.header()))
        .unwrap_or_else(|| String::from("\u{2014}"))
}

#[repr(C)]
pub struct TabbedPageProgrammaticPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    log: RefCell<Vec<String>>,
}

user_control_class!(TabbedPageProgrammaticPage);
ferro_class_info!(TabbedPageProgrammaticPage {
    new: TabbedPageProgrammaticPage::new,
    markup: {
        methods: [
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageProgrammaticPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
            fn OnJumpTo(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageProgrammaticPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_jump_to(&sender, e.as_routed_event_args())
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageProgrammaticPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageProgrammaticPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageProgrammaticPage, "/Pages/TabbedPage/TabbedPageProgrammaticPage.xaml");

impl TabbedPageProgrammaticPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            log: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    /// A named text block: null until `InitializeComponent()` has returned.
    fn text(&self, name: &str) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>(name))
    }

    fn demo_tabs(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("DemoTabs")
    }

    fn on_selection_changed(&self, _sender: &Option<BoxedValue>, e: &PageSelectionChangedEventArgs) {
        self.update_current_label();

        let from = header_text(e.previous_page());
        let to = header_text(e.current_page());
        let text = {
            let mut log = self.log.borrow_mut();
            log.insert(0, format!("{from} \u{2192} {to}"));
            if log.len() > 6 {
                log.pop();
            }
            log.join("\n")
        };

        if let Some(selection_log) = self.text("SelectionLog") {
            selection_log.set_text(Some(&text));
        }
    }

    fn update_current_label(&self) {
        // The fields `CurrentTabLabel` and `DemoTabs` are set together.
        let Some(current_tab_label) = self.text("CurrentTabLabel") else {
            return;
        };
        let demo_tabs = self.demo_tabs();
        let idx = demo_tabs.selected_index();
        let name = header_text(demo_tabs.selected_page());
        current_tab_label.set_text(Some(&format!("Index {idx} | {name}")));
    }

    fn on_jump_to(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let index = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>())
            .and_then(|btn| value_text(&btn.tag()))
            .and_then(|tag| tag.trim().parse::<i32>().ok());
        if let Some(index) = index {
            self.demo_tabs().set_selected_index(index);
        }
    }

    fn on_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_tabs = self.demo_tabs();
        if demo_tabs.selected_index() > 0 {
            demo_tabs.set_selected_index(demo_tabs.selected_index() - 1);
        }
    }

    fn on_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_tabs = self.demo_tabs();
        if demo_tabs.selected_index() < 3 {
            demo_tabs.set_selected_index(demo_tabs.selected_index() + 1);
        }
    }
}

//! Port of `Pages/NavigationPage/NavigationPageStackPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageStackPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, value_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, TextTrimming};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref, Thickness};
use ferroui_controls::{
    Border, Button, Dock, DockPanel, NavigationPage, Page, StackPanel, TextBlock, ToolTip, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageStackPage {
    base: UserControl,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageStackPage);
ferro_class_info!(NavigationPageStackPage {
    new: NavigationPageStackPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageStackPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnInsert(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageStackPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_insert(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageStackPage, "/Pages/NavigationPage/NavigationPageStackPage.xaml");

impl NavigationPageStackPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn depth_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DepthLabel")
    }

    fn stack_display(&self) -> Ref<StackPanel> {
        self.get_control::<StackPanel>("StackDisplay")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);

        // The handlers belong to a child of the control: they hold the control weakly.
        let demo_nav = self.demo_nav();
        let refresh = || {
            let weak = self.to_ref().downgrade();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.refresh_stack();
                }
            }
        };
        let pushed = refresh();
        demo_nav.pushed(move |_ev| pushed());
        let popped = refresh();
        demo_nav.popped(move |_ev| popped());
        let popped_to_root = refresh();
        demo_nav.popped_to_root(move |_ev| popped_to_root());
        let page_inserted = refresh();
        demo_nav.page_inserted(move |_ev| page_inserted());
        let page_removed = refresh();
        demo_nav.page_removed(move |_ev| page_removed());

        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page = NavigationDemoHelper::make_page("Home", &format!("Stack position #{page_count}"), page_count);
        // The stack is refreshed by the handler of the pushed event above.
        drop(demo_nav.push_async_with_transition(page, None));
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page = NavigationDemoHelper::make_page(
            &format!("Page {page_count}"),
            &format!("Stack position #{page_count}"),
            page_count,
        );
        drop(self.demo_nav().push_async(page));
    }

    fn on_insert(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_nav = self.demo_nav();
        let Some(current) = demo_nav.current_page().filter(|_| demo_nav.stack_depth() > 1) else {
            return;
        };

        self.insert_before(&current);
    }

    /// Inserts a new page before `page` and refreshes the display of the stack.
    fn insert_before(&self, page: &Ref<Page>) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let inserted = NavigationDemoHelper::make_page(
            &format!("Inserted {page_count}"),
            &format!("Stack position #{page_count}"),
            page_count,
        );
        self.demo_nav().insert_page(inserted, page.clone());
        self.refresh_stack();
    }

    fn refresh_stack(&self) {
        let demo_nav = self.demo_nav();
        let stack = demo_nav.navigation_stack();
        let depth = stack.len();
        let current = demo_nav.current_page();

        self.depth_label().set_text(Some(&format!("depth: {depth}")));
        let stack_display = self.stack_display();
        stack_display.children().clear();

        // Render from top (current) down to root.
        for i in (0..depth).rev() {
            let page = &stack[i];
            let is_current = current.as_ref().is_some_and(|current| current.ptr_eq(page));
            let is_root = i == 0;

            stack_display.children().add(self.build_stack_entry(page, i as i32 + 1, is_current, is_root));
        }
    }

    fn build_stack_entry(&self, page: &Ref<Page>, position: i32, is_current: bool, is_root: bool) -> Ref<Border> {
        let background = NavigationDemoHelper::get_page_brush(position - 1);

        let position_text = TextBlock::new();
        position_text.set_text(Some(&position.to_string()));
        position_text.set_font_size(11.0);
        position_text.set_font_weight(FontWeight::SemiBold);
        position_text.set_horizontal_alignment(HorizontalAlignment::Center);
        position_text.set_vertical_alignment(VerticalAlignment::Center);
        let badge = Border::new();
        badge.set_width(24.0);
        badge.set_height(24.0);
        badge.set_corner_radius(CornerRadius::uniform(12.0));
        badge.set_background(Some(background));
        badge.set_vertical_alignment(VerticalAlignment::Center);
        badge.set_child(position_text);

        let title = TextBlock::new();
        title.set_text(Some(&value_text(&page.header()).unwrap_or_else(|| String::from("(untitled)"))));
        title.set_font_weight(if is_current { FontWeight::SemiBold } else { FontWeight::Normal });
        title.set_vertical_alignment(VerticalAlignment::Center);
        title.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        title.set_margin(Thickness::new(6.0, 0.0, 0.0, 0.0));

        let badge_text = if is_current {
            Some("current")
        } else if is_root {
            Some("root")
        } else {
            None
        };
        let badge_label = TextBlock::new();
        badge_label.set_text(badge_text);
        badge_label.set_font_size(10.0);
        badge_label.set_opacity(0.5);
        badge_label.set_vertical_alignment(VerticalAlignment::Center);
        badge_label.set_is_visible(badge_text.is_some());
        badge_label.set_margin(Thickness::new(4.0, 0.0, 0.0, 0.0));

        // Remove button (disabled when it is the only page in the stack)
        let remove_btn = Button::new();
        remove_btn.set_content(Some(boxed_text("Remove")));
        remove_btn.set_font_size(11.0);
        remove_btn.set_padding(Thickness::symmetric(6.0, 2.0));
        remove_btn.set_vertical_alignment(VerticalAlignment::Center);
        remove_btn.set_is_enabled(!(is_root && self.demo_nav().stack_depth() == 1));
        // The buttons are descendants of the control: their handlers hold it weakly.
        let weak = self.to_ref().downgrade();
        let removed = page.clone();
        remove_btn.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.demo_nav().remove_page(removed.clone());
                this.refresh_stack();
            }
        });

        // Insert-before button (not shown for current page: use the dedicated button instead)
        let insert_btn = Button::new();
        insert_btn.set_content(Some(boxed_text("Insert \u{2191}")));
        insert_btn.set_font_size(11.0);
        insert_btn.set_padding(Thickness::symmetric(6.0, 2.0));
        insert_btn.set_vertical_alignment(VerticalAlignment::Center);
        insert_btn.set_is_visible(!is_current);
        ToolTip::set_tip(
            &insert_btn,
            Some(boxed_text(&format!(
                "Insert a new page before \"{}\"",
                value_text(&page.header()).unwrap_or_default()
            ))),
        );
        let weak = self.to_ref().downgrade();
        let before = page.clone();
        insert_btn.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.insert_before(&before);
            }
        });

        let buttons_panel = StackPanel::new();
        buttons_panel.set_orientation(Orientation::Horizontal);
        buttons_panel.set_spacing(4.0);
        buttons_panel.set_vertical_alignment(VerticalAlignment::Center);
        buttons_panel.children().add(remove_btn);
        buttons_panel.children().add(insert_btn);

        let title_row = DockPanel::new();
        title_row.set_last_child_fill(true);
        DockPanel::set_dock(&buttons_panel, Dock::Right);
        title_row.children().add(buttons_panel);
        title_row.children().add(badge);
        title_row.children().add(title);
        title_row.children().add(badge_label);

        let entry = Border::new();
        entry.set_border_brush(Some(
            SolidColorBrush::with_color(if is_current { parse_color("#0078D4") } else { parse_color("#CCCCCC") })
                .into(),
        ));
        entry.set_border_thickness(Thickness::uniform(if is_current { 2.0 } else { 1.0 }));
        entry.set_corner_radius(CornerRadius::uniform(6.0));
        entry.set_padding(Thickness::symmetric(8.0, 6.0));
        entry.set_child(title_row);
        entry
    }
}

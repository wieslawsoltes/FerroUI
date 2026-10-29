//! Not from upstream: the page of WinUI's NavigationView (docs/porting/winui-controls.md), the class
//! of `Pages/NavigationViewPage.xaml`. The left navigation view navigates a `Frame` to a page for the
//! selected item and goes back through its journal with the back button, as a WinUI app does; the
//! top and compact ones show the selected item in their content.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, Ref, Thickness};
use ferroui_controls::navigation::NavigationMode;
use ferroui_controls::{
    box_item, unbox_item, ContentPage, Control, Frame, FramePage, NavigationView, NavigationViewItem,
    NavigationViewSelectionChangedEventArgs, StackPanel, TextBlock,
};
use std::cell::Cell;

#[repr(C)]
pub struct NavigationViewPage {
    base: ContentPage,
    /// Set while the left navigation view follows a back navigation of its frame, so that its
    /// selection does not navigate again.
    following_frame: Cell<bool>,
}

content_page_class!(NavigationViewPage);
ferro_class_info!(NavigationViewPage { new: NavigationViewPage::new });
xaml_class!(NavigationViewPage, "/Pages/NavigationViewPage.xaml");

/// The text of the content for the item with the header `item`.
fn description(item: &str) -> &'static str {
    match item {
        "Home" => "The start page: what is new and what was opened last.",
        "Mail" => "The inbox, with the messages of all accounts.",
        "Calendar" => "The appointments of this week.",
        "Folder" => "The documents of the app.",
        "Favorites" => "The items marked with a star.",
        "Settings" => "The settings item of the pane footer (IsSettingsVisible).",
        _ => "",
    }
}

/// The header of the selected item: its content, or "Settings" for the settings item.
fn selected_header(args: &NavigationViewSelectionChangedEventArgs) -> Option<String> {
    if args.is_settings_selected() {
        return Some("Settings".to_string());
    }
    args.selected_item_container().and_then(|item| unbox_item::<String>(&item.content()))
}

/// The content of a page for `header`: a title and a line of text.
fn page_content(header: &str, journal: &str) -> Ref<StackPanel> {
    let title = TextBlock::new();
    title.set_text(Some(header));
    title.set_font_size(24.0);
    title.set_font_weight(FontWeight::SemiBold);
    let body = TextBlock::new();
    body.set_text(Some(description(header)));
    body.set_text_wrapping(TextWrapping::Wrap);
    let footer = TextBlock::new();
    footer.set_text(Some(journal));
    footer.set_opacity(0.6);
    let stack = StackPanel::new();
    stack.set_margin(Thickness::uniform(24.0));
    stack.set_spacing(8.0);
    stack.children().add(title);
    stack.children().add(body);
    stack.children().add(footer);
    stack
}

impl NavigationViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), following_frame: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        Self::set_up_frame_navigation(&this);
        for name in ["TopNav", "CompactNav"] {
            let Some(navigation) = this.find_control::<NavigationView>(name) else { continue };
            let weak = this.downgrade();
            navigation.selection_changed(move |args| {
                if let Some(this) = weak.upgrade() {
                    this.show(name, selected_header(args).as_deref());
                }
            });
            this.show(name, Some("Home"));
        }
        this
    }

    fn set_up_frame_navigation(this: &Ref<Self>) {
        let (Some(navigation), Some(frame)) =
            (this.find_control::<NavigationView>("LeftNav"), this.find_control::<Frame>("LeftFrame"))
        else {
            return;
        };
        // A new page of the frame gets the content of the item it was navigated to for.
        let weak_navigation = navigation.downgrade();
        let weak_frame = frame.downgrade();
        let weak = this.downgrade();
        frame.navigated(move |args| {
            let (Some(this), Some(navigation), Some(frame)) =
                (weak.upgrade(), weak_navigation.upgrade(), weak_frame.upgrade())
            else {
                return;
            };
            let header = unbox_item::<String>(&args.parameter()).unwrap_or_default();
            let journal = format!("Pages to go back to: {}", frame.back_stack_depth());
            if let Some(page) = args.content() {
                page.set_content(Some(Control::boxed(page_content(&header, &journal))));
            }
            navigation.set_is_back_enabled(frame.can_go_back());
            if args.navigation_mode() != NavigationMode::New {
                this.select_left_item(&navigation, &header);
            }
        });
        let weak_frame = frame.downgrade();
        let weak = this.downgrade();
        navigation.selection_changed(move |args| {
            let (Some(this), Some(frame)) = (weak.upgrade(), weak_frame.upgrade()) else { return };
            if this.following_frame.get() {
                return;
            }
            if let Some(header) = selected_header(args) {
                let _ = frame.navigate(FramePage::TYPE, Some(box_item(&header)));
            }
        });
        let weak_frame = frame.downgrade();
        navigation.back_requested(move |_| {
            if let Some(frame) = weak_frame.upgrade() {
                let _ = frame.go_back();
            }
        });
        let _ = frame.navigate(FramePage::TYPE, Some(box_item(&"Home".to_string())));
    }

    /// Selects the item of the left navigation view for `header` without navigating again.
    fn select_left_item(&self, navigation: &NavigationView, header: &str) {
        let item = if header == "Settings" {
            Some(navigation.settings_item())
        } else {
            self.find_control::<NavigationViewItem>(&format!("LeftNav{header}"))
        };
        if let Some(item) = item {
            self.following_frame.set(true);
            navigation.set_selected_item(Some(Control::boxed(item)));
            self.following_frame.set(false);
        }
    }

    fn show(&self, name: &str, header: Option<&str>) {
        let header = header.unwrap_or("");
        if let Some(title) = self.find_control::<TextBlock>(&format!("{name}Title")) {
            title.set_text(Some(header));
        }
        if let Some(body) = self.find_control::<TextBlock>(&format!("{name}Body")) {
            body.set_text(Some(description(header)));
        }
    }
}

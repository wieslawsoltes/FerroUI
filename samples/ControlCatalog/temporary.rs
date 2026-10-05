//! TEMPORARY: what lets the application start while parts of the real
//! shell do not load. Nothing here is a port; every item names what it
//! waits for and goes away with it.
//!
//! - [`load_app_document_subset`]: `App.xaml` without the elements that do
//!   not load yet ([`APP_DOCUMENT_REMOVALS`]), used while `App.xaml` is
//!   listed in `excluded.txt`.
//! - [`fluent_theme`] / [`initial_theme`]: the Fluent theme does not load
//!   yet, so the catalog starts with the Simple theme unless Fluent loads.
//! - [`shell`]: a code-built main window (a list of the pages that load
//!   next to a navigation page), used while `MainWindow.xaml` is listed in
//!   `excluded.txt` (its content, `MainView`, derives from `DrawerPage`,
//!   which is not ported).

use crate::app::App;
use crate::markup::try_load_text_group;
use crate::models::CatalogTheme;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, Ref};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_themes_fluent::FluentTheme;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// An element of `App.xaml` that is removed from the subset that loads:
/// what it waits for, and the text from `start` up to and including `end`
/// (each must occur exactly as written; whole lines are removed).
pub struct DocumentRemoval {
    /// The gap or missing type the element waits for.
    pub waits_for: &'static str,
    /// The first text of the element.
    pub start: &'static str,
    /// The last text of the element.
    pub end: &'static str,
}

/// The elements of `App.xaml` that do not load yet.
pub const APP_DOCUMENT_REMOVALS: &[DocumentRemoval] = &[
    DocumentRemoval {
        waits_for: "gap K-geometry-text: a StreamGeometry cannot be created from element text",
        start: "<StreamGeometry x:Key=\"SearchIcon\">",
        end: "</StreamGeometry>",
    },
    DocumentRemoval {
        waits_for: "gap K-geometry-text: a StreamGeometry cannot be created from element text",
        start: "<StreamGeometry x:Key=\"AppIcon\">",
        end: "</StreamGeometry>",
    },
    DocumentRemoval {
        waits_for: "missing: the assembly FerroUI.Controls.ColorPicker (its Fluent styles)",
        start: "<StyleInclude x:Key=\"ColorPickerFluent\"",
        end: "/>",
    },
    DocumentRemoval {
        waits_for: "missing: the assembly FerroUI.Controls.ColorPicker (its Simple styles)",
        start: "<StyleInclude x:Key=\"ColorPickerSimple\"",
        end: "/>",
    },
    DocumentRemoval {
        waits_for: "missing: RefreshContainer",
        start: "<Style Selector=\"RefreshContainer\">",
        end: "</Style>",
    },
    DocumentRemoval {
        waits_for: "gap C002: NativeMenuItem.Click cannot be assigned a handler from markup",
        start: "<NativeDock.Menu>",
        end: "</NativeDock.Menu>",
    },
    DocumentRemoval {
        waits_for: "gap C003: NativeMenuItem.Icon cannot be assigned from text (an item of the tray menu)",
        start: "<TrayIcon.Icons>",
        end: "</TrayIcon.Icons>",
    },
];

/// The elements of `CustomThemes.xaml` that do not load yet.
pub const CUSTOM_THEMES_REMOVALS: &[DocumentRemoval] = &[DocumentRemoval {
    waits_for: "gap K-transform-none (C100): a setter value is not converted to ITransform from text",
    start: "<Style Selector=\"^ /template/ ToggleButton#PART_Toggle:checked PathIcon#PART_Chevron\">",
    end: "</Style>",
}];

/// The rooted asset path of the dictionary `App.xaml` merges.
pub const CUSTOM_THEMES_PATH: &str = "/CustomThemes.xaml";

/// The text of `CustomThemes.xaml`: the subset of it that loads while the
/// document is listed in `excluded.txt`, else the document.
pub fn custom_themes_text() -> String {
    let xaml = crate::markup::embedded_text(CUSTOM_THEMES_PATH).expect("CustomThemes.xaml is embedded");
    if crate::excluded(CUSTOM_THEMES_PATH).is_some() {
        remove_elements(xaml, CUSTOM_THEMES_REMOVALS)
    } else {
        xaml.to_string()
    }
}

/// Loads `CustomThemes.xaml` (see [`custom_themes_text`]) on its own.
pub fn load_custom_themes() -> Result<BoxedValue, XamlLoadException> {
    crate::markup::try_load_text(&custom_themes_text(), Some(CUSTOM_THEMES_PATH), None)
}

/// `xaml` without the elements of `removals`.
///
/// # Panics
/// Panics when an element is not found: the table no longer matches the
/// document.
pub fn remove_elements(xaml: &str, removals: &[DocumentRemoval]) -> String {
    let mut text = xaml.to_string();
    for removal in removals {
        let start = text.find(removal.start).unwrap_or_else(|| panic!("the document has no {}", removal.start));
        let end = text[start..]
            .find(removal.end)
            .map(|offset| start + offset + removal.end.len())
            .unwrap_or_else(|| panic!("the element {} does not end with {}", removal.start, removal.end));
        // Whole lines: from the start of the first line to the end of the last one.
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
        text.replace_range(line_start..line_end, "");
    }
    text
}

/// Populates the application `root` from the subset of `App.xaml` that
/// loads, with the dictionary it includes.
pub fn load_app_document_subset(root: BoxedValue) -> Result<(), XamlLoadException> {
    let content = crate::assets::asset(App::DOCUMENT_PATH).expect("App.xaml is embedded");
    let xaml = std::str::from_utf8(content).expect("App.xaml is UTF-8");
    let subset = remove_elements(xaml, APP_DOCUMENT_REMOVALS);
    let custom_themes = custom_themes_text();
    try_load_text_group(&subset, App::DOCUMENT_PATH, Some(root), &[(CUSTOM_THEMES_PATH, &custom_themes)]).map(|_| ())
}

/// The resource `FluentTheme` of the application, when the theme loads.
///
/// The resource is created when it is first read, and the constructor of
/// the theme panics while its documents do not load; the panic is caught
/// here and reported as one line.
pub fn fluent_theme(app: &App) -> Option<Ref<FluentTheme>> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| app.resource("FluentTheme")));
    std::panic::set_hook(previous_hook);

    match result {
        Ok(value) => from_markup_value::<Ref<FluentTheme>>(&value),
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let first_line: String = message.lines().next().unwrap_or_default().chars().take(160).collect();
            eprintln!("ControlCatalog: the Fluent theme does not load yet ({first_line}); using the Simple theme");
            None
        }
    }
}

/// The theme the catalog starts with: the one `FERROUI_CATALOG_THEME`
/// (`fluent` or `simple`) asks for, else Fluent as upstream; Simple when
/// the Fluent theme is not available.
pub fn initial_theme(fluent_available: bool) -> CatalogTheme {
    let requested = match std::env::var("FERROUI_CATALOG_THEME").ok().as_deref() {
        Some("simple") => CatalogTheme::Simple,
        _ => CatalogTheme::Fluent,
    };
    if requested == CatalogTheme::Fluent && !fluent_available {
        CatalogTheme::Simple
    } else {
        requested
    }
}

/// The code-built main window.
pub mod shell {
    use crate::models::PageItem;
    use crate::view_models::MainWindowViewModel;
    use ferroui_base::media::FontWeight;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::reactive::IDisposable;
    use ferroui_controls::primitives::ScrollBarVisibility;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::{BoxedValue, Ref, Thickness, WeakRef};
    use std::cell::{Cell, RefCell};
    use std::time::Duration;
    use ferroui_controls::{
        Control, Dock, DockPanel, ListBox, ListBoxItem, NavigationPage, ScrollViewer, TextBlock,
        Window,
    };
    use std::rc::Rc;

    /// The parts of the shell.
    pub struct Shell {
        pub window: Ref<Window>,
        /// The list of the sections and their pages.
        pub page_list: Ref<ListBox>,
        /// The navigation page that shows the current page (the navigator of
        /// the view model).
        pub navigation_page: Ref<NavigationPage>,
        /// The page shown first, when there is one.
        pub first_page: Option<Rc<PageItem>>,
    }

    /// Creates the shell: a window with the list of the pages that load on
    /// the left and a navigation page that shows the selected page. The
    /// window title and sizes are the ones of `MainWindow.xaml`.
    pub fn create_shell(view_model: Rc<MainWindowViewModel>) -> Shell {
        let navigation_page = NavigationPage::new();
        view_model.set_navigator(Some(navigation_page.as_navigation()));

        let page_list = ListBox::new();
        let mut first_page = None;
        for section in view_model.home_sections().iter() {
            let items = section.items().unwrap_or_default();
            if items.is_empty() {
                continue;
            }

            let title = TextBlock::new();
            title.set_text(Some(section.title().as_str()));
            title.set_font_weight(FontWeight::SemiBold);
            let header = ListBoxItem::new();
            header.set_content(Some(Control::boxed(&title)));
            header.set_is_enabled(false);
            page_list.items().add(Some(Control::boxed(&header)));

            for item in items.iter() {
                let entry = ListBoxItem::new();
                entry.set_content(Some(Rc::new(item.header()) as BoxedValue));
                entry.set_padding(Thickness::new(20.0, 4.0, 8.0, 4.0));
                entry.set_tag(Some(item.clone() as BoxedValue));
                page_list.items().add(Some(Control::boxed(&entry)));
                first_page.get_or_insert_with(|| item.clone());
            }
        }

        // The pages the drawer of the main view shows outside the sections: the home page and the
        // settings page.
        for item in [view_model.home_item(), view_model.settings_item()] {
            let document = if Rc::ptr_eq(&item, &view_model.home_item()) {
                crate::pages::HomePage::DOCUMENT_PATH
            } else {
                crate::pages::SettingsPage::DOCUMENT_PATH
            };
            if crate::excluded(document).is_some() {
                continue;
            }
            let entry = ListBoxItem::new();
            entry.set_content(Some(Rc::new(item.header()) as BoxedValue));
            entry.set_tag(Some(item.clone() as BoxedValue));
            page_list.items().add(Some(Control::boxed(&entry)));
        }

        {
            let view_model = view_model.clone();
            page_list.selection_changed(move |_, e| {
                let selected = e.added_items().first().cloned().flatten();
                let entry = selected.as_ref().and_then(Control::from_boxed).and_then(|c| c.cast::<ListBoxItem>());
                if let Some(item) = entry.and_then(|entry| from_markup_value::<Rc<PageItem>>(&entry.tag())) {
                    view_model.navigate_to_item(&item);
                }
            });
        }

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll_viewer.set_width(260.0);
        scroll_viewer.set_content(Some(Control::boxed(&page_list)));
        DockPanel::set_dock(&scroll_viewer, Dock::Left);

        let panel = DockPanel::new();
        panel.children().add(scroll_viewer);
        panel.children().add(navigation_page.clone());

        let window = Window::new();
        window.set_title(Some("FerroUI Control Gallery".to_string()));
        window.set_min_width(500.0);
        window.set_min_height(300.0);
        window.set_width(1100.0);
        window.set_height(800.0);
        window.set_content(Some(Control::boxed(&panel)));
        window.set_data_context(Some(view_model as BoxedValue));

        Shell { window, page_list, navigation_page, first_page }
    }

    thread_local! {
        /// The page list of the shell window created last, for [`show_every_page`].
        static PAGE_LIST: RefCell<Option<WeakRef<ListBox>>> = const { RefCell::new(None) };
    }

    /// The shell window, showing the first page of the list.
    pub fn create_shell_window(view_model: Rc<MainWindowViewModel>) -> Ref<Window> {
        let shell = create_shell(view_model.clone());
        if let Some(first_page) = &shell.first_page {
            view_model.navigate_to_item(first_page);
        }
        PAGE_LIST.with(|list| *list.borrow_mut() = Some(shell.page_list.downgrade()));
        shell.window
    }

    /// Selects the pages of the shell one after the other, each for
    /// `interval`, and prints the header of each (a smoke run over every
    /// page that loads).
    pub fn show_every_page(interval: Duration) {
        let next = Cell::new(0usize);
        let timer: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let stop = timer.clone();
        let subscription = DispatcherTimer::run(
            move || {
                let Some(page_list) = PAGE_LIST.with(|list| list.borrow().as_ref().and_then(|list| list.upgrade())) else {
                    return true;
                };
                let entries = page_list.items().to_vec();
                let mut index = next.get();
                while index < entries.len() {
                    let entry = entries[index].as_ref().and_then(Control::from_boxed).and_then(|c| c.cast::<ListBoxItem>());
                    let item = entry.and_then(|entry| from_markup_value::<Rc<PageItem>>(&entry.tag()));
                    if let Some(item) = item {
                        println!("Selecting {}", item.header());
                        page_list.set_selected_index(index as i32);
                        next.set(index + 1);
                        return true;
                    }
                    index += 1;
                }
                println!("Selected every page");
                stop.borrow_mut().take();
                false
            },
            interval,
            DispatcherPriority::NORMAL,
        );
        *timer.borrow_mut() = Some(subscription);
    }
}

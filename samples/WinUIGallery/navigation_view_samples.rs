//! The eight original NavigationView examples and their page handlers.
//!
//! Category selection keeps the original model identity. A native FuncDataTemplate
//! dispatches the Gallery's Category/Header/Separator models; it is an explicit
//! adaptation of its custom DataTemplateSelector and x:Bind expressions. The
//! adaptive trigger reads the host's client width, or the sample bounds when
//! there is no host. Navigation transition animations remain unsupported.

use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    Ref, StyledElementImpl, TypeInfo, Visual, VisualImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::navigation::NavigationError;
use ferroui_controls::primitives::{TemplatedControlImpl, ToggleButton};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    AutoSuggestBox, CheckBox, ContentControlImpl, Control, ControlImpl, Frame, FramePage,
    FramePageImpl, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem,
    NavigationViewItemHeader, NavigationViewItemSeparator, NavigationViewPaneDisplayMode,
    NavigationViewSelectionFollowsFocus, RadioButton, StackPanel, Symbol, SymbolIcon,
    TextBlock, ToolTip, TopLevel,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::RefCell;
use std::rc::Rc;

macro_rules! sample_page {
    ($name:ident, $source:literal) => {
        #[repr(C)]
        pub(crate) struct $name { base: FramePage }
        ferro_class!($name: FramePage);
        ferro_class_info!($name {});
        ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
            InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl,
            FramePageImpl);
        impl $name {
            fn create() -> Result<Ref<FramePage>, NavigationError> {
                let content = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!($source), None)
                    .map_err(|error| NavigationError::FactoryFailed(error.to_string()))?;
                let page = instantiate(Self { base: FramePage::construct() });
                page.set_content(Some(Control::boxed(content)));
                Ok(page.upcast())
            }
        }
    };
}

sample_page!(SamplePage1, "Pages/NavigationSample1.xaml");
sample_page!(SamplePage2, "Pages/NavigationSample2.xaml");
sample_page!(SamplePage3, "Pages/NavigationSample3.xaml");
sample_page!(SamplePage4, "Pages/NavigationSample4.xaml");
sample_page!(SamplePage5, "Pages/NavigationSample5.xaml");
sample_page!(SamplePage6, "Pages/NavigationSample6.xaml");
sample_page!(SampleSettingsPage, "Pages/NavigationSettings.xaml");

fn register_pages(frame: &Frame) -> Result<(), NavigationError> {
    for (type_, factory) in [
        (SamplePage1::TYPE, SamplePage1::create as fn() -> _),
        (SamplePage2::TYPE, SamplePage2::create as fn() -> _),
        (SamplePage3::TYPE, SamplePage3::create as fn() -> _),
        (SamplePage4::TYPE, SamplePage4::create as fn() -> _),
        (SamplePage5::TYPE, SamplePage5::create as fn() -> _),
        (SamplePage6::TYPE, SamplePage6::create as fn() -> _),
        (
            SampleSettingsPage::TYPE,
            SampleSettingsPage::create as fn() -> _,
        ),
    ] {
        frame.register_page_factory(type_, Rc::new(factory))?;
    }
    Ok(())
}

fn page_type(tag: &str) -> Option<&'static TypeInfo> {
    match tag {
        "SamplePage1" => Some(SamplePage1::TYPE),
        "SamplePage2" => Some(SamplePage2::TYPE),
        "SamplePage3" => Some(SamplePage3::TYPE),
        "SamplePage4" => Some(SamplePage4::TYPE),
        "SamplePage5" => Some(SamplePage5::TYPE),
        "SamplePage6" => Some(SamplePage6::TYPE),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CategoryEntry {
    Category {
        name: &'static str,
        glyph: Symbol,
        tooltip: &'static str,
    },
    Header(&'static str),
    Separator,
}

fn categories(nav: &NavigationView) {
    let items = Rc::new(FerroList::<BoxedValue>::new());
    for item in [
        CategoryEntry::Category {
            name: "Category 1",
            glyph: Symbol::Home,
            tooltip: "This is category 1",
        },
        CategoryEntry::Category {
            name: "Category 2",
            glyph: Symbol::Keyboard,
            tooltip: "This is category 2",
        },
        CategoryEntry::Separator,
        CategoryEntry::Header("More categories"),
        CategoryEntry::Category {
            name: "Category 3",
            glyph: Symbol::Library,
            tooltip: "This is category 3",
        },
        CategoryEntry::Category {
            name: "Category 4",
            glyph: Symbol::Mail,
            tooltip: "This is category 4",
        },
    ] {
        items.add(Rc::new(item));
    }
    nav.set_menu_item_template(Some(FuncDataTemplate::for_type::<CategoryEntry>(
        |entry, _| match entry {
            CategoryEntry::Category {
                name,
                glyph,
                tooltip,
            } => {
                let item = NavigationViewItem::new();
                item.set_content(Some(Rc::new(name.to_string())));
                item.set_icon(Some(SymbolIcon::with_symbol(*glyph).upcast()));
                ToolTip::set_tip(&item, Some(Rc::new(tooltip.to_string())));
                Some(item.upcast())
            }
            CategoryEntry::Header(name) => {
                let item = NavigationViewItemHeader::new();
                item.set_content(Some(Rc::new(name.to_string())));
                Some(item.upcast())
            }
            CategoryEntry::Separator => Some(NavigationViewItemSeparator::new().upcast()),
        },
        false,
    )));
    nav.set_menu_items_source(Some(items.into()));
}

/// Keeps the source's runtime selection behavior, including its specific header
/// updates, the Top sample's fourth item targeting SamplePage3, and every Category
/// displaying SamplePage1. The tabs snippet recommends disabling history, but the
/// actual page handler calls Navigate; this port retains that observable behavior.
fn wire_selection(
    root: &Ref<Control>,
    number: &str,
    update_header: bool,
) -> Result<(), NavigationError> {
    let nav = root.get_control::<NavigationView>(&format!("nvSample{number}"));
    let frame = root.get_control::<Frame>(&format!("contentFrame{number}"));
    register_pages(&frame)?;
    let weak_nav = nav.downgrade();
    let weak_frame = frame.downgrade();
    let weak_error = root
        .get_control::<TextBlock>("NavigationViewError")
        .downgrade();
    nav.selection_changed(move |args| {
        let (Some(nav), Some(frame), Some(error)) = (
            weak_nav.upgrade(),
            weak_frame.upgrade(),
            weak_error.upgrade(),
        ) else {
            return;
        };
        let target = if args.is_settings_selected() {
            Some(SampleSettingsPage::TYPE)
        } else {
            args.selected_item().as_ref().and_then(|value| {
                if let Some(CategoryEntry::Category { name, .. }) =
                    value.downcast_ref::<CategoryEntry>()
                {
                    nav.set_header(Some(Rc::new(format!(
                        "Sample Page {}",
                        name.chars().last().unwrap()
                    ))));
                    Some(SamplePage1::TYPE)
                } else {
                    let item = Control::from_boxed(value)?.cast::<NavigationViewItem>()?;
                    let tag = item.tag()?;
                    let tag = tag.downcast_ref::<String>()?;
                    let target = page_type(tag)?;
                    if update_header {
                        nav.set_header(Some(Rc::new(format!(
                            "Sample Page {}",
                            tag.chars().last().unwrap()
                        ))));
                    }
                    Some(target)
                }
            })
        };
        let Some(target) = target else { return };
        match frame.navigate(target, None) {
            Ok(_) => {
                error.set_text(None);
                error.set_is_visible(false);
            }
            Err(reason) => {
                error.set_text(Some(&reason.to_string()));
                error.set_is_visible(true);
            }
        }
    });
    Ok(())
}

fn wire_check(
    root: &Ref<Control>,
    name: &str,
    change: impl Fn(&NavigationView, &Control, bool) + 'static,
) {
    let checkbox = root.get_control::<CheckBox>(name);
    let weak_checkbox = checkbox.downgrade();
    let weak_root = root.downgrade();
    let weak_nav = root.get_control::<NavigationView>("nvSample").downgrade();
    checkbox.click(move |_, _| {
        if let (Some(checkbox), Some(root), Some(nav)) = (
            weak_checkbox.upgrade(),
            weak_root.upgrade(),
            weak_nav.upgrade(),
        ) {
            change(&nav, &root, checkbox.is_checked() == Some(true));
        }
    });
}

fn wire_options(root: &Ref<Control>) {
    wire_check(root, "settingsCheck", |nav, _, checked| {
        nav.set_is_settings_visible(checked)
    });
    wire_check(root, "visibleCheck", |nav, _, checked| {
        nav.set_is_back_button_visible(if checked {
            NavigationViewBackButtonVisible::Visible
        } else {
            NavigationViewBackButtonVisible::Collapsed
        })
    });
    wire_check(root, "enableCheck", |nav, _, checked| {
        nav.set_is_back_enabled(checked)
    });
    wire_check(root, "headerCheck", |nav, _, checked| {
        nav.set_always_show_header(checked)
    });
    wire_check(root, "autoSuggestCheck", |nav, _, checked| {
        nav.set_auto_suggest_box(if checked {
            let search = AutoSuggestBox::new();
            search.set_query_icon(Some(SymbolIcon::with_symbol(Symbol::Find).upcast()));
            AutomationProperties::set_name(&search, Some("search"));
            Some(search)
        } else {
            None
        });
    });
    wire_check(root, "panemc_Check", |_, root, checked| {
        root.get_control::<Control>("PaneHyperlink")
            .set_is_visible(checked);
    });
    wire_check(root, "paneFooterCheck", |_, root, checked| {
        root.get_control::<StackPanel>("FooterStackPanel")
            .set_is_visible(checked);
    });
    wire_check(root, "sffCheck", |nav, _, checked| {
        nav.set_selection_follows_focus(if checked {
            NavigationViewSelectionFollowsFocus::Enabled
        } else {
            NavigationViewSelectionFollowsFocus::Disabled
        })
    });
    wire_check(
        root,
        "suppressselectionCheck_Checked",
        |_, root, checked| {
            root.get_control::<NavigationViewItem>("SamplePage2Item")
                .set_selects_on_invoked(!checked);
        },
    );
    for (name, target, mode, open) in [
        (
            "nvSampleLeft",
            "nvSample",
            NavigationViewPaneDisplayMode::Left,
            true,
        ),
        (
            "nvSampleTop",
            "nvSample",
            NavigationViewPaneDisplayMode::Top,
            false,
        ),
        (
            "nvSample8Left",
            "nvSample8",
            NavigationViewPaneDisplayMode::Left,
            true,
        ),
        (
            "nvSample8Top",
            "nvSample8",
            NavigationViewPaneDisplayMode::Top,
            false,
        ),
        (
            "nvSample8LeftCompact",
            "nvSample8",
            NavigationViewPaneDisplayMode::LeftCompact,
            false,
        ),
        (
            "nvSample9Left",
            "nvSample9",
            NavigationViewPaneDisplayMode::Left,
            true,
        ),
        (
            "nvSample9Top",
            "nvSample9",
            NavigationViewPaneDisplayMode::Top,
            false,
        ),
    ] {
        let option = root.get_control::<RadioButton>(name);
        let weak_option = option.downgrade();
        let weak_nav = root.get_control::<NavigationView>(target).downgrade();
        let weak_footer = root
            .get_control::<StackPanel>("FooterStackPanel")
            .downgrade();
        option.property_changed(move |change| {
            if change.property() != ToggleButton::is_checked_property().as_property() {
                return;
            }
            if let (Some(option), Some(nav)) = (weak_option.upgrade(), weak_nav.upgrade()) {
                if option.is_checked() != Some(true) {
                    return;
                }
                nav.set_pane_display_mode(mode);
                nav.set_is_pane_open(open);
                if target == "nvSample" {
                    if let Some(footer) = weak_footer.upgrade() {
                        footer.set_orientation(if mode == NavigationViewPaneDisplayMode::Top {
                            Orientation::Horizontal
                        } else {
                            Orientation::Vertical
                        });
                    }
                }
            }
        });
    }
}

#[derive(Default)]
struct AdaptiveSubscription(RefCell<Option<Rc<dyn IDisposable>>>);
impl AdaptiveSubscription {
    fn clear(&self) {
        if let Some(subscription) = self.0.borrow_mut().take() {
            subscription.dispose();
        }
    }
}
impl Drop for AdaptiveSubscription {
    fn drop(&mut self) {
        self.clear();
    }
}

fn adaptive_mode(nav: &NavigationView, width: f64) {
    nav.set_pane_display_mode(if width >= nav.compact_mode_threshold_width() {
        NavigationViewPaneDisplayMode::Top
    } else {
        NavigationViewPaneDisplayMode::Auto
    });
}

fn wire_adaptive(root: &Ref<Control>) {
    let nav = root.get_control::<NavigationView>("nvSample2");
    let subscription = Rc::new(AdaptiveSubscription::default());
    let weak_root = root.downgrade();
    let weak_nav = nav.downgrade();
    let attached_subscription = subscription.clone();
    root.attached_to_visual_tree(move |_| {
        attached_subscription.clear();
        let (Some(root), Some(nav)) = (weak_root.upgrade(), weak_nav.upgrade()) else {
            return;
        };
        let Some(host) = TopLevel::get_top_level(Some(&root)) else {
            return;
        };
        adaptive_mode(&nav, host.client_size().width);
        let weak_host = host.downgrade();
        let weak_nav = nav.downgrade();
        *attached_subscription.0.borrow_mut() = Some(host.property_changed(move |change| {
            if change.property() != TopLevel::client_size_property().as_property() {
                return;
            }
            if let (Some(host), Some(nav)) = (weak_host.upgrade(), weak_nav.upgrade()) {
                adaptive_mode(&nav, host.client_size().width);
            }
        }));
    });
    root.detached_from_visual_tree(move |_| subscription.clear());
    let weak_root = root.downgrade();
    let weak_nav = nav.downgrade();
    root.property_changed(move |change| {
        if change.property() != Visual::bounds_property().as_property() {
            return;
        }
        if let (Some(root), Some(nav)) = (weak_root.upgrade(), weak_nav.upgrade()) {
            let width = TopLevel::get_top_level(Some(&root))
                .map_or(root.bounds().width, |host| host.client_size().width);
            adaptive_mode(&nav, width);
        }
    });
    let weak_root = root.downgrade();
    let weak_nav = nav.downgrade();
    nav.property_changed(move |change| {
        if change.property()
            != NavigationView::compact_mode_threshold_width_property().as_property()
        {
            return;
        }
        if let (Some(root), Some(nav)) = (weak_root.upgrade(), weak_nav.upgrade()) {
            let width = TopLevel::get_top_level(Some(&root))
                .map_or(root.bounds().width, |host| host.client_size().width);
            adaptive_mode(&nav, width);
        }
    });
}

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadException> {
    if root
        .find_control::<Control>("nvSample5")
        .and_then(|x| x.cast::<NavigationView>())
        .is_none()
    {
        return Ok(());
    }
    let convert_error = |error| {
        XamlLoadException::with_inner("Could not initialize NavigationView samples.", error)
    };
    categories(&root.get_control::<NavigationView>("nvSample4"));
    for (number, header) in [
        ("", true),
        ("2", false),
        ("4", true),
        ("5", true),
        ("6", false),
        ("7", false),
        ("8", true),
        ("9", false),
    ] {
        wire_selection(root, number, header).map_err(convert_error)?;
    }
    wire_options(root);
    wire_adaptive(root);
    for number in ["2", "4", "5", "6", "7", "8", "9"] {
        let nav = root.get_control::<NavigationView>(&format!("nvSample{number}"));
        let first = nav
            .menu_items_source()
            .map_or_else(|| nav.menu_items().get_at(0), |source| source.get_at(0));
        nav.set_selected_item(first);
    }
    Ok(())
}

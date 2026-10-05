//! Port of `ViewModels/MainWindowViewModel_PageList.cs`: the sections of
//! the catalog and their pages.
//!
//! The managed original names the class of each page
//! (`s.Add<ButtonsPage>(..)`); here an entry names the document of the
//! class, and the page is created through the table of the classes with a
//! document ([`XamlClass`]).
//!
//! TEMPORARY: an entry whose class is not declared yet, or whose document
//! is listed in `excluded.txt`, is left out of its section and recorded as
//! an [`UnavailablePage`] with what it waits for, so that the application
//! only offers pages that load. With an empty `excluded.txt` the sections
//! are the upstream ones.
//!
//! A page without a document (`ScreenPage`) is added with its constructor.
//!
//! Not ported: the unused method `HomeSectionBuilder.Navigate`.

use crate::controls::SampleInfo;
use crate::icons::Icons;
use crate::markup::XamlClass;
use crate::models::{HomeSection, PageItem};
use ferroui_base::media::StreamGeometry;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::Page;
use std::rc::Rc;

/// A page of the upstream list that is left out of its section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnavailablePage {
    /// The title of the section of the page.
    pub section: String,
    /// The header of the page.
    pub header: &'static str,
    /// The rooted asset path of the document of the page.
    pub document: &'static str,
    /// What the page waits for: the reason of `excluded.txt`, or that its
    /// class is not ported.
    pub reason: String,
}

/// The sections of the catalog, and the pages that are left out of them.
pub(super) fn page_sections() -> (Vec<Rc<HomeSection>>, Vec<UnavailablePage>) {
    let mut unavailable = Vec::new();
    let sections = vec![
        section("Basic Input", Icons::CURSOR2, &mut unavailable, |s| {
            s.add("/Pages/ButtonsPage.xaml", "Buttons", Icons::CURSOR_CLICK, "Button, RepeatButton, ToggleButton and friends");
            s.add("/Pages/ButtonSpinnerPage.xaml", "ButtonSpinner", Icons::SPINNER, "Content with increment and decrement buttons");
            s.add("/Pages/CheckBoxPage.xaml", "CheckBox", Icons::CHECKBOX, "Two- and three-state check boxes");
            s.add("/Pages/ColorPickerPage.xaml", "ColorPicker", Icons::PALETTE, "Pick colors from spectrum and palette views");
            s.add("/Pages/ComboBoxPage.xaml", "ComboBox", Icons::DROPDOWN, "A drop-down list of selectable items");
            s.add("/Pages/NumericUpDownPage.xaml", "NumericUpDown", Icons::NUMBER, "Numeric input with spinner buttons");
            s.add("/Pages/RadioButtonPage.xaml", "RadioButton", Icons::RADIO, "Mutually exclusive option groups");
            s.add("/Pages/SliderPage.xaml", "Slider", Icons::TUNE, "Select a value from a continuous range");
            s.add("/Pages/ToggleSwitchPage.xaml", "ToggleSwitch", Icons::TOGGLE, "An on/off switch with a sliding knob");
        }),
        section("Text", Icons::TEXT_BOX, &mut unavailable, |s| {
            s.add("/Pages/AutoCompleteBoxPage.xaml", "AutoCompleteBox", Icons::TEXT_INPUT, "Text input with completion suggestions");
            s.add("/Pages/LabelsPage.xaml", "Label", Icons::TAG, "Captions with access keys for other controls");
            s.add_with_samples(
                "/Pages/TextBoxPage.xaml",
                "TextBox",
                Icons::TEXT_INPUT,
                "Single- and multi-line text editing",
                Rc::new(crate::pages::TextBoxPage::demos().snapshot().to_vec()),
            );
            s.add("/Pages/TextBlockPage.xaml", "TextBlock", Icons::TEXT_INPUT, "Styled read-only text display");
        }),
        section("Collections & Data", Icons::LISTS, &mut unavailable, |s| {
            s.add("/Pages/CarouselPage.xaml", "Carousel", Icons::SLIDES, "Cycle through a collection of items");
            s.add("/Pages/ListBoxPage.xaml", "ListBox", Icons::LIST, "A selectable, virtualized list of items");
            s.add("/Pages/PipsPagerPage.xaml", "PipsPager", Icons::HORIZONTAL_DOTS, "Dot-style pager for paginated content");
            s.add("/Pages/RefreshContainerPage.xaml", "RefreshContainer", Icons::REFRESH, "Pull-to-refresh for scrollable content");
            s.add("/Pages/TableViewPage.xaml", "TableView", Icons::GRID, "Tabular data with resizable, sortable columns");
            s.add("/Pages/TreeViewPage.xaml", "TreeView", Icons::TREE, "Hierarchical data with expandable nodes");
        }),
        section("Date & Time", Icons::DATE, &mut unavailable, |s| {
            s.add("/Pages/CalendarPage.xaml", "Calendar", Icons::CALENDAR, "A month calendar for selecting dates");
            s.add("/Pages/CalendarDatePickerPage.xaml", "CalendarDatePicker", Icons::CALENDAR, "A date picker with a drop-down calendar");
            s.add("/Pages/DateTimePickerPage.xaml", "Date/Time Picker", Icons::CLOCK, "Spinner-style date and time pickers");
        }),
        section("Menus & Flyouts", Icons::MENUS, &mut unavailable, |s| {
            s.add("/Pages/CommandBarPage.xaml", "CommandBar", Icons::TERMINAL, "A toolbar of commands with an overflow menu");
            s.add("/Pages/ContextFlyoutPage.xaml", "ContextFlyout", Icons::MENU, "Attach flyouts shown on right-click");
            s.add("/Pages/ContextMenuPage.xaml", "ContextMenu", Icons::MENU, "Traditional right-click context menus");
            s.add("/Pages/FlyoutsPage.xaml", "Flyouts", Icons::FLYOUT, "Lightweight popups anchored to controls");
            s.add("/Pages/MenuPage.xaml", "Menu", Icons::MENU, "Menu bars with nested menu items");
        }),
        section("Navigation & Pages", Icons::HAMBURGER_UNREAD, &mut unavailable, |s| {
            s.add("/Pages/CarouselDemoPage.xaml", "CarouselPage", Icons::SLIDES, "Swipeable page-based navigation");
            s.add("/Pages/ContentDemoPage.xaml", "ContentPage", Icons::DOCUMENT, "A page that hosts a single content view");
            s.add("/Pages/DrawerDemoPage.xaml", "DrawerPage", Icons::DRAWER, "A page with a sliding navigation drawer");
            s.add("/Pages/NavigationDemoPage.xaml", "NavigationPage", Icons::NAVIGATION, "Stack-based page navigation");
            s.add("/Pages/SplitViewPage.xaml", "SplitView", Icons::SPLIT, "A collapsible pane beside content");
            s.add("/Pages/TabbedDemoPage.xaml", "TabbedPage", Icons::TAB, "Tab-based page navigation");
            s.add("/Pages/TabControlPage.xaml", "TabControl", Icons::TAB, "Switch between tabbed content views");
            s.add("/Pages/TabStripPage.xaml", "TabStrip", Icons::TAB, "A standalone strip of selectable tabs");
        }),
        section("Layout", Icons::LAYOUTS, &mut unavailable, |s| {
            s.add("/Pages/BorderPage.xaml", "Border", Icons::BORDER, "Decorate elements with borders and corner radii");
            s.add("/Pages/CanvasPage.xaml", "Canvas", Icons::CANVAS, "Position children at explicit coordinates");
            s.add("/Pages/ContainerQueryPage.xaml", "Container Queries", Icons::CONTAINER, "Styles that respond to container size");
            s.add("/Pages/ExpanderPage.xaml", "Expander", Icons::EXPAND, "A header that expands to reveal content");
            s.add("/Pages/FlexPage.xaml", "Flex Panel", Icons::GRID, "Flexible, CSS-style child layout");
            s.add("/Pages/HeaderedContentPage.xaml", "HeaderedContentControl", Icons::HEADER, "Content paired with a header");
            s.add("/Pages/LayoutTransformControlPage.xaml", "LayoutTransformControl", Icons::TRANSFORM, "Apply transforms that affect layout");
            s.add("/Pages/RelativePanelPage.xaml", "RelativePanel", Icons::LAYOUT, "Arrange children relative to each other");
            s.add("/Pages/ScrollViewerPage.xaml", "ScrollViewer", Icons::SCROLL, "Scrollable viewport over large content");
            s.add("/Pages/ViewboxPage.xaml", "Viewbox", Icons::VIEWBOX, "Scale content to fit available space");
            s.add("/Pages/WrapPanelPage.xaml", "WrapPanel", Icons::LAYOUT, "Wrap children onto multiple lines");
        }),
        section("Media & Graphics", Icons::MEDIA, &mut unavailable, |s| {
            s.add("/Pages/AcrylicPage.xaml", "Acrylic", Icons::BLUR, "Translucent acrylic window materials");
            s.add("/Pages/BitmapCachePage.xaml", "BitmapCache", Icons::LIGHTNING, "Cache visuals as bitmaps for performance");
            s.add("/Pages/CompositionPage.xaml", "Composition", Icons::LAYERS, "Composition-layer animations and effects");
            s.add("/Pages/CustomDrawing.xaml", "Custom Drawing", Icons::BRUSH, "Render custom geometry in code");
            s.add("/Pages/ImagePage.xaml", "Image", Icons::IMAGE, "Display bitmaps with different stretch modes");
            s.add("/Pages/OpenGlPage.xaml", "OpenGL", Icons::CUBE_3D, "Embed custom OpenGL rendering");
            s.add("/Pages/OpenGl/OpenGlLeasePage.xaml", "OpenGL Lease", Icons::CUBE_3D, "Low-level access to the OpenGL context");
            s.add("/Pages/OpenGl/OpenGlInteropPage.xaml", "OpenGL Interop", Icons::CUBE_3D, "Compositor and OpenGL interop");
            s.add("/Pages/TransitioningContentControlPage.xaml", "TransitioningContentControl", Icons::TRANSITION, "Animate between content changes");
        }),
        section("Status & Feedback", Icons::CHAT, &mut unavailable, |s| {
            s.add("/Pages/AdornerLayerPage.xaml", "AdornerLayer", Icons::SPARKLE, "Overlay visuals on top of other controls");
            s.add("/Pages/DataValidationPage.xaml", "Data Validation", Icons::SHIELD, "Display validation errors from bindings");
            s.add("/Pages/DialogsPage.xaml", "Dialogs", Icons::DIALOG, "File pickers and modal dialog windows");
            s.add("/Pages/NotificationsPage.xaml", "Notifications", Icons::BELL, "Toast-style in-app notifications");
            s.add("/Pages/ProgressBarPage.xaml", "ProgressBar", Icons::PROGRESS, "Determinate and indeterminate progress");
            s.add("/Pages/ToolTipPage.xaml", "ToolTip", Icons::TOOLTIP, "Hover hints for any control");
        }),
        section("Interaction", Icons::KEYBOARD_FLOAT, &mut unavailable, |s| {
            s.add("/Pages/AcceleratorPage.xaml", "Accelerator", Icons::KEYBOARD, "Keyboard shortcuts that invoke commands");
            s.add("/Pages/ClipboardPage.xaml", "Clipboard", Icons::CLIPBOARD, "Read from and write to the system clipboard");
            s.add("/Pages/CursorPage.xaml", "Cursor", Icons::CURSOR, "Change the pointer cursor over elements");
            s.add("/Pages/DragAndDropPage.xaml", "Drag+Drop", Icons::DRAG_DROP, "Drag data within and between applications");
            s.add("/Pages/FocusPage.xaml", "Focus", Icons::TARGET, "Track and control keyboard focus");
            s.add("/Pages/GesturePage.xaml", "Gestures", Icons::GESTURE, "Tap, scroll and pinch gesture recognition");
            s.add("/Pages/PointersPage.xaml", "Pointers", Icons::CURSOR, "Raw pointer input and capture");
        }),
        section("Window & Platform", Icons::DESKTOP_MOBILE, &mut unavailable, |s| {
            s.add("/Pages/NativeEmbedPage.xaml", "Native Embed", Icons::PUZZLE, "Host native platform controls");
            s.add("/Pages/PlatformInfoPage.xaml", "Platform Information", Icons::INFO, "Runtime platform and capability info");
            s.add("/Pages/PlatformSettingsPage.xaml", "Platform Settings", Icons::TUNE, "Platform-specific system settings");
            s.add_page(|| crate::pages::ScreenPage::new().upcast(), "Screens", Icons::MONITOR, "Enumerate displays and their bounds");
            s.add("/Pages/ThemePage.xaml", "Theme Variants", Icons::THEME, "Switch between light and dark variants");
            s.add("/Pages/WindowCustomizationsPage.xaml", "Window Customizations", Icons::WINDOW, "Custom chrome, decorations and sizing");
        }),
    ];
    (sections, unavailable)
}

fn section(
    title: &str,
    icon_path: &str,
    unavailable: &mut Vec<UnavailablePage>,
    builder_callback: impl FnOnce(&mut HomeSectionBuilder),
) -> Rc<HomeSection> {
    let icon_geometry = parse_icon(icon_path);
    let section = HomeSection::new(title, icon_geometry);
    let mut builder = HomeSectionBuilder { section: section.clone(), items: Vec::new(), unavailable };
    builder_callback(&mut builder);
    let items = builder.items;
    section.set_items(Some(Rc::new(items)));
    section
}

fn parse_icon(icon_path: &str) -> Ref<StreamGeometry> {
    StreamGeometry::parse(icon_path).unwrap_or_else(|e| panic!("invalid icon path data: {e}"))
}

struct HomeSectionBuilder<'a> {
    section: Rc<HomeSection>,
    items: Vec<Rc<PageItem>>,
    unavailable: &'a mut Vec<UnavailablePage>,
}

impl HomeSectionBuilder<'_> {
    /// `Add<TPageType>(header, iconPath, description)`, where `TPageType` is
    /// the class of `document`.
    fn add(&mut self, document: &'static str, header: &'static str, icon_path: &str, description: &str) {
        self.add_entry(document, header, icon_path, description, None);
    }

    /// `Add<TPageType>(header, iconPath, description, samples)`.
    fn add_with_samples(
        &mut self,
        document: &'static str,
        header: &'static str,
        icon_path: &str,
        description: &str,
        samples: Rc<Vec<Rc<SampleInfo>>>,
    ) {
        self.add_entry(document, header, icon_path, description, Some(samples));
    }

    fn add_entry(
        &mut self,
        document: &'static str,
        header: &'static str,
        icon_path: &str,
        description: &str,
        samples: Option<Rc<Vec<Rc<SampleInfo>>>>,
    ) {
        let reason = match (XamlClass::find(document), crate::excluded(document)) {
            (_, Some(excluded)) => Some(excluded.reason.to_string()),
            (None, None) => Some("the class of the document is not ported".to_string()),
            (Some(_), None) => None,
        };
        if let Some(reason) = reason {
            self.unavailable.push(UnavailablePage { section: self.section.title(), header, document, reason });
            return;
        }

        let icon_geometry = parse_icon(icon_path);
        self.items.push(PageItem::new(
            header,
            move || create_page(document),
            icon_geometry,
            description,
            Some(&self.section),
            samples,
        ));
    }
}

impl HomeSectionBuilder<'_> {
    /// `Add<TPageType>(header, iconPath, description)` for a class without a
    /// document, created by `factory`.
    fn add_page(
        &mut self,
        factory: impl Fn() -> Ref<Page> + 'static,
        header: &'static str,
        icon_path: &str,
        description: &str,
    ) {
        let icon_geometry = parse_icon(icon_path);
        self.items.push(PageItem::new(header, factory, icon_geometry, description, Some(&self.section), None));
    }
}

/// `new TPageType()` for the class of `document`.
fn create_page(document: &str) -> Ref<Page> {
    let class = XamlClass::find(document).unwrap_or_else(|| panic!("the class of {document} is not declared"));
    from_markup_value::<Ref<Page>>(&Some((class.create)()))
        .unwrap_or_else(|| panic!("the class of {document} is not a page"))
}

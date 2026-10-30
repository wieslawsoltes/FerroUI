//! Native gallery composition, search, navigation, source inspection and diagnostics.

use crate::catalog::{self, CatalogEntry, SampleDefinition, CATALOG};
use crate::{examples, navigation::NavigationHistory};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    Application, Button, ComboBox, ContentControl, Control, Expander, HyperlinkButton, ListBox,
    ScrollViewer, StackPanel, TextBlock, TextBox,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_winui::{migrate_winui_xaml, WinUIXamlOptions};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

/// An owned gallery session. Keeping this value alive retains its event state;
/// controls hold weak references back to that state, so closing releases the tree.
pub struct GalleryView {
    state: Rc<GalleryState>,
}

struct GalleryState {
    root: Ref<Control>,
    search: Ref<TextBox>,
    category: Ref<ComboBox>,
    pages: Ref<ListBox>,
    body: Ref<StackPanel>,
    page_scroll: Ref<ScrollViewer>,
    results: Ref<TextBlock>,
    back: Ref<Button>,
    forward: Ref<Button>,
    categories: Vec<&'static str>,
    filtered: RefCell<Vec<&'static CatalogEntry>>,
    history: RefCell<NavigationHistory>,
    updating: Cell<bool>,
    native_example_error: RefCell<Option<String>>,
    native_example: RefCell<Option<Ref<Control>>>,
    preview_options: WinUIXamlOptions,
}

impl GalleryView {
    pub fn new() -> Result<Self, XamlLoadException> {
        Self::with_preview_options(WinUIXamlOptions::default())
    }

    /// Hosts choose the preview policy explicitly. For a host that installs the
    /// WinUI theme, use `WinUIXamlOptions::for_winui_theme()`.
    pub fn with_preview_options(
        preview_options: WinUIXamlOptions,
    ) -> Result<Self, XamlLoadException> {
        ferroui_markup_xaml::register_types();
        FerroRuntimeXamlLoader::register();
        let root =
            FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!("Gallery.xaml"), None)?;
        let categories: Vec<_> = CATALOG
            .iter()
            .map(|page| page.category)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let category = root.get_control::<ComboBox>("Category");
        category.items().add(box_text("All categories"));
        for name in &categories {
            category.items().add(box_text(name));
        }
        category.set_selected_index(0);
        let theme = root.get_control::<ComboBox>("Theme");
        theme.set_selected_index(0);
        let state = Rc::new(GalleryState {
            search: root.get_control("Search"),
            category,
            pages: root.get_control("Pages"),
            body: root.get_control("PageBody"),
            page_scroll: root.get_control("PageScroll"),
            results: root.get_control("ResultCount"),
            back: root.get_control("Back"),
            forward: root.get_control("Forward"),
            root,
            categories,
            filtered: RefCell::new(Vec::new()),
            history: RefCell::new(NavigationHistory::default()),
            updating: Cell::new(false),
            native_example_error: RefCell::new(None),
            native_example: RefCell::new(None),
            preview_options,
        });
        let weak = Rc::downgrade(&state);
        state.search.property_changed(move |change| {
            if change.property() == TextBox::text_property().as_property() {
                if let Some(state) = weak.upgrade() {
                    state.filter();
                }
            }
        });
        let weak = Rc::downgrade(&state);
        state.category.property_changed(move |change| {
            if change.property() == SelectingItemsControl::selected_index_property().as_property() {
                if let Some(state) = weak.upgrade() {
                    state.filter();
                }
            }
        });
        let weak = Rc::downgrade(&state);
        state.pages.property_changed(move |change| {
            if change.property() == SelectingItemsControl::selected_index_property().as_property() {
                if let Some(state) = weak.upgrade() {
                    if state.updating.get() {
                        return;
                    }
                    let index = usize::try_from(state.pages.selected_index()).ok();
                    let page = index.and_then(|index| state.filtered.borrow().get(index).copied());
                    if let Some(page) = page {
                        state.navigate(page.id);
                    }
                }
            }
        });
        let weak = Rc::downgrade(&state);
        state.back.click(move |_, _| {
            if let Some(state) = weak.upgrade() {
                let changed = state.history.borrow_mut().go_back();
                if changed {
                    state.show_current();
                }
            }
        });
        let weak = Rc::downgrade(&state);
        state.forward.click(move |_, _| {
            if let Some(state) = weak.upgrade() {
                let changed = state.history.borrow_mut().go_forward();
                if changed {
                    state.show_current();
                }
            }
        });
        let weak_theme = theme.downgrade();
        theme.property_changed(move |change| {
            if change.property() == SelectingItemsControl::selected_index_property().as_property() {
                if let (Some(theme), Some(app)) = (weak_theme.upgrade(), Application::current()) {
                    let variant = match theme.selected_index() {
                        1 => Some(ThemeVariant::light()),
                        2 => Some(ThemeVariant::dark()),
                        _ => None,
                    };
                    app.set_requested_theme_variant(variant);
                }
            }
        });
        state.filter();
        state.navigate("Button");
        Ok(Self { state })
    }

    pub fn root(&self) -> Ref<Control> {
        self.state.root.clone()
    }

    /// The selected page's native construction failure, if any. Source-only
    /// pages have no construction result and do not count as native coverage.
    pub fn native_example_error(&self) -> Option<String> {
        self.state.native_example_error.borrow().clone()
    }

    /// The selected page's actual native sample root, absent for source-only
    /// pages or a failed adaptation. Useful to drive bounded host integration.
    pub fn native_example(&self) -> Option<Ref<Control>> {
        self.state.native_example.borrow().clone()
    }
    pub fn current_page(&self) -> Option<&'static str> {
        self.state.history.borrow().current()
    }
    pub fn navigate(&self, id: &str) -> bool {
        self.state.navigate(id)
    }
    pub fn set_search(&self, query: &str) {
        self.state.search.set_text(Some(query));
    }
    pub fn visible_page_count(&self) -> usize {
        self.state.filtered.borrow().len()
    }

    pub fn go_back(&self) -> bool {
        let changed = self.state.history.borrow_mut().go_back();
        if changed {
            self.state.show_current();
        }
        changed
    }

    pub fn go_forward(&self) -> bool {
        let changed = self.state.history.borrow_mut().go_forward();
        if changed {
            self.state.show_current();
        }
        changed
    }
}

impl GalleryState {
    fn filter(&self) {
        if self.updating.replace(true) {
            return;
        }
        let category_index = self
            .category
            .selected_index()
            .checked_sub(1)
            .and_then(|index| usize::try_from(index).ok());
        let category = category_index.and_then(|index| self.categories.get(index).copied());
        let query = self.search.text().unwrap_or_default();
        let filtered = catalog::search(&query, category);
        self.pages.items().clear();
        for entry in &filtered {
            self.pages
                .items()
                .add(box_text(&format!("{}  ·  {}", entry.title, entry.category)));
        }
        self.results.set_text(Some(&format!(
            "{} of {} pages",
            filtered.len(),
            CATALOG.len()
        )));
        *self.filtered.borrow_mut() = filtered;
        self.updating.set(false);
        self.sync_selection();
    }

    fn navigate(&self, id: &str) -> bool {
        let changed = self.history.borrow_mut().navigate(id);
        if changed {
            self.show_current();
        }
        changed
    }

    fn sync_selection(&self) {
        let current = self.history.borrow().current();
        let index = self
            .filtered
            .borrow()
            .iter()
            .position(|entry| Some(entry.id) == current);
        self.updating.set(true);
        self.pages
            .set_selected_index(index.map_or(-1, |index| index as i32));
        self.updating.set(false);
    }

    fn show_current(&self) {
        let history = self.history.borrow().clone();
        self.back.set_is_enabled(history.can_go_back());
        self.forward.set_is_enabled(history.can_go_forward());
        self.sync_selection();
        let Some(entry) = history.current().and_then(catalog::find) else {
            return;
        };
        self.body.children().clear();
        *self.native_example_error.borrow_mut() = None;
        *self.native_example.borrow_mut() = None;
        let title = text(entry.title);
        title.set_font_size(32.0);
        title.set_font_weight(FontWeight::SemiBold);
        self.body.children().add(title);
        self.body.children().add(text(entry.description));
        let links = StackPanel::new();
        links.set_spacing(4.0);
        links
            .children()
            .add(link("Pinned Gallery source", &entry.source_url()));
        for doc in entry.docs {
            links.children().add(link(doc.title, doc.uri));
        }
        self.body.children().add(links);

        if let Some(adaptation) = examples::find(entry.id) {
            self.body.children().add(text("Port state: partial. The native examples below cover a reviewed subset of this page."));
            let heading = text("Native FerroUI examples");
            heading.set_font_size(20.0);
            heading.set_font_weight(FontWeight::SemiBold);
            self.body.children().add(heading);
            match examples::create(adaptation) {
                Ok(control) => {
                    *self.native_example.borrow_mut() = Some(control.clone());
                    self.body.children().add(control);
                }
                Err(error) => {
                    *self.native_example_error.borrow_mut() = Some(error.to_string());
                    self.body
                        .children()
                        .add(text(&format!("Native adaptation failed to load: {error}")));
                }
            }
            self.body.children().add(text(adaptation.notes));
            self.body
                .children()
                .add(source_expander("FerroUI XAML", adaptation.xaml));
        } else if entry.id == "ScratchPad" {
            self.body.children().add(text("Port state: partial. The editor runs the explicit WinUI XAML migration policy; compiled bindings, platform features and code-behind remain separate requirements."));
        } else {
            self.body.children().add(text("This page's source is available below. Its full native behavior is pending; source coverage does not establish feature parity."));
        }

        if entry.id == "ScratchPad" {
            self.body.children().add(scratch_pad(&self.preview_options));
        }
        for sample in entry.samples {
            self.body
                .children()
                .add(sample_source(sample, &self.preview_options));
        }
        if entry.samples.is_empty() {
            self.body.children().add(text("This guidance page has no standalone snippets in the pinned upstream catalog. Use its documentation and source links above."));
        }
        // Every actual navigation builds a fresh page. Start that page at its
        // heading instead of retaining an unrelated page's outer scroll offset.
        // Sample-owned scroll viewers keep their own navigation/state behavior.
        self.page_scroll.scroll_to_home();
    }
}

fn box_text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}
fn boxed_control(value: Ref<Control>) -> Option<BoxedValue> {
    Some(Rc::new(value))
}

fn text(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}

fn link(title: &str, url: &str) -> Ref<HyperlinkButton> {
    let link = HyperlinkButton::new();
    link.set_content(box_text(title));
    link.set_horizontal_alignment(HorizontalAlignment::Left);
    if let Ok(uri) = Uri::new(url, UriKind::Absolute) {
        link.set_navigate_uri(Some(uri));
    }
    link
}

fn source_expander(header: &str, source: &str) -> Ref<Expander> {
    let expander = Expander::new();
    expander.set_header(box_text(header));
    let code = TextBox::new();
    code.set_text(Some(source));
    code.set_is_read_only(true);
    code.set_accepts_return(true);
    code.set_text_wrapping(TextWrapping::Wrap);
    code.set_max_height(500.0);
    code.set_min_height(80.0);
    AutomationProperties::set_name(&code, Some(header));
    expander.set_content(boxed_control(code.upcast()));
    expander
}

fn sample_source(sample: &'static SampleDefinition, options: &WinUIXamlOptions) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_margin(Thickness::new(0.0, 12.0, 0.0, 0.0));
    let title = text(sample.header);
    title.set_font_weight(FontWeight::SemiBold);
    panel.children().add(title);
    panel
        .children()
        .add(link("Original sample", &catalog::source_url(sample.source)));
    if !sample.xaml.is_empty() {
        panel
            .children()
            .add(source_expander("Original WinUI XAML", sample.xaml));
    }
    if !sample.code.is_empty() {
        panel.children().add(source_expander(
            "Original code-behind — not executed",
            sample.code,
        ));
    }
    if !sample.xaml.is_empty() {
        let button = Button::new();
        button.set_content(box_text("Load XAML preview"));
        let status =
            text("The preview validates XAML migration. Original code-behind is not executed.");
        let host = ContentControl::new();
        let weak_host = host.downgrade();
        let weak_status = status.downgrade();
        let options = options.clone();
        button.click(move |_, _| {
            if let (Some(host), Some(status)) = (weak_host.upgrade(), weak_status.upgrade()) {
                let wrapped = wrap_winui_sample(sample);
                match load_winui_preview_with_options(&wrapped, &options) {
                    Ok(control) => {
                        host.set_content(boxed_control(control));
                        status.set_text(Some(&preview_status(&wrapped, &options)));
                    }
                    Err(error) => {
                        host.set_content(None);
                        status.set_text(Some(&format!("Migration diagnostic: {error}")));
                    }
                }
            }
        });
        panel.children().add(button);
        panel.children().add(status);
        panel.children().add(host);
    }
    panel
}

/// Wraps a catalog fragment in its actual WinUI namespace. Prefix declarations
/// from the catalog are retained; no event handlers, bindings or tokens are erased.
pub fn wrap_winui_sample(sample: &SampleDefinition) -> String {
    format!(
        "<StackPanel xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\" {}>\n{}\n</StackPanel>",
        sample.namespace_imports.join(" "), sample.xaml,
    )
}

/// Exercises the public opt-in migration path, shared with compiled XAML.
pub fn load_winui_preview(xaml: &str) -> Result<Ref<Control>, XamlLoadException> {
    load_winui_preview_with_options(xaml, &WinUIXamlOptions::default())
}

pub fn load_winui_preview_with_options(
    xaml: &str,
    options: &WinUIXamlOptions,
) -> Result<Ref<Control>, XamlLoadException> {
    ferroui_markup_xaml_winui::parse_winui_as(xaml, None, options)
}

fn preview_status(xaml: &str, options: &WinUIXamlOptions) -> String {
    let mut result =
        "XAML loaded. Event, binding, rendering and platform parity require separate verification."
            .to_string();
    if let Ok(migration) = migrate_winui_xaml(xaml, options) {
        for diagnostic in migration.diagnostics {
            result.push_str(&format!(
                "\n{} at {}:{}: {}",
                diagnostic.code, diagnostic.span.line, diagnostic.span.column, diagnostic.message
            ));
        }
    }
    result
}

fn scratch_pad(options: &WinUIXamlOptions) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    let editor = TextBox::new();
    editor.set_accepts_return(true);
    editor.set_height(180.0);
    editor.set_text(Some("<StackPanel xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" Spacing=\"12\"><TextBlock Text=\"WinUI markup on FerroUI\"/><Button Content=\"Try me\"/></StackPanel>"));
    AutomationProperties::set_name(&editor, Some("WinUI XAML editor"));
    let button = Button::new();
    button.set_content(box_text("Apply XAML"));
    let output = ContentControl::new();
    let status = text("Enter WinUI XAML to inspect its compatibility diagnostics.");
    let weak_editor = editor.downgrade();
    let weak_output = output.downgrade();
    let weak_status = status.downgrade();
    let options = options.clone();
    button.click(move |_, _| {
        if let (Some(editor), Some(output), Some(status)) = (
            weak_editor.upgrade(),
            weak_output.upgrade(),
            weak_status.upgrade(),
        ) {
            let xaml = editor.text().unwrap_or_default();
            match load_winui_preview_with_options(&xaml, &options) {
                Ok(root) => {
                    output.set_content(boxed_control(root));
                    status.set_text(Some(&preview_status(&xaml, &options)));
                }
                Err(error) => {
                    output.set_content(None);
                    status.set_text(Some(&error.to_string()));
                }
            }
        }
    });
    panel.children().add(editor);
    panel.children().add(button);
    panel.children().add(status);
    panel.children().add(output);
    panel
}

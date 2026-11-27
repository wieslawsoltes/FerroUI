//! The code behind of the pinned Iconography page in Rust: `IconographyPage.xaml.cs` with the
//! icons of `IconsDataSource` and `IconData`.
//!
//! The icons are the 1574 entries of the source's `IconsData.json`, generated into
//! `icons_data.rs` by `generate_catalog.py`; only this sample links them. As in the source, the
//! grid of icons is an `ItemsView` with a `UniformGridLayout`, the search box filters the icons
//! by every word of its text over their code, name and tags, the side panel shows the selected
//! icon with its name, glyphs and the code of a `FontIcon` and a `SymbolIcon`, and a tag of the
//! panel puts its text into the search box.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, WeakRef};
use ferroui_controls::{AutoSuggestBox, Border, Control, ItemsSource, Symbol, TextBlock};
use ferroui_controls_items_view::ItemsView;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "Iconography" {
        register();
        IconographyPage::wire(root);
    }
}

/// The markup type of the page, for the `{Binding}` paths of its templates and side panel.
fn register() {
    MarkupType::register_all(&[<IconData as MarkupTyped>::MARKUP]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<IconData>();
}

/// An entry of the source's `IconsData.json` (the table of `icons_data.rs`).
pub(crate) struct IconRecord {
    pub(crate) code: &'static str,
    pub(crate) name: &'static str,
    pub(crate) tags: &'static [&'static str],
    pub(crate) is_segoe_fluent_only: bool,
}

/// `IconData`: an icon of the symbol font, with the texts the page shows of it.
pub(crate) struct IconData {
    pub(crate) record: &'static IconRecord,
}

impl PartialEq for IconData {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.record, other.record)
    }
}

impl IconData {
    pub(crate) fn name(&self) -> &'static str {
        self.record.name
    }

    pub(crate) fn code(&self) -> &'static str {
        self.record.code
    }

    pub(crate) fn tags(&self) -> &'static [&'static str] {
        self.record.tags
    }

    /// `Character`: the glyph of the code point.
    pub(crate) fn character(&self) -> String {
        u32::from_str_radix(self.record.code, 16)
            .ok()
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_default()
    }

    /// `CodeGlyph`: the code point as a C# escape.
    pub(crate) fn code_glyph(&self) -> String {
        format!("\\u{}", self.record.code)
    }

    /// `TextGlyph`: the code point as a XAML character reference.
    pub(crate) fn text_glyph(&self) -> String {
        format!("&#x{};", self.record.code)
    }

    /// `SymbolName`: the member of `Symbol` with the icon's name, if there is one (the source's
    /// `Enum.TryParse<Symbol>`).
    pub(crate) fn symbol_name(&self) -> Option<String> {
        Symbol::ALL
            .iter()
            .map(|symbol| format!("{symbol:?}"))
            .find(|symbol| symbol == self.record.name)
    }

    /// Whether every word of a search is in the code, the name or a tag of the icon, ignoring case
    /// (the source's `Filter`; an empty word matches every icon).
    pub(crate) fn fits_filter(&self, words: &[String]) -> bool {
        let code = self.record.code.to_lowercase();
        let name = self.record.name.to_lowercase();
        words.iter().all(|word| {
            code.contains(word.as_str())
                || name.contains(word.as_str())
                || self
                    .record
                    .tags
                    .iter()
                    .any(|tag| !tag.is_empty() && tag.to_lowercase().contains(word.as_str()))
        })
    }
}

ferro_markup_type!(class IconData {
    this: Rc<IconData>,
    handles: [IconData, Rc<IconData>, Option<Rc<IconData>>],
    properties: [
        Name: String { get: |this: &Rc<IconData>| this.name().to_string() },
        Code: String { get: |this: &Rc<IconData>| this.code().to_string() },
        Character: String { get: |this: &Rc<IconData>| this.character() },
        CodeGlyph: String { get: |this: &Rc<IconData>| this.code_glyph() },
        TextGlyph: String { get: |this: &Rc<IconData>| this.text_glyph() },
        SymbolName: String { get: |this: &Rc<IconData>| this.symbol_name().unwrap_or_default() },
        IsSegoeFluentOnly: bool { get: |this: &Rc<IconData>| this.record.is_segoe_fluent_only },
    ],
});

/// `IconsDataSource.Icons`: every icon of the source, in its order.
pub(crate) fn icons() -> Vec<Rc<IconData>> {
    crate::icons_data::ICONS
        .iter()
        .map(|record| Rc::new(IconData { record }))
        .collect()
}

/// The icons that fit every space-separated word of a search (the source's `Filter`).
pub(crate) fn filter<'a>(icons: impl IntoIterator<Item = &'a Rc<IconData>>, search: &str) -> Vec<Rc<IconData>> {
    let words: Vec<String> = search.split(' ').map(str::to_lowercase).collect();
    icons
        .into_iter()
        .filter(|icon| icon.fits_filter(&words))
        .cloned()
        .collect()
}

/// The announcement of a search (the source's `UIHelper.AnnounceActionForAccessibility` text).
pub(crate) fn found_text(count: usize) -> String {
    match count {
        0 => "No icons found.".to_string(),
        1 => "1 icon found.".to_string(),
        count => format!("{count} icons found."),
    }
}

fn items(icons: &[Rc<IconData>]) -> ItemsSource {
    ItemsSource::from_items(icons.iter().map(|icon| Some(Rc::new(icon.clone()) as BoxedValue)))
}

/// The page's state: the icons of the source and those the search shows. The handlers of the
/// page's controls hold it, and it holds the page weakly, so replacing the page releases both.
struct IconographyPage {
    all: Vec<Rc<IconData>>,
    shown: RefCell<Vec<Rc<IconData>>>,
    root: WeakRef<Control>,
}

impl IconographyPage {
    fn wire(root: &Ref<Control>) {
        let page = Rc::new(Self {
            all: icons(),
            shown: RefCell::new(Vec::new()),
            root: root.downgrade(),
        });

        // IconsItemsView_Loaded: the icons, the first one selected and the side panel shown. The
        // repeater of the view realizes only the icons in view, so they are set at once.
        page.show(root, page.all.clone());
        root.get_control::<Border>("SidePanel").set_is_visible(true);

        let handler = page.clone();
        let search = root.get_control::<AutoSuggestBox>("IconsAutoSuggestBox");
        let sender = search.downgrade();
        search
            .text_changed(move |_| {
                if let Some(search) = sender.upgrade() {
                    handler.filter(&search.text());
                }
            })
            .detach();
        let handler = page.clone();
        root.get_control::<ItemsView>("IconsItemsView")
            .selection_changed(move |_| handler.on_selection_changed());
        let weak = root.downgrade();
        root.get_control::<ItemsView>("TagsItemsView")
            .item_invoked(move |args| {
                let tag = args
                    .invoked_item
                    .as_ref()
                    .and_then(|item| item.downcast_ref::<String>().cloned());
                if let (Some(root), Some(tag)) = (weak.upgrade(), tag) {
                    root.get_control::<AutoSuggestBox>("IconsAutoSuggestBox").set_text(tag);
                }
            });
    }

    /// `Filter`: the icons of a search; the first is selected. The filter runs on the UI thread
    /// (the source starts a thread for it).
    fn filter(&self, search: &str) {
        let Some(root) = self.root.upgrade() else { return };
        let found = filter(&self.all, search);
        root.get_control::<TextBlock>("IconsFoundText")
            .set_text(Some(&found_text(found.len())));
        self.show(&root, found);
    }

    fn show(&self, root: &Ref<Control>, icons: Vec<Rc<IconData>>) {
        let view = root.get_control::<ItemsView>("IconsItemsView");
        let first = icons.first().cloned();
        view.set_items_source(Some(items(&icons)));
        *self.shown.borrow_mut() = icons;
        if let Some(first) = first {
            set_selected_item(root, &first);
            view.select(0);
        }
    }

    /// `IconsItemsView_SelectionChanged`: the current item of the view is the selected icon.
    fn on_selection_changed(&self) {
        let Some(root) = self.root.upgrade() else { return };
        let index = root.get_control::<ItemsView>("IconsItemsView").current_item_index();
        let icon = usize::try_from(index)
            .ok()
            .and_then(|index| self.shown.borrow().get(index).cloned());
        if let Some(icon) = icon {
            set_selected_item(&root, &icon);
        }
    }
}

/// The `SelectedItem` setter with `SetSampleCodePresenterCode`, and the `x:Load` of the tags and of
/// their empty text.
fn set_selected_item(root: &Ref<Control>, icon: &Rc<IconData>) {
    let text = |name: &str, value: &str| root.get_control::<TextBlock>(name).set_text(Some(value));
    root.get_control::<Border>("SidePanel")
        .set_data_context(Some(Rc::new(icon.clone()) as BoxedValue));
    text(
        "XAMLCodePresenterFont",
        &format!("<FontIcon Glyph=\"{}\" />", icon.text_glyph()),
    );
    text(
        "CSharpCodePresenterFont",
        &format!(
            "FontIcon icon = new FontIcon();\nicon.Glyph = \"{}\";",
            icon.code_glyph()
        ),
    );
    let symbol = icon.symbol_name();
    match &symbol {
        Some(symbol) => {
            text(
                "XAMLCodePresenterSymbol",
                &format!("<SymbolIcon Symbol=\"{symbol}\" />"),
            );
            text(
                "CSharpCodePresenterSymbol",
                &format!("SymbolIcon icon = new SymbolIcon();\nicon.Symbol = Symbol.{symbol};"),
            );
        }
        None => {
            text("XAMLCodePresenterSymbol", "");
            text("CSharpCodePresenterSymbol", "");
        }
    }
    for label in ["SymbolXamlLabel", "SymbolCSharpLabel"] {
        root.get_control::<TextBlock>(label).set_is_visible(symbol.is_some());
    }
    let has_tags = !icon.tags().is_empty();
    let tags = root.get_control::<ItemsView>("TagsItemsView");
    tags.set_items_source(Some(ItemsSource::from_items(
        icon.tags()
            .iter()
            .map(|tag| Some(Rc::new(tag.to_string()) as BoxedValue)),
    )));
    tags.set_is_visible(has_tags);
    root.get_control::<TextBlock>("NoTagsTextBlock")
        .set_is_visible(!has_tags);
}

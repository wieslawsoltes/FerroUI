//! Test-only translation of the pinned shared runtime SampleRibbon.cs.
//! Copyright Wiesław Šoltés, MIT. Original identities and glyphs are retained.
use crate::{compat::MenuFlyoutItem, controls::*};
use ferroribbon_core::{
    RibbonContextualActivation, RibbonGroupItemsLayout, RibbonGroupState, RibbonItemSize,
};
use ferroui_base::{
    Ref,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    layout::{HorizontalAlignment, VerticalAlignment},
    threading::Dispatcher,
};
use ferroui_controls::{Application, Border, Control, MenuFlyout, TextBlock, Window};
use std::rc::Rc;

fn tab(id: &str, header: &str) -> Ref<RibbonTab> {
    let value = RibbonTab::new();
    value.set_id(Some(id.into()));
    value.set_header(Some(header.into()));
    value
}
fn group(id: &str, header: &str) -> Ref<RibbonGroup> {
    let value = RibbonGroup::new();
    value.set_id(Some(id.into()));
    value.set_header(Some(header.into()));
    value
}
fn button(id: &str, label: &str, glyph: &str) -> Ref<RibbonButton> {
    let value = RibbonButton::new();
    value.set_id(Some(id.into()));
    value.set_label(Some(label.into()));
    value.set_icon(Some(Rc::new(glyph.to_owned())));
    value
}
fn toggle(id: &str, label: &str, glyph: &str) -> Ref<RibbonToggleButton> {
    let value = RibbonToggleButton::new();
    value.set_id(Some(id.into()));
    value.set_label(Some(label.into()));
    value.set_icon(Some(Rc::new(glyph.to_owned())));
    value
}
pub(crate) fn menu(labels: &[&str]) -> Ref<MenuFlyout> {
    let menu = MenuFlyout::new();
    for label in labels {
        let item = MenuFlyoutItem::new();
        item.set_text(Some((*label).into()));
        menu.items().add(Some(Control::boxed(item)));
    }
    menu
}
pub(crate) fn create() -> Ref<Ribbon> {
    let ribbon = Ribbon::new();
    let home = tab("home", "Home");
    let clipboard = group("clipboard", "Clipboard");
    clipboard.set_is_dialog_launcher_visible(true);
    let paste = RibbonSplitButton::new();
    paste.set_id(Some("paste".into()));
    paste.set_label(Some("Paste".into()));
    paste.set_icon(Some(Rc::new("".to_owned())));
    paste.set_size_definition(Some("Large".into()));
    paste.set_flyout(Some(menu(&["Keep Source", "Text Only"]).upcast()));
    clipboard.items().add(Some(paste.upcast()));
    let cut = button("cut", "Cut", "");
    cut.set_shortcut(Some("Ctrl+X".into()));
    clipboard.items().add(Some(cut.upcast()));
    let copy = button("copy", "Copy", "");
    copy.set_command_id(Some("copy".into()));
    clipboard.items().add(Some(copy.upcast()));
    let painter = toggle("painter", "Format Painter", "");
    painter.set_size(RibbonItemSize::Medium);
    clipboard.items().add(Some(painter.upcast()));
    home.groups().add(Some(clipboard));

    let font = group("font", "Font");
    font.set_items_layout(RibbonGroupItemsLayout::Rows);
    font.set_row_count(2);
    let row1 = RibbonStackPanel::new();
    let family = RibbonFontComboBox::new();
    family.set_id(Some("fontFamily".into()));
    family.set_text(Some("Segoe UI".into()));
    row1.items().add(Some(family.upcast()));
    let size = RibbonFontSizeComboBox::new();
    size.set_id(Some("fontSize".into()));
    size.set_text(Some("11".into()));
    row1.items().add(Some(size.upcast()));
    let row2 = RibbonStackPanel::new();
    let styles = RibbonButtonGroup::new();
    let bold = toggle("bold", "Bold", "");
    bold.set_shortcut(Some("Ctrl+B".into()));
    bold.set_key_tip(Some("1".into()));
    styles.items().add(Some(bold.upcast()));
    let italic = toggle("italic", "Italic", "");
    italic.set_key_tip(Some("2".into()));
    styles.items().add(Some(italic.upcast()));
    row2.items().add(Some(styles.upcast()));
    let color = RibbonColorPicker::new();
    color.set_id(Some("fontColor".into()));
    color.set_label(Some("Font Color".into()));
    row2.items().add(Some(color.upcast()));
    font.items().add(Some(row1.upcast()));
    font.items().add(Some(row2.upcast()));
    home.groups().add(Some(font));

    let paragraph = group("paragraph", "Paragraph");
    for (id, label, glyph) in [
        ("left", "Align Left", ""),
        ("center", "Center", ""),
        ("right", "Align Right", ""),
    ] {
        let item = toggle(id, label, glyph);
        item.set_group_name(Some("align".into()));
        item.set_size(RibbonItemSize::Medium);
        item.set_is_checked(Some(id == "left"));
        paragraph.items().add(Some(item.upcast()));
    }
    home.groups().add(Some(paragraph));
    let gallery_group = group("styles", "Styles");
    gallery_group.set_reduction_order(1);
    let gallery = RibbonGallery::new();
    gallery.set_id(Some("stylesGallery".into()));
    gallery.set_label(Some("Styles".into()));
    gallery.set_max_columns(5);
    gallery.set_min_columns(2);
    for name in [
        "Normal",
        "No Spacing",
        "Heading 1",
        "Heading 2",
        "Title",
        "Subtitle",
        "Quote",
    ] {
        let item = RibbonGalleryItem::new();
        item.set_label(Some(name.into()));
        item.set_item_value(Some(Rc::new(name.to_owned())));
        item.set_preview_text(Some("AaBb".into()));
        item.set_category(Some(
            if name.starts_with("Heading") {
                "Headings"
            } else {
                "Paragraph"
            }
            .into(),
        ));
        gallery.items().add_native(Some(Rc::new(item)));
    }
    gallery_group.items().add(Some(gallery.upcast()));
    home.groups().add(Some(gallery_group));
    let editing = group("editing", "Editing");
    for (id, label, glyph) in [("find", "Find", ""), ("replace", "Replace", "")] {
        let item = button(id, label, glyph);
        item.set_size(RibbonItemSize::Large);
        editing.items().add(Some(item.upcast()));
    }
    home.groups().add(Some(editing));
    ribbon.tabs().add(Some(home));

    let insert = tab("insert", "Insert");
    let tables = group("tables", "Tables");
    let table = button("table", "Table", "");
    table.set_size(RibbonItemSize::Large);
    tables.items().add(Some(table.upcast()));
    insert.groups().add(Some(tables));
    ribbon.tabs().add(Some(insert));
    let contextual = RibbonContextualTabGroup::new();
    contextual.set_id(Some("tableTools".into()));
    contextual.set_header(Some("Table Tools".into()));
    contextual.set_activation(RibbonContextualActivation::SelectOnShow);
    ribbon.contextual_groups().add(Some(contextual));
    let design = tab("tableDesign", "Table Design");
    design.set_contextual_group_id(Some("tableTools".into()));
    let options = group("tableOptions", "Options");
    let header = RibbonCheckBox::new();
    header.set_id(Some("headerRow".into()));
    header.set_label(Some("Header Row".into()));
    header.set_is_checked(Some(true));
    options.items().add(Some(header.upcast()));
    design.groups().add(Some(options));
    ribbon.tabs().add(Some(design));
    ribbon
        .quick_access_tool_bar()
        .unwrap()
        .items()
        .add(Some(button("save", "Save", "").upcast()));
    let backstage = RibbonBackstage::new();
    backstage.set_title(Some("Tests".into()));
    let info = RibbonBackstageItem::new();
    info.set_id(Some("info".into()));
    info.set_header(Some("Info".into()));
    let content = TextBlock::new();
    content.set_text("Info page".into());
    info.set_content(Some(Control::boxed(content)));
    backstage.items().add(Some(info));
    let save = RibbonBackstageItem::new();
    save.set_id(Some("saveAction".into()));
    save.set_header(Some("Save".into()));
    backstage.items().add(Some(save));
    ribbon.set_backstage(Some(backstage));
    ribbon
}
pub(crate) fn item<T: ferroui_base::ObjectType>(ribbon: &Ribbon, id: &str) -> Ref<T> {
    ribbon
        .find_item(Some(id.into()))
        .unwrap()
        .cast::<T>()
        .unwrap()
}
pub(crate) fn find_group(ribbon: &Ribbon, id: &str) -> Ref<RibbonGroup> {
    ribbon
        .tabs()
        .iter()
        .flatten()
        .flat_map(|tab| tab.groups().iter().flatten())
        .find(|group| group.id().as_deref() == Some(id))
        .unwrap()
}

pub(crate) struct RuntimeHost {
    container: Ref<Border>,
    window: Ref<Window>,
    resources: Ref<ResourceDictionary>,
}
impl RuntimeHost {
    pub(crate) fn new(control: Ref<Control>, width: Option<f64>) -> Self {
        Self::with_setup(|| control, width, || {})
    }
    pub(crate) fn with_setup(
        create: impl FnOnce() -> Ref<Control>,
        width: Option<f64>,
        setup: impl FnOnce(),
    ) -> Self {
        let app = Application::current().unwrap();
        let resources = app.resources();
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Generic.xaml")
                .unwrap()
                .unwrap();
        let dictionary = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        app.set_resources(dictionary);
        setup();
        let control = create();
        let container = Border::new();
        container.set_width(width.unwrap_or(f64::NAN));
        container.set_horizontal_alignment(if width.is_some() {
            HorizontalAlignment::Left
        } else {
            HorizontalAlignment::Stretch
        });
        container.set_vertical_alignment(VerticalAlignment::Top);
        container.set_child(Some(control));
        let window = Window::new();
        window.set_width(1500.);
        window.set_height(900.);
        window.set_content(Some(Control::boxed(container.clone())));
        window.show();
        let host = Self {
            container,
            window,
            resources,
        };
        host.settle();
        host
    }
    pub(crate) fn settle(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
    }
    pub(crate) fn resize(&self, width: f64) {
        self.container.set_width(width);
        self.settle();
    }
}
impl Drop for RuntimeHost {
    fn drop(&mut self) {
        self.window.close();
        Application::current()
            .unwrap()
            .set_resources(self.resources.clone());
    }
}

pub(crate) struct Mounted {
    pub(crate) ribbon: Ref<Ribbon>,
    host: RuntimeHost,
}
impl std::ops::Deref for Mounted {
    type Target = RuntimeHost;
    fn deref(&self) -> &Self::Target {
        &self.host
    }
}
impl Mounted {
    pub(crate) fn new(width: f64) -> Self {
        Self::with_ribbon(create(), width)
    }
    pub(crate) fn with_ribbon(ribbon: Ref<Ribbon>, width: f64) -> Self {
        let host = RuntimeHost::new(ribbon.clone().upcast(), Some(width));
        Self { ribbon, host }
    }
    pub(crate) fn groups(&self) -> Vec<Ref<RibbonGroup>> {
        self.ribbon
            .selected_tab()
            .unwrap()
            .groups()
            .iter()
            .flatten()
            .collect()
    }
    pub(crate) fn states(&self) -> Vec<RibbonGroupState> {
        self.groups().iter().map(|g| g.state()).collect()
    }
}

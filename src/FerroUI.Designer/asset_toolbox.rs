//! Reusable, searchable native asset catalog. Tiles retain standard Button focus,
//! keyboard, automation and theme behavior; inserting an asset stays host-owned.
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Geometry, SolidColorBrush, TextTrimming};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Button, ComboBox, Control, Grid, PathIcon, RowDefinitions, ScrollViewer, StackPanel, TextBlock,
    TextBox, ToolTip, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

/// A source snippet available to a designer host. Custom controls can register
/// their own category, description and vector icon without modifying the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignerAsset {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub source: String,
    pub icon: AssetIcon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetIcon {
    Control,
    Text,
    Input,
    Toggle,
    Panel,
    Stack,
    Canvas,
    Rectangle,
    Ellipse,
    Value,
    Content,
    /// A project-owned vector, in the same 24 × 24 coordinate space as the built-ins.
    CustomPath(String),
}
impl AssetIcon {
    fn path(&self) -> &str {
        match self {
            Self::Control => "M3 5H21V19H3Z M5 7V17H19V7Z M7 10H17V14H7Z",
            Self::Text => "M3 4H21V7H14V21H10V7H3Z",
            Self::Input => "M2 5H22V19H2Z M4 7V17H20V7Z M7 8H13V10H11V14H13V16H7V14H9V10H7Z",
            Self::Toggle => "M3 3H21V21H3Z M5 5V19H19V5Z M7 11L10 14L17 7L18.5 8.5L10 17L5.5 12.5Z",
            Self::Panel => "M3 3H10V10H3Z M14 3H21V10H14Z M3 14H10V21H3Z M14 14H21V21H14Z",
            Self::Stack => "M3 3H21V7H3Z M3 10H21V14H3Z M3 17H21V21H3Z",
            Self::Canvas => "M3 3H5V19H21V21H3Z M8 5H17V12H8Z M12 14H21V17H12Z",
            Self::Rectangle => "M3 5H21V19H3Z M5 7V17H19V7Z",
            Self::Ellipse => {
                "M12 2A10 10 0 1 1 12 22A10 10 0 1 1 12 2Z M12 4A8 8 0 1 0 12 20A8 8 0 1 0 12 4Z"
            }
            Self::Value => "M2 11H7V13H2Z M13 11H22V13H13Z M8 5H12V19H8Z",
            Self::Content => {
                "M3 3H21V21H3Z M5 5V19H19V5Z M6 16L10 11L13 14L16 10L19 17H6Z M7 7H10V10H7Z"
            }
            Self::CustomPath(path) => path,
        }
    }
}

type Activate = Rc<dyn Fn(&DesignerAsset)>;
type BeginDrag = Rc<dyn Fn(&DesignerAsset, &PointerPressedEventArgs)>;

pub struct AssetToolbox {
    root: Ref<Grid>,
    search: Ref<TextBox>,
    category: Ref<ComboBox>,
    categories: RefCell<Vec<String>>,
    tiles: Ref<WrapPanel>,
    count: Ref<TextBlock>,
    catalog: RefCell<Vec<DesignerAsset>>,
    visible: RefCell<Vec<String>>,
    activate: RefCell<Option<Activate>>,
    begin_drag: RefCell<Option<BeginDrag>>,
    guard: Cell<bool>,
}
impl AssetToolbox {
    pub fn new(assets: Vec<DesignerAsset>) -> Rc<Self> {
        let root = Grid::new();
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto,*,Auto").unwrap());
        root.classes().add("designer-assets");
        let search = TextBox::new();
        search.set_name(Some("AssetSearch".into()));
        search.set_placeholder_text(Some("Search controls"));
        search.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        AutomationProperties::set_name(&search, Some("Search assets"));
        root.children().add(search.clone().upcast::<Control>());
        let category = ComboBox::new();
        category.set_name(Some("AssetCategory".into()));
        category.set_horizontal_alignment(HorizontalAlignment::Stretch);
        category.set_margin(Thickness::new(0.0, 0.0, 0.0, 10.0));
        AutomationProperties::set_name(&category, Some("Asset category"));
        Grid::set_row(&category, 1);
        root.children().add(category.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let tiles = WrapPanel::new();
        tiles.set_item_width(104.0);
        tiles.set_item_height(78.0);
        tiles.set_item_spacing(4.0);
        scroll.set_content(Some(Control::boxed(&tiles)));
        Grid::set_row(&scroll, 2);
        root.children().add(scroll.upcast::<Control>());
        let count = TextBlock::new();
        count.set_font_size(11.0);
        count.set_opacity(0.62);
        count.set_margin(Thickness::new(2.0, 8.0, 0.0, 0.0));
        Grid::set_row(&count, 3);
        root.children().add(count.clone().upcast::<Control>());
        let this = Rc::new(Self {
            root,
            search,
            category,
            categories: RefCell::new(Vec::new()),
            tiles,
            count,
            catalog: RefCell::new(Vec::new()),
            visible: RefCell::new(Vec::new()),
            activate: RefCell::new(None),
            begin_drag: RefCell::new(None),
            guard: Cell::new(false),
        });
        // Register through the same replacement path hosts use for extensions.
        for asset in assets {
            this.upsert(asset);
        }
        this.rebuild_categories();
        let weak = Rc::downgrade(&this);
        this.search.text_changing(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.refresh_tiles();
            }
        });
        let weak = Rc::downgrade(&this);
        this.category.selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this.guard.get() {
                    this.refresh_tiles();
                }
            }
        });
        this.refresh_tiles();
        this
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn query(&self) -> String {
        self.search.text().unwrap_or_default()
    }
    pub fn set_query(&self, query: &str) {
        self.search.set_text(Some(query));
    }
    pub fn visible_asset_ids(&self) -> Vec<String> {
        self.visible.borrow().clone()
    }
    pub fn assets(&self) -> Vec<DesignerAsset> {
        self.catalog.borrow().clone()
    }
    pub fn on_activate(&self, callback: impl Fn(&DesignerAsset) + 'static) {
        *self.activate.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_begin_drag(
        &self,
        callback: impl Fn(&DesignerAsset, &PointerPressedEventArgs) + 'static,
    ) {
        *self.begin_drag.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn register(self: &Rc<Self>, asset: DesignerAsset) -> Result<(), String> {
        if asset.id.trim().is_empty()
            || asset.title.trim().is_empty()
            || asset.category.trim().is_empty()
            || asset.source.trim().is_empty()
        {
            return Err("An asset requires an id, title, category and source snippet".into());
        }
        Geometry::parse(asset.icon.path())
            .map_err(|error| format!("Invalid asset icon: {error}"))?;
        self.upsert(asset);
        self.rebuild_categories();
        self.refresh_tiles();
        Ok(())
    }
    pub fn set_category(&self, category: Option<&str>) -> bool {
        let index = category.map_or(Some(0), |category| {
            self.categories
                .borrow()
                .iter()
                .position(|name| name == category)
                .map(|index| index + 1)
        });
        if let Some(index) = index {
            self.category.set_selected_index(index as i32);
            true
        } else {
            false
        }
    }
    fn upsert(&self, asset: DesignerAsset) {
        let mut catalog = self.catalog.borrow_mut();
        if let Some(existing) = catalog.iter_mut().find(|entry| entry.id == asset.id) {
            *existing = asset;
        } else {
            catalog.push(asset);
        }
    }
    fn rebuild_categories(&self) {
        let selected = self.selected_category();
        let categories: Vec<_> = self
            .catalog
            .borrow()
            .iter()
            .map(|asset| asset.category.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.guard.set(true);
        self.category.items().clear();
        self.category
            .items()
            .add(Some(Rc::new("All controls".to_owned())));
        for category in &categories {
            self.category.items().add(Some(Rc::new(category.clone())));
        }
        let index = selected
            .and_then(|selected| categories.iter().position(|name| *name == selected))
            .map_or(0, |index| index + 1);
        *self.categories.borrow_mut() = categories;
        self.category.set_selected_index(index as i32);
        self.guard.set(false);
    }
    fn selected_category(&self) -> Option<String> {
        let index = usize::try_from(self.category.selected_index())
            .ok()?
            .checked_sub(1)?;
        self.categories.borrow().get(index).cloned()
    }
    fn refresh_tiles(self: &Rc<Self>) {
        let query = self.query().to_lowercase();
        let terms: Vec<_> = query.split_whitespace().collect();
        let category = self.selected_category();
        let assets: Vec<_> = self
            .catalog
            .borrow()
            .iter()
            .filter(|asset| {
                if category
                    .as_ref()
                    .is_some_and(|category| *category != asset.category)
                {
                    return false;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    asset.title, asset.id, asset.category, asset.description
                )
                .to_lowercase();
                terms.iter().all(|term| haystack.contains(term))
            })
            .cloned()
            .collect();
        self.tiles.children().clear();
        *self.visible.borrow_mut() = assets.iter().map(|asset| asset.id.clone()).collect();
        self.count.set_text(Some(&if assets.is_empty() {
            "No matching controls".to_owned()
        } else {
            format!("{} controls · drag to the artboard", assets.len())
        }));
        for asset in assets {
            let tile = Button::new();
            tile.classes().add("asset-tile");
            tile.set_margin(Thickness::uniform(2.0));
            tile.set_padding(Thickness::symmetric(4.0, 6.0));
            tile.set_horizontal_alignment(HorizontalAlignment::Stretch);
            tile.set_vertical_alignment(VerticalAlignment::Stretch);
            tile.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            tile.set_vertical_content_alignment(VerticalAlignment::Center);
            AutomationProperties::set_name(&tile, Some(&format!("Insert {}", asset.title)));
            AutomationProperties::set_automation_id(&tile, Some(&format!("Asset.{}", asset.id)));
            AutomationProperties::set_help_text(&tile, Some(&asset.description));
            ToolTip::set_tip(
                &tile,
                Some(Rc::new(format!(
                    "{}\n{}\nDrag to place, or press Enter to choose a location",
                    asset.title, asset.description
                ))),
            );
            let content = StackPanel::new();
            content.set_spacing(7.0);
            let icon = PathIcon::new();
            icon.set_width(22.0);
            icon.set_height(22.0);
            icon.set_horizontal_alignment(HorizontalAlignment::Center);
            icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff99d8ff).into()));
            icon.set_data(Geometry::parse(asset.icon.path()).unwrap_or_else(|_| {
                Geometry::parse(AssetIcon::Control.path()).expect("built-in asset vector")
            }));
            content.children().add(icon.upcast::<Control>());
            let title = TextBlock::new();
            title.set_text(Some(&asset.title));
            title.set_font_size(11.0);
            title.set_horizontal_alignment(HorizontalAlignment::Center);
            title.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
            content.children().add(title.upcast::<Control>());
            tile.set_content(Some(Control::boxed(&content)));
            let weak = Rc::downgrade(self);
            let drag_asset = asset.clone();
            // Button handles pointer presses in its class handler. Observe the
            // handled bubble event so native drag initiation still reaches the host.
            tile.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, event| {
                    if event.properties().is_left_button_pressed {
                        if let Some(this) = weak.upgrade() {
                            let callback = this.begin_drag.borrow().clone();
                            if let Some(callback) = callback {
                                callback(&drag_asset, event);
                            }
                        }
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            );
            let weak = Rc::downgrade(self);
            tile.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let callback = this.activate.borrow().clone();
                    if let Some(callback) = callback {
                        callback(&asset);
                    }
                }
            });
            self.tiles.children().add(tile.upcast::<Control>());
        }
    }
}

/// Practical defaults rather than runtime-only infrastructure types. Hosts may
/// append project-specific controls using [`AssetToolbox::register`].
pub fn default_designer_assets() -> Vec<DesignerAsset> {
    use AssetIcon::*;
    [
        ("Button", "Controls", "A command or action", "<Button Content=\"Button\" Width=\"120\" Height=\"36\" />", Control),
        ("TextBlock", "Controls", "Display text and headings", "<TextBlock Text=\"Text\" FontSize=\"18\" />", Text),
        ("TextBox", "Controls", "Editable text input", "<TextBox Text=\"Edit me\" Width=\"180\" />", Input),
        ("CheckBox", "Controls", "An independent selection", "<CheckBox Content=\"Option\" />", Toggle),
        ("RadioButton", "Controls", "Choose one option", "<RadioButton Content=\"Option\" />", Toggle),
        ("ToggleSwitch", "Controls", "An on/off setting", "<ToggleSwitch OnContent=\"On\" OffContent=\"Off\" />", Toggle),
        ("Slider", "Controls", "Choose a numeric value", "<Slider Width=\"180\" Minimum=\"0\" Maximum=\"100\" Value=\"40\" />", Value),
        ("ProgressBar", "Controls", "Display progress", "<ProgressBar Width=\"180\" Height=\"6\" Value=\"60\" />", Value),
        ("ComboBox", "Controls", "A compact choice list", "<ComboBox Width=\"160\" PlaceholderText=\"Choose an option\" />", Input),
        ("Border", "Layout", "Decorate and frame content", "<Border Width=\"180\" Height=\"100\" Background=\"#FFE0E7FF\" CornerRadius=\"8\" />", Rectangle),
        ("Grid", "Layout", "Arrange rows and columns", "<Grid Width=\"220\" Height=\"160\" Background=\"#FFF1F5F9\" RowDefinitions=\"*,*\" ColumnDefinitions=\"*,*\" />", Panel),
        ("StackPanel", "Layout", "Arrange a vertical stack", "<StackPanel Width=\"180\" Height=\"140\" Background=\"#FFF1F5F9\" Spacing=\"8\" />", Stack),
        ("Canvas", "Layout", "Position content precisely", "<Canvas Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Canvas),
        ("WrapPanel", "Layout", "Flow content into new rows", "<WrapPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Panel),
        ("DockPanel", "Layout", "Dock content to edges", "<DockPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Panel),
        ("UniformGrid", "Layout", "Arrange equal-size cells", "<UniformGrid Width=\"240\" Height=\"160\" Rows=\"2\" Columns=\"2\" />", Panel),
        ("Viewbox", "Layout", "Scale content to fit", "<Viewbox Width=\"160\" Height=\"120\"><TextBlock Text=\"Scale\" /></Viewbox>", Content),
        ("ScrollViewer", "Layout", "A scrollable content viewport", "<ScrollViewer Width=\"220\" Height=\"160\"><TextBlock Text=\"Scrollable content\" /></ScrollViewer>", Content),
        ("Rectangle", "Shapes", "A rectangle or card background", "<Rectangle Width=\"120\" Height=\"80\" Fill=\"#FF60CDFF\" />", Rectangle),
        ("Ellipse", "Shapes", "A circle or ellipse", "<Ellipse Width=\"100\" Height=\"100\" Fill=\"#FFA78BFA\" />", Ellipse),
    ].into_iter().map(|(title, category, description, source, icon)| DesignerAsset {
        id: title.into(), title: title.into(), category: category.into(),
        description: description.into(), source: source.into(), icon,
    }).collect()
}

#[cfg(test)]
#[path = "asset_toolbox_tests.rs"]
mod tests;

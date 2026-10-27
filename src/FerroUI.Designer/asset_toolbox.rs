//! Reusable, searchable native asset catalog. Tiles retain standard Button focus,
//! keyboard, automation and theme behavior; inserting an asset stays host-owned.
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, Geometry, SolidColorBrush, TextTrimming, TextWrapping};
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ComboBox, Control, Grid, PathIcon, RowDefinitions,
    ScrollViewer, StackPanel, TextBlock, TextBox, ToolTip, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
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
    Radio,
    Switch,
    Combo,
    Progress,
    Panel,
    Dock,
    Wrap,
    Uniform,
    Frame,
    Viewbox,
    Scroll,
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
            Self::Radio => {
                "M12 2A10 10 0 1 1 12 22A10 10 0 1 1 12 2Z M12 4A8 8 0 1 0 12 20A8 8 0 1 0 12 4Z M12 8A4 4 0 1 1 12 16A4 4 0 1 1 12 8Z"
            }
            Self::Switch => {
                "M8 5H16A7 7 0 0 1 16 19H8A7 7 0 0 1 8 5Z M8 7A5 5 0 0 0 8 17H16A5 5 0 0 0 16 7Z M16 9A3 3 0 1 1 16 15A3 3 0 1 1 16 9Z"
            }
            Self::Combo => {
                "M2 5H22V19H2Z M4 7V17H20V7Z M6 10H11V12H6Z M14 10L17 13L20 10L18.5 8.5L17 10L15.5 8.5Z"
            }
            Self::Progress => "M2 7H22V17H2Z M4 9V15H20V9Z M5 10H14V14H5Z",
            Self::Panel => "M3 3H21V21H3Z M5 5V19H19V5Z M3 9H21V11H3Z M10 3H12V21H10Z",
            Self::Dock => "M3 3H21V21H3Z M5 9V19H19V9Z M5 5V7H19V5Z M10 9H12V21H10Z",
            Self::Wrap => {
                "M3 4H15V8H3Z M17 4H21V8H17Z M3 10H9V14H3Z M11 10H21V14H11Z M3 16H15V20H3Z"
            }
            Self::Uniform => {
                "M3 3H10V10H3Z M5 5V8H8V5Z M14 3H21V10H14Z M16 5V8H19V5Z M3 14H10V21H3Z M5 16V19H8V16Z M14 14H21V21H14Z M16 16V19H19V16Z"
            }
            Self::Frame => {
                "M7 3H17Q21 3 21 7V17Q21 21 17 21H7Q3 21 3 17V7Q3 3 7 3Z M7 5Q5 5 5 7V17Q5 19 7 19H17Q19 19 19 17V7Q19 5 17 5Z"
            }
            Self::Viewbox => {
                "M3 3H10V5H5V10H3Z M14 3H21V10H19V5H14Z M3 14H5V19H10V21H3Z M19 14H21V21H14V19H19Z M8 8H16V16H8Z M10 10V14H14V10Z"
            }
            Self::Scroll => {
                "M3 3H21V21H3Z M5 5V19H19V5Z M16 7H18V13H16Z M7 7H13V9H7Z M7 11H13V13H7Z M7 15H13V17H7Z"
            }
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

/// The asset library keeps the same filtering and insertion semantics in both presentations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssetViewMode {
    /// Visual cards for browsing shapes and controls.
    #[default]
    Grid,
    /// Compact rows with descriptions for name-led browsing.
    List,
}

type Activate = Rc<dyn Fn(&DesignerAsset)>;
type BeginDrag = Rc<dyn Fn(&DesignerAsset, &PointerPressedEventArgs)>;

pub struct AssetToolbox {
    root: Ref<Grid>,
    search: Ref<TextBox>,
    category: Ref<ComboBox>,
    categories: RefCell<Vec<String>>,
    groups: Ref<StackPanel>,
    buttons: RefCell<Vec<Ref<Button>>>,
    grid_view: Ref<Button>,
    list_view: Ref<Button>,
    view_mode: Cell<AssetViewMode>,
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
        search.classes().add("designer-search");
        search.set_placeholder_text(Some("Search assets"));
        search.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        let search_icon = toolbox_icon(
            "M10 2A8 8 0 1 1 5 16.25A8 8 0 0 1 10 2Z M10 4A6 6 0 1 0 10 16A6 6 0 1 0 10 4Z M15.5 14L22 20.5L20.5 22L14 15.5Z",
            14.0,
            0xff828795,
        );
        search_icon.set_margin(Thickness::new(7.0, 0.0, 1.0, 0.0));
        search.set_inner_left_content(Some(Control::boxed(&search_icon)));
        AutomationProperties::set_name(&search, Some("Search assets"));
        root.children().add(search.clone().upcast::<Control>());
        let filters = Grid::new();
        filters.set_column_definitions(ColumnDefinitions::parse("*,30,30").unwrap());
        filters.set_margin(Thickness::new(0.0, 0.0, 0.0, 12.0));
        let category = ComboBox::new();
        category.set_name(Some("AssetCategory".into()));
        category.set_horizontal_alignment(HorizontalAlignment::Stretch);
        category.set_margin(Thickness::new(0.0, 0.0, 6.0, 0.0));
        AutomationProperties::set_name(&category, Some("Asset category"));
        filters.children().add(category.clone().upcast::<Control>());
        let grid_view = view_button(
            "Grid view",
            "M3 3H10V10H3Z M5 5V8H8V5Z M14 3H21V10H14Z M16 5V8H19V5Z M3 14H10V21H3Z M5 16V19H8V16Z M14 14H21V21H14Z M16 16V19H19V16Z",
        );
        grid_view.classes().add("active");
        Grid::set_column(&grid_view, 1);
        filters
            .children()
            .add(grid_view.clone().upcast::<Control>());
        let list_view = view_button(
            "List view",
            "M3 4H7V8H3Z M10 5H21V7H10Z M3 10H7V14H3Z M10 11H21V13H10Z M3 16H7V20H3Z M10 17H21V19H10Z",
        );
        Grid::set_column(&list_view, 2);
        filters
            .children()
            .add(list_view.clone().upcast::<Control>());
        Grid::set_row(&filters, 1);
        root.children().add(filters.upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let groups = StackPanel::new();
        groups.set_spacing(14.0);
        scroll.set_content(Some(Control::boxed(&groups)));
        Grid::set_row(&scroll, 2);
        root.children().add(scroll.upcast::<Control>());
        let footer = StackPanel::new();
        footer.set_spacing(4.0);
        footer.set_margin(Thickness::new(0.0, 10.0, 0.0, 4.0));
        let count = TextBlock::new();
        count.set_font_size(11.0);
        count.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        footer.children().add(count.clone().upcast::<Control>());
        let hint = TextBlock::new();
        hint.set_text(Some("Drag to canvas · Enter to insert"));
        hint.set_font_size(10.0);
        hint.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
        footer.children().add(hint.upcast::<Control>());
        Grid::set_row(&footer, 3);
        root.children().add(footer.upcast::<Control>());
        let this = Rc::new(Self {
            root,
            search,
            category,
            categories: RefCell::new(Vec::new()),
            groups,
            buttons: RefCell::new(Vec::new()),
            grid_view,
            list_view,
            view_mode: Cell::new(AssetViewMode::Grid),
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
        for (button, mode) in [
            (&this.grid_view, AssetViewMode::Grid),
            (&this.list_view, AssetViewMode::List),
        ] {
            let weak = Rc::downgrade(&this);
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_view_mode(mode);
                }
            });
        }
        this.refresh_tiles();
        this
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn view_mode(&self) -> AssetViewMode {
        self.view_mode.get()
    }
    /// Switch presentation without changing the category, search or registered catalog.
    pub fn set_view_mode(self: &Rc<Self>, mode: AssetViewMode) {
        if self.view_mode.replace(mode) == mode {
            return;
        }
        for (button, candidate) in [
            (&self.grid_view, AssetViewMode::Grid),
            (&self.list_view, AssetViewMode::List),
        ] {
            if mode == candidate {
                button.classes().add("active");
            } else {
                button.classes().remove("active");
            }
        }
        self.refresh_tiles();
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
        self.groups.children().clear();
        self.buttons.borrow_mut().clear();
        *self.visible.borrow_mut() = assets.iter().map(|asset| asset.id.clone()).collect();
        self.count.set_text(Some(&if assets.is_empty() {
            "No matching controls".to_owned()
        } else {
            format!("{} controls", assets.len())
        }));
        if assets.is_empty() {
            self.groups.children().add(empty_assets());
            return;
        }
        let mut sections = BTreeMap::<String, Vec<DesignerAsset>>::new();
        for asset in assets {
            sections
                .entry(asset.category.clone())
                .or_default()
                .push(asset);
        }
        for (name, assets) in sections {
            let section = StackPanel::new();
            section.set_spacing(6.0);
            let heading = Grid::new();
            heading.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
            heading.set_margin(Thickness::new(2.0, 0.0, 4.0, 0.0));
            let title = TextBlock::new();
            title.set_text(Some(&name));
            title.set_font_size(11.0);
            title.set_font_weight(FontWeight::SemiBold);
            title.set_foreground(Some(SolidColorBrush::from_uint32(0xffb8becc).into()));
            heading.children().add(title.upcast::<Control>());
            let count = TextBlock::new();
            count.set_text(Some(&assets.len().to_string()));
            count.set_font_size(10.0);
            count.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
            Grid::set_column(&count, 1);
            heading.children().add(count.upcast::<Control>());
            section.children().add(heading.upcast::<Control>());
            let grid = WrapPanel::new();
            grid.set_item_width(104.0);
            grid.set_item_height(80.0);
            grid.set_item_spacing(4.0);
            grid.set_line_spacing(4.0);
            let list = StackPanel::new();
            list.set_spacing(3.0);
            let container = if self.view_mode.get() == AssetViewMode::Grid {
                grid.clone().upcast::<Control>()
            } else {
                list.clone().upcast::<Control>()
            };
            section.children().add(container);
            for asset in assets {
                let tile = Button::new();
                tile.classes()
                    .add(if self.view_mode.get() == AssetViewMode::Grid {
                        "asset-tile"
                    } else {
                        "asset-row"
                    });
                tile.set_padding(Thickness::uniform(0.0));
                tile.set_margin(Thickness::uniform(0.0));
                tile.set_horizontal_alignment(HorizontalAlignment::Stretch);
                tile.set_vertical_alignment(VerticalAlignment::Stretch);
                tile.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
                tile.set_vertical_content_alignment(VerticalAlignment::Stretch);
                AutomationProperties::set_name(&tile, Some(&format!("Insert {}", asset.title)));
                AutomationProperties::set_automation_id(
                    &tile,
                    Some(&format!("Asset.{}", asset.id)),
                );
                AutomationProperties::set_help_text(&tile, Some(&asset.description));
                ToolTip::set_tip(
                    &tile,
                    Some(Rc::new(format!(
                        "{}\n{}\nDrag to place, or press Enter to choose a location",
                        asset.title, asset.description
                    ))),
                );
                let content = asset_content(&asset, self.view_mode.get());
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
                self.buttons.borrow_mut().push(tile.clone());
                if self.view_mode.get() == AssetViewMode::Grid {
                    grid.children().add(tile.upcast::<Control>());
                } else {
                    list.children().add(tile.upcast::<Control>());
                }
            }
            self.groups.children().add(section.upcast::<Control>());
        }
    }
}

fn toolbox_icon(path: &str, size: f64, color: u32) -> Ref<PathIcon> {
    let icon = PathIcon::new();
    icon.set_width(size);
    icon.set_height(size);
    icon.set_vertical_alignment(VerticalAlignment::Center);
    icon.set_foreground(Some(SolidColorBrush::from_uint32(color).into()));
    icon.set_data(Geometry::parse(path).unwrap_or_else(|_| {
        Geometry::parse(AssetIcon::Control.path()).expect("built-in asset vector")
    }));
    icon
}

fn view_button(name: &str, path: &str) -> Ref<Button> {
    let button = Button::new();
    button.classes().add("icon");
    button.classes().add("asset-view");
    button.set_width(28.0);
    button.set_height(26.0);
    button.set_padding(Thickness::uniform(6.0));
    button.set_content(Some(Control::boxed(toolbox_icon(path, 14.0, 0xffb8becc))));
    AutomationProperties::set_name(&button, Some(name));
    ToolTip::set_tip(&button, Some(Rc::new(name.to_owned())));
    button
}

fn asset_content(asset: &DesignerAsset, mode: AssetViewMode) -> Ref<Control> {
    let color = match asset.category.as_str() {
        "Layout" => 0xff9caecb,
        "Shapes" => 0xffb6a2f6,
        "Controls" => 0xffb8becc,
        _ => 0xffa4c5b7,
    };
    let icon = toolbox_icon(
        asset.icon.path(),
        if mode == AssetViewMode::Grid {
            24.0
        } else {
            18.0
        },
        color,
    );
    let title = TextBlock::new();
    title.set_text(Some(&asset.title));
    title.set_font_size(11.0);
    title.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    title.set_foreground(Some(SolidColorBrush::from_uint32(0xffe1e4eb).into()));
    if mode == AssetViewMode::Grid {
        let content = Grid::new();
        content.set_row_definitions(RowDefinitions::parse("*,25").unwrap());
        let preview = Border::new();
        preview.set_margin(Thickness::new(4.0, 4.0, 4.0, 0.0));
        preview.set_background(Some(SolidColorBrush::from_uint32(0xff202126).into()));
        preview.set_corner_radius(CornerRadius::uniform(3.0));
        icon.set_horizontal_alignment(HorizontalAlignment::Center);
        preview.set_child(Some(icon.upcast::<Control>()));
        content.children().add(preview.upcast::<Control>());
        title.set_vertical_alignment(VerticalAlignment::Center);
        title.set_horizontal_alignment(HorizontalAlignment::Center);
        title.set_margin(Thickness::symmetric(6.0, 0.0));
        Grid::set_row(&title, 1);
        content.children().add(title.upcast::<Control>());
        content.upcast()
    } else {
        let content = Grid::new();
        content.set_column_definitions(ColumnDefinitions::parse("28,*").unwrap());
        content.set_min_height(36.0);
        content.set_margin(Thickness::new(7.0, 3.0, 6.0, 3.0));
        icon.set_horizontal_alignment(HorizontalAlignment::Left);
        content.children().add(icon.upcast::<Control>());
        let labels = StackPanel::new();
        labels.set_spacing(2.0);
        labels.set_vertical_alignment(VerticalAlignment::Center);
        labels.children().add(title.upcast::<Control>());
        let description = TextBlock::new();
        description.set_text(Some(&asset.description));
        description.set_font_size(10.0);
        description.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        description.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        labels.children().add(description.upcast::<Control>());
        Grid::set_column(&labels, 1);
        content.children().add(labels.upcast::<Control>());
        content.upcast()
    }
}

fn empty_assets() -> Ref<Control> {
    let state = StackPanel::new();
    state.set_spacing(8.0);
    state.set_margin(Thickness::new(8.0, 24.0, 8.0, 24.0));
    state.set_horizontal_alignment(HorizontalAlignment::Center);
    let icon = toolbox_icon(AssetIcon::Panel.path(), 26.0, 0xff697184);
    icon.set_horizontal_alignment(HorizontalAlignment::Center);
    state.children().add(icon.upcast::<Control>());
    let title = TextBlock::new();
    title.set_text(Some("No assets found"));
    title.set_font_size(12.0);
    title.set_horizontal_alignment(HorizontalAlignment::Center);
    title.set_foreground(Some(SolidColorBrush::from_uint32(0xffb8becc).into()));
    state.children().add(title.upcast::<Control>());
    let hint = TextBlock::new();
    hint.set_text(Some("Try another name or category."));
    hint.set_font_size(11.0);
    hint.set_text_wrapping(TextWrapping::Wrap);
    hint.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
    hint.set_horizontal_alignment(HorizontalAlignment::Center);
    state.children().add(hint.upcast::<Control>());
    state.upcast()
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
        ("RadioButton", "Controls", "Choose one option", "<RadioButton Content=\"Option\" />", Radio),
        ("ToggleSwitch", "Controls", "An on/off setting", "<ToggleSwitch OnContent=\"On\" OffContent=\"Off\" />", Switch),
        ("Slider", "Controls", "Choose a numeric value", "<Slider Width=\"180\" Minimum=\"0\" Maximum=\"100\" Value=\"40\" />", Value),
        ("ProgressBar", "Controls", "Display progress", "<ProgressBar Width=\"180\" Height=\"6\" Value=\"60\" />", Progress),
        ("ComboBox", "Controls", "A compact choice list", "<ComboBox Width=\"160\" PlaceholderText=\"Choose an option\" />", Combo),
        ("Border", "Layout", "Decorate and frame content", "<Border Width=\"180\" Height=\"100\" Background=\"#FFE0E7FF\" CornerRadius=\"8\" />", Frame),
        ("Grid", "Layout", "Arrange rows and columns", "<Grid Width=\"220\" Height=\"160\" Background=\"#FFF1F5F9\" RowDefinitions=\"*,*\" ColumnDefinitions=\"*,*\" />", Panel),
        ("StackPanel", "Layout", "Arrange a vertical stack", "<StackPanel Width=\"180\" Height=\"140\" Background=\"#FFF1F5F9\" Spacing=\"8\" />", Stack),
        ("Canvas", "Layout", "Position content precisely", "<Canvas Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Canvas),
        ("WrapPanel", "Layout", "Flow content into new rows", "<WrapPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Wrap),
        ("DockPanel", "Layout", "Dock content to edges", "<DockPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />", Dock),
        ("UniformGrid", "Layout", "Arrange equal-size cells", "<UniformGrid Width=\"240\" Height=\"160\" Rows=\"2\" Columns=\"2\" />", Uniform),
        ("Viewbox", "Layout", "Scale content to fit", "<Viewbox Width=\"160\" Height=\"120\"><TextBlock Text=\"Scale\" /></Viewbox>", Viewbox),
        ("ScrollViewer", "Layout", "A scrollable content viewport", "<ScrollViewer Width=\"220\" Height=\"160\"><TextBlock Text=\"Scrollable content\" /></ScrollViewer>", Scroll),
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

//! Native component/resource browser. Catalog data and callbacks are host-owned;
//! browsing, filtering and selection never author a document.
use ferroui_base::input::{InputElement, Key, PointerPressedEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, Geometry, SolidColorBrush, TextTrimming};
use ferroui_base::{CornerRadius, Point, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ComboBox, Control, Grid, ListBox, PathIcon, RowDefinitions,
    StackPanel, TextBlock, TextBox, ToolTip,
};
use ferroui_designer_core::NodeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComponentLibraryFilter {
    #[default]
    All,
    Components,
    Templates,
    Themes,
    Resources,
    Dictionaries,
}
impl ComponentLibraryFilter {
    fn title(self) -> &'static str {
        match self {
            Self::All => "All definitions",
            Self::Components => "Components",
            Self::Templates => "Templates",
            Self::Themes => "Control themes and styles",
            Self::Resources => "Resources",
            Self::Dictionaries => "Resource dictionaries",
        }
    }
}
const FILTERS: [ComponentLibraryFilter; 6] = [
    ComponentLibraryFilter::All,
    ComponentLibraryFilter::Components,
    ComponentLibraryFilter::Templates,
    ComponentLibraryFilter::Themes,
    ComponentLibraryFilter::Resources,
    ComponentLibraryFilter::Dictionaries,
];

/// A definition in the current document or an application-registered asset.
/// IDs are host identities, independent of display names and resource aliases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentLibraryItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub detail: String,
    pub filter: ComponentLibraryFilter,
    pub node: Option<NodeId>,
    pub asset_id: Option<String>,
    pub swatch: Option<String>,
    pub usages: usize,
}

type SelectItem = Rc<dyn Fn(&ComponentLibraryItem)>;
type DragItem = Rc<dyn Fn(&ComponentLibraryItem, &PointerPressedEventArgs)>;

pub struct ComponentLibrary {
    root: Ref<Grid>,
    search: Ref<TextBox>,
    filter: Ref<ComboBox>,
    list: Ref<ListBox>,
    count: Ref<TextBlock>,
    empty: Ref<TextBlock>,
    items: RefCell<Vec<ComponentLibraryItem>>,
    visible: RefCell<Vec<ComponentLibraryItem>>,
    selected: RefCell<Option<String>>,
    select: RefCell<Option<SelectItem>>,
    activate: RefCell<Option<SelectItem>>,
    drag: RefCell<Option<DragItem>>,
    create: RefCell<Option<Rc<dyn Fn()>>>,
    press: RefCell<Option<(ComponentLibraryItem, PointerPressedEventArgs, Point)>>,
    guard: Cell<bool>,
}

impl ComponentLibrary {
    pub fn new() -> Rc<Self> {
        let root = Grid::new();
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto,*,Auto").unwrap());
        root.set_margin(Thickness::new(8.0, 8.0, 8.0, 8.0));
        let header = Grid::new();
        header.set_column_definitions(ColumnDefinitions::parse("*,28").unwrap());
        header.set_margin(Thickness::new(4.0, 0.0, 4.0, 7.0));
        let search = TextBox::new();
        search.set_name(Some("ComponentLibrarySearch".into()));
        search.set_placeholder_text(Some("Search definitions and resources"));
        search.set_margin(Thickness::new(0.0, 0.0, 4.0, 0.0));
        AutomationProperties::set_name(&search, Some("Search component library"));
        header.children().add(search.clone().upcast());
        let create = Button::new();
        create.set_name(Some("NewComponentResource".into()));
        create.classes().add("icon");
        create.set_content(Some(Rc::new("+".to_owned())));
        create.set_height(26.0);
        ToolTip::set_tip(
            &create,
            Some(Rc::new("Create a resource, theme or template".to_owned())),
        );
        AutomationProperties::set_name(&create, Some("Create resource or definition"));
        Grid::set_column(&create, 1);
        header.children().add(create.clone().upcast());
        root.children().add(header.upcast());
        let filter = ComboBox::new();
        filter.set_name(Some("ComponentLibraryFilter".into()));
        filter.set_horizontal_alignment(HorizontalAlignment::Stretch);
        filter.set_margin(Thickness::new(4.0, 0.0, 4.0, 8.0));
        AutomationProperties::set_name(&filter, Some("Definition category"));
        for choice in FILTERS {
            filter.items().add(Some(Rc::new(choice.title().to_owned())));
        }
        filter.set_selected_index(0);
        Grid::set_row(&filter, 1);
        root.children().add(filter.clone().upcast());
        let list = ListBox::new();
        list.set_name(Some("ComponentLibraryList".into()));
        list.classes().add("designer-library-list");
        list.set_background(Some(SolidColorBrush::from_uint32(0x0025262b).into()));
        list.set_border_thickness(Thickness::ZERO);
        list.set_padding(Thickness::ZERO);
        Grid::set_row(&list, 2);
        root.children().add(list.clone().upcast());
        let empty = TextBlock::new();
        empty.set_text(Some(
            "No matching definitions.\nCreate a resource or register a custom control asset.",
        ));
        empty.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        empty.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
        empty.set_font_size(11.0);
        empty.set_margin(Thickness::uniform(12.0));
        empty.set_is_hit_test_visible(false);
        empty.set_vertical_alignment(VerticalAlignment::Top);
        Grid::set_row(&empty, 2);
        root.children().add(empty.clone().upcast());
        let footer = StackPanel::new();
        footer.set_spacing(4.0);
        footer.set_margin(Thickness::new(4.0, 8.0, 4.0, 2.0));
        let count = TextBlock::new();
        count.set_font_size(11.0);
        count.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        footer.children().add(count.clone().upcast());
        let hint = TextBlock::new();
        hint.set_text(Some("Enter to edit · Drag resources to properties"));
        hint.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        hint.set_font_size(10.0);
        hint.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
        footer.children().add(hint.upcast());
        Grid::set_row(&footer, 3);
        root.children().add(footer.upcast());
        let this = Rc::new(Self {
            root,
            search,
            filter,
            list,
            count,
            empty,
            items: RefCell::new(Vec::new()),
            visible: RefCell::new(Vec::new()),
            selected: RefCell::new(None),
            select: RefCell::new(None),
            activate: RefCell::new(None),
            drag: RefCell::new(None),
            create: RefCell::new(None),
            press: RefCell::new(None),
            guard: Cell::new(false),
        });
        let weak = Rc::downgrade(&this);
        this.list
            .set_item_template(Some(FuncDataTemplate::for_type::<ComponentLibraryItem>(
                move |item, _| weak.upgrade().map(|this| this.item_control(item)),
                false,
            )));
        let weak = Rc::downgrade(&this);
        this.search.text_changing(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.refresh();
            }
        });
        let weak = Rc::downgrade(&this);
        this.filter.selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this.guard.get() {
                    this.refresh();
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.list.selection_changed(move |_, _| {
            let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) else {
                return;
            };
            let item = this
                .list
                .selected_item()
                .and_then(|item| item.downcast_ref::<ComponentLibraryItem>().cloned());
            *this.selected.borrow_mut() = item.as_ref().map(|item| item.id.clone());
            if let Some(item) = item {
                if let Some(callback) = this.select.borrow().clone() {
                    callback(&item);
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.list
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if event.key == Key::Enter {
                    if let Some(item) = this.selected_item() {
                        if let Some(callback) = this.activate.borrow().clone() {
                            callback(&item);
                            event.set_handled(true);
                        }
                    }
                }
                if event.key == Key::Escape {
                    this.press.borrow_mut().take();
                }
            });
        let weak = Rc::downgrade(&this);
        create.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if let Some(callback) = this.create.borrow().clone() {
                    callback();
                }
            }
        });
        this.refresh();
        this
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn list(&self) -> Ref<ListBox> {
        self.list.clone()
    }
    pub fn on_select(&self, callback: impl Fn(&ComponentLibraryItem) + 'static) {
        *self.select.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_activate(&self, callback: impl Fn(&ComponentLibraryItem) + 'static) {
        *self.activate.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_begin_drag(
        &self,
        callback: impl Fn(&ComponentLibraryItem, &PointerPressedEventArgs) + 'static,
    ) {
        *self.drag.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_create(&self, callback: impl Fn() + 'static) {
        *self.create.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn set_query(&self, query: &str) {
        self.search.set_text(Some(query));
    }
    pub fn set_filter(&self, filter: ComponentLibraryFilter) {
        self.filter.set_selected_index(
            FILTERS
                .iter()
                .position(|value| *value == filter)
                .unwrap_or(0) as i32,
        );
    }
    pub fn visible_items(&self) -> Vec<ComponentLibraryItem> {
        self.visible.borrow().clone()
    }
    pub fn selected_item(&self) -> Option<ComponentLibraryItem> {
        self.selected.borrow().as_ref().and_then(|id| {
            self.items
                .borrow()
                .iter()
                .find(|item| &item.id == id)
                .cloned()
        })
    }
    pub fn select(&self, id: &str) -> bool {
        let Some(index) = self.visible.borrow().iter().position(|item| item.id == id) else {
            return false;
        };
        self.list.set_selected_index(index as i32);
        self.list.scroll_into_view(index as i32);
        true
    }
    pub fn replace_items(&self, mut items: Vec<ComponentLibraryItem>) {
        items.sort_by(|a, b| {
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then(a.id.cmp(&b.id))
        });
        if *self.items.borrow() == items {
            return;
        }
        self.press.borrow_mut().take();
        *self.items.borrow_mut() = items;
        self.refresh();
    }
    fn refresh(&self) {
        let query = self.search.text().unwrap_or_default().to_lowercase();
        let terms = query.split_whitespace().collect::<Vec<_>>();
        let filter = FILTERS
            .get(self.filter.selected_index().max(0) as usize)
            .copied()
            .unwrap_or_default();
        let items = self
            .items
            .borrow()
            .iter()
            .filter(|item| {
                (filter == ComponentLibraryFilter::All || item.filter == filter)
                    && terms.iter().all(|term| {
                        format!("{} {} {}", item.title, item.kind, item.detail)
                            .to_lowercase()
                            .contains(term)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        let selected = self.selected.borrow().clone();
        self.guard.set(true);
        self.list.items().clear();
        for item in &items {
            self.list.items().add(Some(Rc::new(item.clone())));
        }
        self.list.set_selected_index(
            selected
                .as_ref()
                .and_then(|selected| items.iter().position(|item| &item.id == selected))
                .map_or(-1, |index| index as i32),
        );
        self.guard.set(false);
        self.empty.set_is_visible(items.is_empty());
        self.count.set_text(Some(&format!(
            "{} of {} definitions",
            items.len(),
            self.items.borrow().len()
        )));
        *self.visible.borrow_mut() = items;
    }
    fn item_control(self: &Rc<Self>, item: &ComponentLibraryItem) -> Ref<Control> {
        let row = Grid::new();
        row.set_column_definitions(ColumnDefinitions::parse("22,*,Auto").unwrap());
        row.set_min_height(46.0);
        row.set_margin(Thickness::new(7.0, 3.0, 7.0, 3.0));
        let color = match item.filter {
            ComponentLibraryFilter::Components | ComponentLibraryFilter::Templates => 0xffb6a2f6,
            ComponentLibraryFilter::Themes => 0xff79a8ff,
            _ => 0xff7fcbb4,
        };
        let icon: Ref<Control> = if let Some(swatch) = item
            .swatch
            .as_ref()
            .and_then(|value| ferroui_base::media::Color::parse(value).ok())
        {
            let square = Border::new();
            square.set_width(14.0);
            square.set_height(14.0);
            square.set_corner_radius(CornerRadius::uniform(3.0));
            square.set_background(Some(SolidColorBrush::with_color(swatch).into()));
            square.upcast()
        } else {
            let icon = PathIcon::new();
            icon.set_width(14.0);
            icon.set_height(14.0);
            icon.set_foreground(Some(SolidColorBrush::from_uint32(color).into()));
            icon.set_data(Geometry::parse(match item.filter {
                ComponentLibraryFilter::Components => "M8,0 L16,8 8,16 0,8 Z M8,3 L3,8 8,13 13,8 Z",
                ComponentLibraryFilter::Templates => "M1,1 L15,1 15,15 1,15 Z M3,3 L3,13 13,13 13,3 Z M5,5 L11,5 11,7 5,7 Z M5,9 L9,9 9,11 5,11 Z",
                ComponentLibraryFilter::Themes => "M8,0 L16,8 8,16 0,8 Z M8,3 L3,8 8,13 Z",
                ComponentLibraryFilter::Dictionaries => "M1,2 L7,2 9,4 15,4 15,15 1,15 Z M3,6 L3,13 13,13 13,6 Z",
                _ => "M1,1 L15,1 15,15 1,15 Z M3,3 L3,13 13,13 13,3 Z M5,5 L11,5 11,11 5,11 Z",
            }).unwrap());
            icon.upcast()
        };
        icon.set_vertical_alignment(VerticalAlignment::Center);
        row.children().add(icon);
        let labels = StackPanel::new();
        labels.set_spacing(2.0);
        labels.set_vertical_alignment(VerticalAlignment::Center);
        let title = TextBlock::new();
        title.set_text(Some(&item.title));
        title.set_font_size(12.0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_text_trimming(TextTrimming::CharacterEllipsis);
        labels.children().add(title.upcast());
        let detail = TextBlock::new();
        detail.set_text(Some(&item.kind));
        detail.set_font_size(10.0);
        detail.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
        detail.set_text_trimming(TextTrimming::CharacterEllipsis);
        labels.children().add(detail.upcast());
        Grid::set_column(&labels, 1);
        row.children().add(labels.upcast());
        if item.usages > 0 {
            let count = TextBlock::new();
            count.set_text(Some(&item.usages.to_string()));
            count.set_font_size(10.0);
            count.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
            count.set_margin(Thickness::new(5.0, 0.0, 0.0, 0.0));
            count.set_vertical_alignment(VerticalAlignment::Center);
            Grid::set_column(&count, 2);
            row.children().add(count.upcast());
        }
        let description = format!(
            "{} · {} · {} · {} references",
            item.title, item.kind, item.detail, item.usages
        );
        ToolTip::set_tip(&row, Some(Rc::new(description.clone())));
        AutomationProperties::set_name(&row, Some(&description));
        let weak = Rc::downgrade(self);
        let item = item.clone();
        row.add_handler_with_options(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                if !event.properties().is_left_button_pressed {
                    return;
                }
                if let Some(this) = weak.upgrade() {
                    if event.click_count() == 2 {
                        if let Some(callback) = this.activate.borrow().clone() {
                            callback(&item);
                            event.set_handled(true);
                        }
                    } else {
                        *this.press.borrow_mut() = Some((
                            item.clone(),
                            event.clone(),
                            event.get_position(Some(&this.list)),
                        ));
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        row.add_handler(InputElement::pointer_moved_event(), move |_, event| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let moved = this.press.borrow().as_ref().is_some_and(|(_, _, point)| {
                let current = event.get_position(Some(&this.list));
                (current.x - point.x).abs() + (current.y - point.y).abs() > 6.0
            });
            if moved && event.properties().is_left_button_pressed {
                let press = this.press.borrow_mut().take();
                if let Some((item, press, _)) = press {
                    if let Some(callback) = this.drag.borrow().clone() {
                        callback(&item, &press);
                        event.set_handled(true);
                    }
                }
            }
        });
        let weak = Rc::downgrade(self);
        row.add_handler(InputElement::pointer_released_event(), move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.press.borrow_mut().take();
            }
        });
        row.upcast()
    }
}

#[cfg(test)]
#[path = "component_browser_tests.rs"]
mod tests;

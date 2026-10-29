//! Native overview of named journeys and their shared source-backed screen graph.
use crate::{PreviewPrototype, PreviewPrototypeAction, PreviewStageId, PreviewStageInfo};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
use ferroui_base::media::{DrawingContext, FontWeight, IPen, Pen, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Border, Button, Canvas, CheckBox, ComboBox, Control, ControlImpl, Grid, RowDefinitions,
    ScrollViewer, StackPanel, TextBlock, TextBox, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewPrototypeViewAction {
    Back,
    NewFlow,
    EditFlow { index: usize },
    RemoveFlow { index: usize },
    Play { flow: usize },
    AddLink { source: Option<PreviewStageId> },
    EditLink { index: usize },
    RemoveLink { index: usize },
    FocusStage { stage: PreviewStageId },
}

type Action = Rc<dyn Fn(PreviewPrototypeViewAction, u64, Ref<Control>) -> Result<(), String>>;

pub struct PreviewPrototypeView {
    root: Ref<Grid>,
    flow: Ref<ComboBox>,
    reachable: Ref<CheckBox>,
    search: Ref<TextBox>,
    summary: Ref<TextBlock>,
    selection: Ref<TextBlock>,
    error: Ref<TextBlock>,
    map: Ref<Canvas>,
    map_scroll: Ref<ScrollViewer>,
    links: Ref<StackPanel>,
    flow_commands: Vec<Ref<Button>>,
    prototype: RefCell<PreviewPrototype>,
    stages: RefCell<Vec<PreviewStageInfo>>,
    selected: Cell<Option<PreviewStageId>>,
    epoch: Cell<u64>,
    updating: Cell<bool>,
    retired: RefCell<Rc<Cell<bool>>>,
    action: RefCell<Option<Action>>,
}

impl PreviewPrototypeView {
    pub fn new(_palette: &Control) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewPrototypeView".into()));
        root.classes().add("designer-chrome");
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto,*").unwrap());
        let header = StackPanel::new();
        header.set_spacing(8.0);
        header.set_margin(Thickness::new(16.0, 14.0, 16.0, 10.0));
        let title = text("Prototype flows");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        header.children().add(title.upcast::<Control>());
        let summary = text("Connect authored elements across your native preview stages.");
        summary.set_name(Some("PrototypeSummary".into()));
        summary.classes().add("designer-muted");
        header.children().add(summary.clone().upcast::<Control>());
        let toolbar = WrapPanel::new();
        toolbar.set_item_spacing(7.0);
        toolbar.set_line_spacing(6.0);
        let flow = ComboBox::new();
        flow.set_name(Some("PrototypeFlowChoice".into()));
        flow.set_width(230.0);
        AutomationProperties::set_name(&flow, Some("Prototype flow"));
        flow.items().add(Some(Rc::new("All screens".to_owned())));
        flow.set_selected_index(0);
        toolbar.children().add(flow.clone().upcast::<Control>());
        let new_flow = button("New flow…", "PrototypeNewFlow");
        let edit_flow = button("Edit flow…", "PrototypeEditFlow");
        let remove_flow = button("Remove flow", "PrototypeRemoveFlow");
        let play = button("Play", "PrototypePlay");
        play.classes().add("primary");
        let add_link = button("Add interaction…", "PrototypeAddLink");
        let back = button("Back to stages", "PrototypeBack");
        for control in [&new_flow, &edit_flow, &remove_flow, &play, &add_link, &back] {
            toolbar.children().add(control.clone().upcast::<Control>());
        }
        header.children().add(toolbar.upcast::<Control>());
        let filters = WrapPanel::new();
        filters.set_item_spacing(10.0);
        filters.set_line_spacing(6.0);
        let search = TextBox::new();
        search.set_name(Some("PrototypeSearch".into()));
        search.set_width(290.0);
        search.set_placeholder_text(Some("Search screens, source files and interactions"));
        AutomationProperties::set_name(&search, Some("Search prototype screens and interactions"));
        filters.children().add(search.clone().upcast::<Control>());
        let reachable = CheckBox::new();
        reachable.set_name(Some("PrototypeReachable".into()));
        reachable.set_content(Some(Rc::new(
            "Only screens reachable from this flow".to_owned(),
        )));
        reachable.set_is_checked(Some(true));
        AutomationProperties::set_name(&reachable, Some("Only reachable prototype screens"));
        filters
            .children()
            .add(reachable.clone().upcast::<Control>());
        header.children().add(filters.upcast::<Control>());
        root.children().add(header.upcast::<Control>());
        let error = text("");
        error.set_name(Some("PrototypeError".into()));
        error.classes().add("designer-error");
        error.set_is_visible(false);
        error.set_margin(Thickness::new(16.0, 0.0, 16.0, 8.0));
        Grid::set_row(&error, 1);
        root.children().add(error.clone().upcast::<Control>());
        let body = StackPanel::new();
        body.set_spacing(10.0);
        body.set_margin(Thickness::new(16.0, 0.0, 16.0, 16.0));
        let map = Canvas::new();
        map.set_name(Some("PrototypeMap".into()));
        let map_scroll = ScrollViewer::new();
        map_scroll.set_name(Some("PrototypeMapScroll".into()));
        map_scroll.set_height(360.0);
        map_scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        map_scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        map_scroll.set_bring_into_view_on_focus_change(true);
        map_scroll.set_content(Some(Control::boxed(map.clone().upcast::<Control>())));
        body.children().add(map_scroll.clone().upcast::<Control>());
        let selection = text("All interactions");
        selection.set_name(Some("PrototypeSelection".into()));
        selection.set_font_weight(FontWeight::SemiBold);
        body.children().add(selection.clone().upcast::<Control>());
        let clear = button("Show all interactions", "PrototypeClearSelection");
        clear.set_horizontal_alignment(HorizontalAlignment::Left);
        body.children().add(clear.clone().upcast::<Control>());
        let links = StackPanel::new();
        links.set_name(Some("PrototypeLinkList".into()));
        links.set_spacing(7.0);
        body.children().add(links.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_name(Some("PrototypeOverviewScroll".into()));
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(body.upcast::<Control>())));
        Grid::set_row(&scroll, 2);
        root.children().add(scroll.upcast::<Control>());
        let view = Rc::new(Self {
            root,
            flow,
            reachable,
            search,
            summary,
            selection,
            error,
            map,
            map_scroll,
            links,
            flow_commands: vec![edit_flow.clone(), remove_flow.clone(), play.clone()],
            prototype: RefCell::new(PreviewPrototype::default()),
            stages: RefCell::new(Vec::new()),
            selected: Cell::new(None),
            epoch: Cell::new(0),
            updating: Cell::new(false),
            retired: RefCell::new(Rc::new(Cell::new(false))),
            action: RefCell::new(None),
        });
        view.toolbar(&new_flow, |_| Some(PreviewPrototypeViewAction::NewFlow));
        view.toolbar(&edit_flow, |view| {
            view.selected_flow()
                .map(|index| PreviewPrototypeViewAction::EditFlow { index })
        });
        view.toolbar(&remove_flow, |view| {
            view.selected_flow()
                .map(|index| PreviewPrototypeViewAction::RemoveFlow { index })
        });
        view.toolbar(&play, |view| {
            view.selected_flow()
                .map(|flow| PreviewPrototypeViewAction::Play { flow })
        });
        view.toolbar(&add_link, |view| {
            Some(PreviewPrototypeViewAction::AddLink {
                source: view.selected.get(),
            })
        });
        view.toolbar(&back, |_| Some(PreviewPrototypeViewAction::Back));
        let weak = Rc::downgrade(&view);
        clear.click(move |_, event| {
            if let Some(view) = weak.upgrade() {
                view.selected.set(None);
                view.refresh();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&view);
        view.flow.selection_changed(move |_, _| {
            if let Some(view) = weak.upgrade().filter(|view| !view.updating.get()) {
                view.selected.set(None);
                view.refresh();
            }
        });
        let weak = Rc::downgrade(&view);
        view.reachable.is_checked_changed(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.refresh();
            }
        });
        let weak = Rc::downgrade(&view);
        view.search.text_changing(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.refresh();
            }
        });
        // Native controls receive keys first. Unhandled panel commands must not
        // bubble into the surrounding workspace's XAML authoring shortcuts.
        view.root.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| event.set_handled(true),
            RoutingStrategies::BUBBLE,
            false,
        );
        view.refresh();
        view
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn on_action(
        &self,
        callback: impl Fn(PreviewPrototypeViewAction, u64, Ref<Control>) -> Result<(), String> + 'static,
    ) {
        *self.action.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
        self.error.set_is_visible(!message.is_empty());
    }
    pub fn selected_flow(&self) -> Option<usize> {
        usize::try_from(self.flow.selected_index() - 1)
            .ok()
            .filter(|index| *index < self.prototype.borrow().flows.len())
    }
    pub fn set_query(&self, query: &str) {
        self.search.set_text(Some(query));
    }
    pub fn select_flow(&self, index: Option<usize>) {
        self.flow
            .set_selected_index(index.map_or(0, |index| index.saturating_add(1) as i32));
    }

    fn toolbar(
        self: &Rc<Self>,
        button: &Ref<Button>,
        action: fn(&Self) -> Option<PreviewPrototypeViewAction>,
    ) {
        let weak = Rc::downgrade(self);
        let anchor = button.downgrade();
        button.click(move |_, event| {
            if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                if let Some(action) = action(&view) {
                    view.emit(action, view.epoch.get(), anchor.upcast());
                }
            }
            event.set_handled(true);
        });
    }

    fn emit(&self, action: PreviewPrototypeViewAction, epoch: u64, anchor: Ref<Control>) {
        let result = self
            .action
            .borrow()
            .clone()
            .ok_or_else(|| "No prototype overview host is attached".to_owned())
            .and_then(|callback| callback(action, epoch, anchor));
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
    }

    pub fn update(
        self: &Rc<Self>,
        prototype: &PreviewPrototype,
        stages: &[PreviewStageInfo],
        epoch: u64,
    ) {
        // A workspace refresh often changes neither graph nor preview metadata.
        // Retain native map/list focus when there is nothing new to present.
        if self.epoch.get() == epoch
            && *self.prototype.borrow() == *prototype
            && self
                .stages
                .borrow()
                .iter()
                .map(|stage| {
                    (
                        stage.id,
                        stage.epoch,
                        &stage.configuration.name,
                        &stage.configuration.source_uri,
                    )
                })
                .eq(stages.iter().map(|stage| {
                    (
                        stage.id,
                        stage.epoch,
                        &stage.configuration.name,
                        &stage.configuration.source_uri,
                    )
                }))
        {
            return;
        }
        let selected = self.selected_flow().and_then(|index| {
            self.prototype
                .borrow()
                .flows
                .get(index)
                .map(|flow| flow.name.clone())
        });
        let first_flow = self.prototype.borrow().flows.is_empty() && !prototype.flows.is_empty();
        self.updating.set(true);
        *self.prototype.borrow_mut() = prototype.clone();
        *self.stages.borrow_mut() = stages.to_vec();
        self.epoch.set(epoch);
        self.flow.items().clear();
        self.flow
            .items()
            .add(Some(Rc::new("All screens".to_owned())));
        for flow in &prototype.flows {
            self.flow.items().add(Some(Rc::new(flow.name.clone())));
        }
        self.flow.set_selected_index(
            selected
                .and_then(|name| prototype.flows.iter().position(|flow| flow.name == name))
                .map_or(i32::from(first_flow), |index| index as i32 + 1),
        );
        if !stages
            .iter()
            .any(|stage| Some(stage.id) == self.selected.get())
        {
            self.selected.set(None);
        }
        self.updating.set(false);
        self.refresh();
    }

    fn refresh(self: &Rc<Self>) {
        if self.updating.get() {
            return;
        }
        let restore_map_focus = self.map.is_keyboard_focus_within();
        let mut selected_control = None;
        self.retired.borrow().set(true);
        *self.retired.borrow_mut() = Rc::new(Cell::new(false));
        let selected_flow = self.selected_flow();
        for control in &self.flow_commands {
            control.set_is_enabled(selected_flow.is_some());
        }
        self.reachable.set_is_enabled(selected_flow.is_some());
        let prototype = self.prototype.borrow().clone();
        let stages = self.stages.borrow().clone();
        let start = selected_flow
            .and_then(|index| prototype.flows.get(index))
            .map(|flow| flow.start);
        let reachable = start.map(|start| prototype.reachable(start));
        let only_reachable = self.reachable.is_checked() == Some(true);
        let query = self.search.text().unwrap_or_default().to_lowercase();
        let terms = query.split_whitespace().collect::<Vec<_>>();
        let in_scope = |id: PreviewStageId| {
            !only_reachable || reachable.as_ref().is_none_or(|set| set.contains(&id))
        };
        let filtered = stages
            .iter()
            .filter(|stage| {
                in_scope(stage.id)
                    && matches_query(
                        &terms,
                        &format!(
                            "{} {}",
                            stage.configuration.name, stage.configuration.source_uri
                        ),
                    )
            })
            .collect::<Vec<_>>();
        let visible = filtered
            .iter()
            .map(|stage| stage.id)
            .collect::<BTreeSet<_>>();
        if self.selected.get().is_some_and(|id| !visible.contains(&id)) {
            self.selected.set(None);
        }
        let depths = depths(&prototype, start);
        let mut columns = BTreeMap::<usize, usize>::new();
        let mut positions = BTreeMap::new();
        for (index, stage) in filtered.iter().enumerate() {
            let column = if start.is_some() {
                depths.get(&stage.id).copied().unwrap_or(3).min(3)
            } else {
                index % 3
            };
            let row = columns.entry(column).or_default();
            positions.insert(
                stage.id,
                Point::new(20.0 + column as f64 * 280.0, 20.0 + *row as f64 * 192.0),
            );
            *row += 1;
        }
        let width = positions
            .values()
            .map(|point| point.x + 250.0)
            .fold(300.0, f64::max);
        let height = positions
            .values()
            .map(|point| point.y + 190.0)
            .fold(90.0, f64::max);
        self.map.children().clear();
        self.map.set_width(width);
        self.map.set_height(height);
        self.map_scroll.set_height(height.clamp(210.0, 440.0));
        let routes = prototype
            .links
            .iter()
            .filter_map(|link| {
                let PreviewPrototypeAction::Navigate { stage } = link.action else {
                    return None;
                };
                let from = positions.get(&link.source)?;
                let to = positions.get(&stage)?;
                Some(route(*from, *to))
            })
            .collect::<Vec<_>>();
        let connectors = PrototypeConnectors::new(routes);
        connectors.set_width(width);
        connectors.set_height(height);
        self.map.children().add(connectors.upcast::<Control>());
        for stage in &filtered {
            let count = prototype
                .links
                .iter()
                .filter(|link| link.source == stage.id)
                .count();
            let has_back = prototype.links.iter().any(|link| {
                link.source == stage.id && matches!(link.action, PreviewPrototypeAction::Back)
            });
            let card = Border::new();
            card.set_name(Some(format!("PrototypeScreen{}", stage.id)));
            card.classes().add("designer-stage-card");
            card.classes().set(
                "active",
                self.selected.get() == Some(stage.id) || start == Some(stage.id),
            );
            card.set_width(220.0);
            card.set_height(144.0);
            card.set_padding(Thickness::uniform(10.0));
            if reachable
                .as_ref()
                .is_some_and(|set| !set.contains(&stage.id))
            {
                card.set_opacity(0.65);
            }
            let content = StackPanel::new();
            content.set_spacing(6.0);
            let select = button(
                &stage.configuration.name,
                &format!("PrototypeSelect{}", stage.id),
            );
            if self.selected.get() == Some(stage.id) {
                selected_control = Some(select.clone());
            }
            select.classes().add("nav");
            select.set_horizontal_alignment(HorizontalAlignment::Stretch);
            let caption = text(&stage.configuration.name);
            caption.set_font_weight(FontWeight::SemiBold);
            caption.set_max_height(38.0);
            select.set_content(Some(Control::boxed(caption)));
            content.children().add(select.clone().upcast::<Control>());
            let file = text(
                stage
                    .configuration
                    .source_uri
                    .rsplit('/')
                    .next()
                    .unwrap_or(&stage.configuration.source_uri),
            );
            file.classes().add("designer-muted");
            file.set_max_height(18.0);
            file.set_font_size(11.0);
            content.children().add(file.upcast::<Control>());
            let detail = text(&format!(
                "{}{} interaction{}{}",
                if start == Some(stage.id) {
                    "Start · "
                } else {
                    ""
                },
                count,
                if count == 1 { "" } else { "s" },
                if has_back { " · Back" } else { "" }
            ));
            detail.set_font_size(11.0);
            content.children().add(detail.upcast::<Control>());
            let commands = WrapPanel::new();
            commands.set_item_spacing(5.0);
            let open = button("Open", &format!("PrototypeOpen{}", stage.id));
            let add = button("Add interaction", &format!("PrototypeAdd{}", stage.id));
            self.connect_row(
                &open,
                PreviewPrototypeViewAction::FocusStage { stage: stage.id },
            );
            self.connect_row(
                &add,
                PreviewPrototypeViewAction::AddLink {
                    source: Some(stage.id),
                },
            );
            commands.children().add(open.upcast::<Control>());
            commands.children().add(add.upcast::<Control>());
            content.children().add(commands.upcast::<Control>());
            card.set_child(Some(content.upcast::<Control>()));
            let point = positions[&stage.id];
            Canvas::set_left(&card, point.x);
            Canvas::set_top(&card, point.y);
            self.map.children().add(card.upcast::<Control>());
            let weak = Rc::downgrade(self);
            let retired = self.retired.borrow().clone();
            let id = stage.id;
            select.click(move |_, event| {
                if !retired.get() {
                    if let Some(view) = weak.upgrade() {
                        view.selected.set(Some(id));
                        view.refresh();
                    }
                }
                event.set_handled(true);
            });
        }
        if filtered.is_empty() {
            let empty = text(if stages.is_empty() {
                "Add preview stages to begin a prototype."
            } else {
                "No screens match the selected flow and search."
            });
            empty.set_width(270.0);
            Canvas::set_left(&empty, 15.0);
            Canvas::set_top(&empty, 15.0);
            self.map.children().add(empty.upcast::<Control>());
        }
        let stage_name = |id: PreviewStageId| {
            stages
                .iter()
                .find(|stage| stage.id == id)
                .map_or("Unavailable screen", |stage| {
                    stage.configuration.name.as_str()
                })
        };
        let stage_uri = |id: PreviewStageId| {
            stages
                .iter()
                .find(|stage| stage.id == id)
                .map_or("", |stage| stage.configuration.source_uri.as_str())
        };
        self.selection
            .set_text(Some(&self.selected.get().map_or_else(
                || "All interactions".to_owned(),
                |id| format!("Interactions from {}", stage_name(id)),
            )));
        self.links.children().clear();
        let mut link_count = 0;
        for (index, link) in prototype.links.iter().enumerate() {
            let target = match link.action {
                PreviewPrototypeAction::Navigate { stage } => stage_name(stage),
                PreviewPrototypeAction::Back => "Back in journey",
            };
            let target_uri = match link.action {
                PreviewPrototypeAction::Navigate { stage } => stage_uri(stage),
                PreviewPrototypeAction::Back => "",
            };
            if !in_scope(link.source)
                || self.selected.get().is_some_and(|id| id != link.source)
                || !matches_query(
                    &terms,
                    &format!(
                        "{} {} {} {} {}",
                        link.name,
                        stage_name(link.source),
                        target,
                        stage_uri(link.source),
                        target_uri
                    ),
                )
            {
                continue;
            }
            let row = Border::new();
            row.set_name(Some(format!("PrototypeLink{index}")));
            row.classes().add("designer-stage-card");
            row.set_padding(Thickness::uniform(10.0));
            let content = StackPanel::new();
            content.set_spacing(5.0);
            let label = text(&link.name);
            label.set_font_weight(FontWeight::SemiBold);
            content.children().add(label.upcast::<Control>());
            let description = text(&format!("{} → {target}", stage_name(link.source)));
            description.classes().add("designer-muted");
            content.children().add(description.upcast::<Control>());
            let commands = WrapPanel::new();
            commands.set_item_spacing(6.0);
            let edit = button("Edit interaction…", &format!("PrototypeEditLink{index}"));
            let remove = button("Remove", &format!("PrototypeRemoveLink{index}"));
            self.connect_row(&edit, PreviewPrototypeViewAction::EditLink { index });
            self.connect_row(&remove, PreviewPrototypeViewAction::RemoveLink { index });
            commands.children().add(edit.upcast::<Control>());
            commands.children().add(remove.upcast::<Control>());
            content.children().add(commands.upcast::<Control>());
            row.set_child(Some(content.upcast::<Control>()));
            self.links.children().add(row.upcast::<Control>());
            link_count += 1;
        }
        if link_count == 0 {
            self.links.children().add(text("No interactions match this selection. Select a screen and add an interaction to connect its authored controls.").upcast::<Control>());
        }
        let unreachable = reachable.as_ref().map_or(0, |set| {
            stages
                .iter()
                .filter(|stage| !set.contains(&stage.id))
                .count()
        });
        self.summary.set_text(Some(&format!(
            "{} screens shown · {link_count} interactions listed · {} flows{}",
            visible.len(),
            prototype.flows.len(),
            if start.is_some() {
                format!(" · {unreachable} screens outside this journey")
            } else {
                String::new()
            }
        )));
        if restore_map_focus {
            if let Some(control) = selected_control {
                control.focus();
            }
        }
    }

    fn connect_row(self: &Rc<Self>, control: &Ref<Button>, action: PreviewPrototypeViewAction) {
        let weak = Rc::downgrade(self);
        let anchor = control.downgrade();
        let retired = self.retired.borrow().clone();
        let epoch = self.epoch.get();
        control.click(move |_, event| {
            if !retired.get() {
                if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    view.emit(action.clone(), epoch, anchor.upcast());
                }
            }
            event.set_handled(true);
        });
    }
}

impl Drop for PreviewPrototypeView {
    fn drop(&mut self) {
        self.retired.get_mut().set(true);
    }
}

fn matches_query(terms: &[&str], value: &str) -> bool {
    let value = value.to_lowercase();
    terms.iter().all(|term| value.contains(term))
}
fn depths(
    prototype: &PreviewPrototype,
    start: Option<PreviewStageId>,
) -> BTreeMap<PreviewStageId, usize> {
    let Some(start) = start else {
        return BTreeMap::new();
    };
    let mut depth = BTreeMap::from([(start, 0)]);
    let mut pending = VecDeque::from([start]);
    while let Some(source) = pending.pop_front() {
        let next_depth = depth[&source] + 1;
        for link in prototype.links.iter().filter(|link| link.source == source) {
            if let PreviewPrototypeAction::Navigate { stage } = link.action {
                if let std::collections::btree_map::Entry::Vacant(entry) = depth.entry(stage) {
                    entry.insert(next_depth);
                    pending.push_back(stage);
                }
            }
        }
    }
    depth
}
fn route(from: Point, to: Point) -> Vec<Point> {
    if to.x == from.x && to.y > from.y && to.y - from.y <= 192.0 {
        // Adjacent screens in one column connect through the clear vertical gap.
        vec![
            Point::new(from.x + 110.0, from.y + 144.0),
            Point::new(to.x + 110.0, to.y),
        ]
    } else if to.x > from.x {
        let from = Point::new(from.x + 220.0, from.y + 72.0);
        let to = Point::new(to.x, to.y + 72.0);
        let middle = (from.x + to.x) / 2.0;
        vec![
            from,
            Point::new(middle, from.y),
            Point::new(middle, to.y),
            to,
        ]
    } else {
        let from = Point::new(from.x + 165.0, from.y + 144.0);
        let to = Point::new(to.x + 55.0, to.y + 144.0);
        let bottom = from.y.max(to.y) + 28.0;
        vec![
            from,
            Point::new(from.x, bottom),
            Point::new(to.x, bottom),
            to,
        ]
    }
}

#[repr(C)]
struct PrototypeConnectors {
    base: Control,
    routes: Vec<Vec<Point>>,
}
ferro_class!(PrototypeConnectors: Control);
ferro_class_info!(PrototypeConnectors {
    new: PrototypeConnectors::empty
});
ferro_impl_classes!(PrototypeConnectors: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
impl PrototypeConnectors {
    fn empty() -> Ref<Self> {
        Self::new(Vec::new())
    }
    fn new(routes: Vec<Vec<Point>>) -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            routes,
        });
        this.set_name(Some("PrototypeConnections".into()));
        this.set_is_hit_test_visible(false);
        this
    }
}
impl VisualImpl for PrototypeConnectors {
    fn render(this: &Self, context: &mut DrawingContext) {
        let pen: Rc<dyn IPen> = Pen::from_uint32(0xff7099d5, 1.5).into();
        for route in &this.routes {
            for pair in route.windows(2) {
                context.draw_line(&pen, pair[0], pair[1]);
            }
            let Some((tip, rest)) = route.split_last() else {
                continue;
            };
            let Some(previous) = rest.last() else {
                continue;
            };
            let dx = tip.x - previous.x;
            let dy = tip.y - previous.y;
            let length = dx.hypot(dy);
            if length <= 0.0 {
                continue;
            }
            let (ux, uy) = (dx / length, dy / length);
            for side in [-1.0, 1.0] {
                context.draw_line(
                    &pen,
                    *tip,
                    Point::new(
                        tip.x - ux * 7.0 + uy * 4.0 * side,
                        tip.y - uy * 7.0 - ux * 4.0 * side,
                    ),
                );
            }
        }
    }
}
fn text(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn button(value: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(value.to_owned())));
    AutomationProperties::set_name(&button, Some(value));
    button
}

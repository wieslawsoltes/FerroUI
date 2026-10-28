//! Native responsive stage cards. Editor chrome is confined to headers and toolbars;
//! application preview roots retain their own styles, themes and input behavior.
use crate::{PreviewStageId, PreviewStageInfo, PreviewStageMode, PreviewStageStatus, PreviewTheme};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, Stretch, StretchDirection, TextWrapping};
use ferroui_base::{Point, Ref, Thickness, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, Control, Grid, MenuFlyout, MenuItem, RowDefinitions,
    ScrollViewer, StackPanel, TextBlock, ToolTip, Viewbox, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStageComparison {
    Sizes,
    Themes,
    Orientations,
    Directions,
}

/// Commands retain the card's accepted epoch. Hosts reject callbacks from a replaced
/// graph or dismissed menu rather than applying them to a different preview scenario.
#[derive(Clone, Debug)]
pub enum PreviewStageAction {
    Add,
    Compare(PreviewStageComparison),
    RefreshAll,
    Save,
    Load,
    Back,
    Edit {
        stage: PreviewStageId,
        epoch: u64,
    },
    Duplicate {
        stage: PreviewStageId,
        epoch: u64,
    },
    Remove {
        stage: PreviewStageId,
        epoch: u64,
    },
    Move {
        stage: PreviewStageId,
        epoch: u64,
        offset: i32,
    },
    Pin {
        stage: PreviewStageId,
        epoch: u64,
        pinned: bool,
    },
    Mode {
        stage: PreviewStageId,
        epoch: u64,
        mode: PreviewStageMode,
    },
    Pause {
        stage: PreviewStageId,
        epoch: u64,
        paused: bool,
    },
    Refresh {
        stage: PreviewStageId,
        epoch: u64,
    },
    Focus {
        stage: PreviewStageId,
        epoch: u64,
    },
    Select {
        stage: PreviewStageId,
        epoch: u64,
        position: Point,
    },
}

type ActionHandler = Rc<dyn Fn(PreviewStageAction, Ref<Control>) -> Result<(), String>>;

struct StageCard {
    root: Ref<Border>,
    viewbox: Ref<Viewbox>,
    host: Ref<Control>,
    epoch: u64,
    retired: Rc<Cell<bool>>,
    options: Ref<MenuFlyout>,
}

impl StageCard {
    fn retire(&self) {
        self.retired.set(true);
        self.options.hide();
        self.viewbox.set_child(None::<Ref<Control>>);
    }
}

/// The view never owns document or runtime state. A host supplies stable native stage
/// controls and immutable descriptions, then handles explicit guarded commands.
pub struct PreviewStageView {
    root: Ref<Grid>,
    cards: RefCell<BTreeMap<PreviewStageId, StageCard>>,
    panel: Ref<WrapPanel>,
    empty: Ref<TextBlock>,
    summary: Ref<TextBlock>,
    error: Ref<TextBlock>,
    palette: WeakRef<Control>,
    action: RefCell<Option<ActionHandler>>,
}

impl PreviewStageView {
    pub fn new(palette: &Control) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewStageBoard".into()));
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto,*").unwrap());
        let heading = StackPanel::new();
        heading.classes().add("designer-chrome");
        heading.set_margin(Thickness::new(16.0, 14.0, 16.0, 10.0));
        heading.set_spacing(7.0);
        let title = text("Preview stages");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        heading.children().add(title.upcast::<Control>());
        let summary = text("Compare native views at desktop, tablet and phone layout sizes.");
        summary.classes().add("designer-muted");
        heading.children().add(summary.clone().upcast::<Control>());
        let commands = WrapPanel::new();
        commands.set_item_spacing(5.0);
        commands.set_line_spacing(5.0);
        heading.children().add(commands.clone().upcast::<Control>());
        root.children().add(heading.upcast::<Control>());
        let error = text("");
        error.classes().add("designer-error");
        error.set_margin(Thickness::new(16.0, 0.0, 16.0, 8.0));
        error.set_is_visible(false);
        Grid::set_row(&error, 1);
        root.children().add(error.clone().upcast::<Control>());
        let panel = WrapPanel::new();
        panel.set_name(Some("PreviewStageCards".into()));
        panel.set_item_spacing(16.0);
        panel.set_line_spacing(16.0);
        panel.set_item_width(360.0);
        panel.set_margin(Thickness::uniform(16.0));
        panel.set_vertical_alignment(VerticalAlignment::Top);
        let scroll = ScrollViewer::new();
        scroll.set_name(Some("PreviewStageScroll".into()));
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_bring_into_view_on_focus_change(false);
        scroll.set_content(Some(Control::boxed(panel.clone().upcast::<Control>())));
        Grid::set_row(&scroll, 2);
        root.children().add(scroll.clone().upcast::<Control>());
        let empty = text("No stages are visible for this document. Add a stage, or pin a stage to keep it visible across document tabs.");
        empty.classes().add("designer-chrome");
        empty.set_margin(Thickness::uniform(24.0));
        empty.set_vertical_alignment(VerticalAlignment::Top);
        Grid::set_row(&empty, 2);
        root.children().add(empty.clone().upcast::<Control>());
        let view = Rc::new(Self {
            root,
            cards: RefCell::new(BTreeMap::new()),
            panel,
            empty,
            summary,
            error,
            palette: palette.to_ref().downgrade(),
            action: RefCell::new(None),
        });
        for (label, name, action) in [
            ("Add stage", "StageAdd", PreviewStageAction::Add),
            (
                "Refresh all",
                "StageRefreshAll",
                PreviewStageAction::RefreshAll,
            ),
            ("Save set", "StageSave", PreviewStageAction::Save),
            ("Load set", "StageLoad", PreviewStageAction::Load),
            ("Back to canvas", "StageBack", PreviewStageAction::Back),
        ] {
            let button = button(label, name);
            view.connect_button(&button, action, None);
            commands.children().add(button.upcast::<Control>());
        }
        let compare = button("Compare variants", "StageCompare");
        let menu = MenuFlyout::new();
        for (label, kind) in [
            ("Phone, tablet and desktop", PreviewStageComparison::Sizes),
            ("Light and dark themes", PreviewStageComparison::Themes),
            (
                "Portrait and landscape",
                PreviewStageComparison::Orientations,
            ),
            (
                "Left-to-right and right-to-left",
                PreviewStageComparison::Directions,
            ),
        ] {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(label.to_owned())));
            let weak = Rc::downgrade(&view);
            let anchor = compare.downgrade();
            item.click(move |_, event| {
                if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    view.emit(PreviewStageAction::Compare(kind), anchor.upcast());
                }
                event.set_handled(true);
            });
            menu.items().add(Some(Control::boxed(item)));
        }
        compare.set_flyout(Some(menu.upcast()));
        crate::chrome::apply_designer_menu_resources(&compare, palette);
        commands.children().insert(1, compare.upcast::<Control>());
        let weak = Rc::downgrade(&view);
        scroll.size_changed(move |_, event| {
            if let Some(view) = weak.upgrade() {
                view.resize_cards(event.new_size().width);
            }
        });
        view
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn on_action(
        &self,
        callback: impl Fn(PreviewStageAction, Ref<Control>) -> Result<(), String> + 'static,
    ) {
        *self.action.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
        self.error.set_is_visible(!message.is_empty());
    }

    fn emit(&self, action: PreviewStageAction, anchor: Ref<Control>) {
        let callback = self.action.borrow().clone();
        let result = callback
            .ok_or_else(|| "No stage board host is attached".to_owned())
            .and_then(|callback| callback(action, anchor));
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
    }

    fn connect_button(
        self: &Rc<Self>,
        button: &Ref<Button>,
        action: PreviewStageAction,
        retired: Option<Rc<Cell<bool>>>,
    ) {
        let weak = Rc::downgrade(self);
        let anchor = button.downgrade();
        button.click(move |_, event| {
            if retired.as_ref().is_some_and(|retired| retired.get()) {
                event.set_handled(true);
                return;
            }
            if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                view.emit(action.clone(), anchor.upcast());
            }
            event.set_handled(true);
        });
    }

    /// Reconcile cards by stable identity and graph epoch. Unchanged live roots stay
    /// attached, preserving input state, focus and scroll while another card updates.
    pub fn update(
        self: &Rc<Self>,
        stages: &[PreviewStageInfo],
        hosts: &BTreeMap<PreviewStageId, Ref<Control>>,
        active: Option<PreviewStageId>,
        current_uri: &str,
        total: usize,
    ) {
        let desired = stages.iter().map(|info| info.id).collect::<BTreeSet<_>>();
        let mut cards = self.cards.borrow_mut();
        let removed = cards
            .keys()
            .copied()
            .filter(|id| !desired.contains(id))
            .collect::<Vec<_>>();
        for id in removed {
            if let Some(card) = cards.remove(&id) {
                card.retire();
                self.panel.children().remove(&card.root);
            }
        }
        for (index, info) in stages.iter().enumerate() {
            let Some(host) = hosts.get(&info.id) else {
                continue;
            };
            let replace = cards
                .get(&info.id)
                .is_none_or(|card| card.epoch != info.epoch || !std::ptr::eq(&*card.host, &**host));
            if replace {
                if let Some(card) = cards.remove(&info.id) {
                    card.retire();
                    self.panel.children().remove(&card.root);
                }
                cards.insert(info.id, self.card(info, host.clone()));
            }
            let card = &cards[&info.id];
            card.root.classes().set("active", active == Some(info.id));
            let control = card.root.clone().upcast::<Control>();
            if let Some(existing) = self.panel.children().index_of(&control) {
                if existing != index {
                    self.panel.children().move_item(existing, index);
                }
            } else {
                self.panel.children().insert(index, control);
            }
        }
        self.empty.set_is_visible(stages.is_empty());
        self.summary.set_text(Some(&format!(
            "{} · {} visible of {total} stages · sizes use logical units",
            short_uri(current_uri),
            stages.len()
        )));
    }

    fn resize_cards(&self, width: f64) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        // Account for outer gutters and an available scrollbar. Wrap instead of
        // allowing native content or card commands to expand the viewport sideways.
        let available = (width - 44.0).max(1.0);
        let columns = ((available + 16.0) / 356.0).floor().max(1.0);
        self.panel
            .set_item_width(((available - 16.0 * (columns - 1.0)) / columns).clamp(1.0, 460.0));
    }

    fn card(self: &Rc<Self>, info: &PreviewStageInfo, host: Ref<Control>) -> StageCard {
        let root = Border::new();
        root.set_name(Some(format!("PreviewStage{}", info.id)));
        root.classes().add("designer-stage-card");
        root.set_horizontal_alignment(HorizontalAlignment::Stretch);
        root.set_vertical_alignment(VerticalAlignment::Top);
        let body = Grid::new();
        body.set_row_definitions(RowDefinitions::parse("Auto,Auto,Auto").unwrap());
        root.set_child(Some(body.clone().upcast::<Control>()));
        let heading = StackPanel::new();
        heading.classes().add("designer-chrome");
        heading.set_margin(Thickness::uniform(12.0));
        heading.set_spacing(7.0);
        let title_row = Grid::new();
        title_row.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
        let title = text(&info.configuration.name);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_font_size(13.0);
        title_row.children().add(title.upcast::<Control>());
        let options_button = button("⋯", &format!("Stage{}Options", info.id));
        options_button.classes().add("nav");
        AutomationProperties::set_name(
            &options_button,
            Some(&format!("{} stage options", info.configuration.name)),
        );
        Grid::set_column(&options_button, 1);
        title_row
            .children()
            .add(options_button.clone().upcast::<Control>());
        heading.children().add(title_row.upcast::<Control>());
        let environment = info.configuration.environment;
        let details = text(&format!(
            "{} · {} × {} · {}{}",
            short_uri(&info.configuration.source_uri),
            environment.width,
            environment.height,
            match environment.theme {
                PreviewTheme::Light => "Light",
                PreviewTheme::Dark => "Dark",
                PreviewTheme::Inherit => "App theme",
            },
            if info.configuration.pinned {
                " · Pinned"
            } else {
                ""
            }
        ));
        details.classes().add("designer-muted");
        ToolTip::set_tip(
            &details,
            Some(Rc::new(info.configuration.source_uri.clone())),
        );
        heading.children().add(details.upcast::<Control>());
        let commands = WrapPanel::new();
        commands.set_item_spacing(4.0);
        commands.set_line_spacing(4.0);
        let retired = Rc::new(Cell::new(false));
        for (label, suffix, action, selected) in [
            (
                "Focus",
                "Focus",
                PreviewStageAction::Focus {
                    stage: info.id,
                    epoch: info.epoch,
                },
                false,
            ),
            (
                "Select",
                "Select",
                PreviewStageAction::Mode {
                    stage: info.id,
                    epoch: info.epoch,
                    mode: PreviewStageMode::Select,
                },
                info.configuration.mode == PreviewStageMode::Select,
            ),
            (
                "Live",
                "Live",
                PreviewStageAction::Mode {
                    stage: info.id,
                    epoch: info.epoch,
                    mode: PreviewStageMode::Live,
                },
                info.configuration.mode == PreviewStageMode::Live,
            ),
            (
                if info.configuration.auto_update {
                    "Pause"
                } else {
                    "Resume"
                },
                "Pause",
                PreviewStageAction::Pause {
                    stage: info.id,
                    epoch: info.epoch,
                    paused: info.configuration.auto_update,
                },
                !info.configuration.auto_update,
            ),
            (
                "Refresh",
                "Refresh",
                PreviewStageAction::Refresh {
                    stage: info.id,
                    epoch: info.epoch,
                },
                false,
            ),
        ] {
            let command = button(label, &format!("Stage{}{suffix}", info.id));
            command.classes().add("nav");
            command.classes().set("active", selected);
            if suffix == "Pause" {
                ToolTip::set_tip(
                    &command,
                    Some(Rc::new(
                        "Pause or resume source updates for this stage".to_owned(),
                    )),
                );
            }
            self.connect_button(&command, action, Some(retired.clone()));
            commands.children().add(command.upcast::<Control>());
        }
        heading.children().add(commands.upcast::<Control>());
        body.children().add(heading.upcast::<Control>());

        let frame = Grid::new();
        frame.set_name(Some(format!("Stage{}PreviewFrame", info.id)));
        frame.set_height(360.0);
        frame.set_clip_to_bounds(true);
        frame.set_margin(Thickness::new(10.0, 0.0, 10.0, 0.0));
        Grid::set_row(&frame, 1);
        let viewbox = Viewbox::new();
        viewbox.set_stretch(Stretch::Uniform);
        viewbox.set_stretch_direction(StretchDirection::DownOnly);
        viewbox.set_horizontal_alignment(HorizontalAlignment::Center);
        viewbox.set_vertical_alignment(VerticalAlignment::Center);
        viewbox.set_child(Some(host.clone()));
        frame.children().add(viewbox.clone().upcast::<Control>());
        if info.configuration.mode == PreviewStageMode::Select {
            let overlay = Border::new();
            overlay.set_name(Some(format!("Stage{}SelectionSurface", info.id)));
            overlay.set_background(Some(SolidColorBrush::parse("Transparent").unwrap().into()));
            let weak = Rc::downgrade(self);
            let weak_host = host.downgrade();
            let weak_overlay = overlay.downgrade();
            let retired = retired.clone();
            let id = info.id;
            let epoch = info.epoch;
            overlay.add_handler(InputElement::pointer_pressed_event(), move |_, event| {
                if !retired.get()
                    && event
                        .get_current_point(None)
                        .properties
                        .is_left_button_pressed
                {
                    if let (Some(view), Some(host), Some(overlay)) =
                        (weak.upgrade(), weak_host.upgrade(), weak_overlay.upgrade())
                    {
                        view.emit(
                            PreviewStageAction::Select {
                                stage: id,
                                epoch,
                                position: event.get_position(Some(&host)),
                            },
                            overlay.upcast(),
                        );
                    }
                }
                event.set_handled(true);
            });
            frame.children().add(overlay.upcast::<Control>());
        }
        // Native controls handle input first. Unhandled live-app shortcuts must not
        // bubble into the editor's Delete/Undo/selection or authoring commands.
        frame.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| event.set_handled(true),
            RoutingStrategies::BUBBLE,
            false,
        );
        body.children().add(frame.upcast::<Control>());
        let footer = StackPanel::new();
        footer.classes().add("designer-chrome");
        footer.set_spacing(5.0);
        footer.set_margin(Thickness::uniform(12.0));
        Grid::set_row(&footer, 2);
        let status = text(&status_text(info));
        status.set_name(Some(format!("Stage{}Status", info.id)));
        status.classes().add(
            if matches!(
                info.status,
                PreviewStageStatus::Failed { .. }
                    | PreviewStageStatus::InvalidSource { .. }
                    | PreviewStageStatus::MissingDocument
            ) {
                "designer-error"
            } else {
                "designer-muted"
            },
        );
        footer.children().add(status.upcast::<Control>());
        if !info.notices.is_empty() {
            let notices = text(&info.notices.join("\n"));
            notices.classes().add("designer-muted");
            footer.children().add(notices.upcast::<Control>());
        }
        body.children().add(footer.upcast::<Control>());

        let options = MenuFlyout::new();
        for (label, action) in [
            (
                "Edit stage…",
                PreviewStageAction::Edit {
                    stage: info.id,
                    epoch: info.epoch,
                },
            ),
            (
                "Duplicate",
                PreviewStageAction::Duplicate {
                    stage: info.id,
                    epoch: info.epoch,
                },
            ),
            (
                if info.configuration.pinned {
                    "Unpin from other tabs"
                } else {
                    "Pin across document tabs"
                },
                PreviewStageAction::Pin {
                    stage: info.id,
                    epoch: info.epoch,
                    pinned: !info.configuration.pinned,
                },
            ),
            (
                "Move earlier",
                PreviewStageAction::Move {
                    stage: info.id,
                    epoch: info.epoch,
                    offset: -1,
                },
            ),
            (
                "Move later",
                PreviewStageAction::Move {
                    stage: info.id,
                    epoch: info.epoch,
                    offset: 1,
                },
            ),
            (
                "Remove stage",
                PreviewStageAction::Remove {
                    stage: info.id,
                    epoch: info.epoch,
                },
            ),
        ] {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(label.to_owned())));
            let weak = Rc::downgrade(self);
            let anchor = options_button.downgrade();
            let retired = retired.clone();
            item.click(move |_, event| {
                if !retired.get() {
                    if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                        view.emit(action.clone(), anchor.upcast());
                    }
                }
                event.set_handled(true);
            });
            options.items().add(Some(Control::boxed(item)));
        }
        if let Some(palette) = self.palette.upgrade() {
            crate::chrome::apply_designer_menu_resources(&options_button, &palette);
        }
        options_button.set_flyout(Some(options.clone().upcast()));
        StageCard {
            root,
            viewbox,
            host,
            epoch: info.epoch,
            retired,
            options,
        }
    }
}

impl Drop for PreviewStageView {
    fn drop(&mut self) {
        for card in self.cards.get_mut().values() {
            card.retire();
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
fn short_uri(uri: &str) -> &str {
    uri.rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(uri)
}
fn status_text(info: &PreviewStageInfo) -> String {
    match &info.status {
        PreviewStageStatus::Ready { revision } => format!("Ready · source revision {revision}"),
        PreviewStageStatus::Paused {
            revision: Some(revision),
        } => format!("Updates paused · last rendered revision {revision}"),
        PreviewStageStatus::Paused { revision: None } => {
            "Updates paused · Refresh to build this stage".into()
        }
        PreviewStageStatus::PendingSource { .. } => {
            "Source draft pending · showing the last accepted preview".into()
        }
        PreviewStageStatus::InvalidSource { .. } => {
            "Invalid XAML · showing the last accepted preview".into()
        }
        PreviewStageStatus::MissingDocument => {
            "Source document is unavailable · open it or edit the stage".into()
        }
        PreviewStageStatus::Failed { message, .. } => message.clone(),
    }
}

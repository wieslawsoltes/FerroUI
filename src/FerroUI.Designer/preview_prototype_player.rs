//! Native prototype presentation. Application roots and navigation remain host-owned.
use crate::PreviewStageSize;
use ferroui_base::input::{InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, Stretch, StretchDirection, TextWrapping};
use ferroui_base::{Rect, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Border, Button, Canvas, CheckBox, ColumnDefinitions, Control, Grid, RowDefinitions,
    ScrollViewer, StackPanel, TextBlock, TextBox, Viewbox, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Clipped bounds are expressed in the accepted stage host's logical coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewPrototypeHotspot {
    pub link: usize,
    pub name: String,
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewPrototypePlayerAction {
    Activate { link: usize },
    Back,
    Restart,
    Exit,
}

type Action = Rc<dyn Fn(PreviewPrototypePlayerAction, u64, Ref<Control>) -> Result<(), String>>;

struct HotspotControl {
    info: PreviewPrototypeHotspot,
    button: Ref<Button>,
    command: Ref<Button>,
}

pub struct PreviewPrototypePlayerView {
    root: Ref<Grid>,
    title: Ref<TextBlock>,
    dimensions: Ref<TextBlock>,
    back: Ref<Button>,
    error: Ref<TextBlock>,
    hints: Ref<CheckBox>,
    search: Ref<TextBox>,
    empty: Ref<TextBlock>,
    content: Ref<Grid>,
    hotspots: Ref<Canvas>,
    commands: Ref<StackPanel>,
    host: RefCell<Option<Ref<Control>>>,
    controls: RefCell<Vec<HotspotControl>>,
    epoch: Cell<u64>,
    frame_retired: RefCell<Rc<Cell<bool>>>,
    action: RefCell<Option<Action>>,
}

impl PreviewPrototypePlayerView {
    pub fn new(_palette: &Control) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewPrototypePlayer".into()));
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto,*").unwrap());
        let header = StackPanel::new();
        header.classes().add("designer-chrome");
        header.set_margin(Thickness::new(16.0, 14.0, 16.0, 10.0));
        header.set_spacing(7.0);
        let title = text("Prototype");
        title.set_name(Some("PrototypePlayerTitle".into()));
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        header.children().add(title.clone().upcast::<Control>());
        let toolbar = WrapPanel::new();
        toolbar.set_item_spacing(7.0);
        toolbar.set_line_spacing(6.0);
        let back = button("Back", "PrototypePlayerBack");
        let restart = button("Restart journey", "PrototypePlayerRestart");
        let exit = button("Exit prototype", "PrototypePlayerExit");
        let hints = CheckBox::new();
        hints.set_name(Some("PrototypePlayerHints".into()));
        hints.set_content(Some(Rc::new("Show hotspots".to_owned())));
        hints.set_is_checked(Some(false));
        AutomationProperties::set_name(&hints, Some("Show prototype hotspots"));
        for control in [
            back.clone().upcast::<Control>(),
            restart.clone().upcast(),
            exit.clone().upcast(),
            hints.clone().upcast(),
        ] {
            toolbar.children().add(control);
        }
        header.children().add(toolbar.upcast::<Control>());
        root.children().add(header.upcast::<Control>());
        let error = text("");
        error.set_name(Some("PrototypePlayerError".into()));
        error.classes().add("designer-error");
        error.classes().add("designer-chrome");
        error.set_is_visible(false);
        error.set_margin(Thickness::new(16.0, 0.0, 16.0, 8.0));
        Grid::set_row(&error, 1);
        root.children().add(error.clone().upcast::<Control>());
        let body = Grid::new();
        body.set_column_definitions(ColumnDefinitions::parse("*,240").unwrap());
        body.set_column_spacing(16.0);
        body.set_margin(Thickness::new(16.0, 0.0, 16.0, 16.0));
        Grid::set_row(&body, 2);
        let viewport = Border::new();
        viewport.set_name(Some("PrototypePlayerViewport".into()));
        viewport.set_background(Some(SolidColorBrush::parse("#15161A").unwrap().into()));
        viewport.set_clip_to_bounds(true);
        viewport.set_padding(Thickness::uniform(16.0));
        let content = Grid::new();
        content.set_clip_to_bounds(true);
        let hotspots = Canvas::new();
        hotspots.set_name(Some("PrototypePlayerHotspots".into()));
        hotspots.set_clip_to_bounds(true);
        content.children().add(hotspots.clone().upcast::<Control>());
        let viewbox = Viewbox::new();
        viewbox.set_name(Some("PrototypePlayerViewbox".into()));
        viewbox.set_stretch(Stretch::Uniform);
        viewbox.set_stretch_direction(StretchDirection::Both);
        viewbox.set_horizontal_alignment(HorizontalAlignment::Center);
        viewbox.set_vertical_alignment(VerticalAlignment::Center);
        viewbox.set_child(Some(content.clone().upcast::<Control>()));
        viewport.set_child(Some(viewbox.upcast::<Control>()));
        body.children().add(viewport.upcast::<Control>());
        let aside = Grid::new();
        aside.set_name(Some("PrototypePlayerInteractions".into()));
        aside.classes().add("designer-chrome");
        aside.set_row_definitions(RowDefinitions::parse("Auto,Auto,Auto,*").unwrap());
        Grid::set_column(&aside, 1);
        let dimensions = text("");
        dimensions.set_name(Some("PrototypePlayerDimensions".into()));
        dimensions.classes().add("designer-muted");
        dimensions.set_margin(Thickness::new(0.0, 0.0, 0.0, 12.0));
        aside.children().add(dimensions.clone().upcast::<Control>());
        let heading = text("Interactions");
        heading.set_font_weight(FontWeight::SemiBold);
        heading.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        Grid::set_row(&heading, 1);
        aside.children().add(heading.upcast::<Control>());
        let search = TextBox::new();
        search.set_name(Some("PrototypePlayerSearch".into()));
        search.set_placeholder_text(Some("Find an interaction"));
        search.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        AutomationProperties::set_name(&search, Some("Search prototype interactions"));
        Grid::set_row(&search, 2);
        aside.children().add(search.clone().upcast::<Control>());
        let commands = StackPanel::new();
        commands.set_spacing(7.0);
        let empty = text("This screen has no available interactions. Native controls remain live.");
        empty.set_name(Some("PrototypePlayerEmpty".into()));
        empty.classes().add("designer-muted");
        commands.children().add(empty.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(commands.clone().upcast::<Control>())));
        Grid::set_row(&scroll, 3);
        aside.children().add(scroll.upcast::<Control>());
        body.children().add(aside.upcast::<Control>());
        root.children().add(body.upcast::<Control>());
        let view = Rc::new(Self {
            root,
            title,
            dimensions,
            back,
            error,
            hints,
            search,
            empty,
            content,
            hotspots,
            commands,
            host: RefCell::new(None),
            controls: RefCell::new(Vec::new()),
            epoch: Cell::new(0),
            frame_retired: RefCell::new(Rc::new(Cell::new(false))),
            action: RefCell::new(None),
        });
        for (control, action) in [
            (view.back.clone(), PreviewPrototypePlayerAction::Back),
            (restart, PreviewPrototypePlayerAction::Restart),
            (exit, PreviewPrototypePlayerAction::Exit),
        ] {
            let weak = Rc::downgrade(&view);
            let anchor = control.downgrade();
            control.click(move |_, event| {
                if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    if view.host.borrow().is_some() {
                        view.emit(action.clone(), view.epoch.get(), anchor.upcast());
                    }
                }
                event.set_handled(true);
            });
        }
        let weak = Rc::downgrade(&view);
        view.hints.is_checked_changed(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.refresh_hints();
            }
        });
        let weak = Rc::downgrade(&view);
        view.search.text_changing(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.filter_commands();
            }
        });
        let weak = Rc::downgrade(&view);
        view.root.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key == Key::Escape {
                    if let Some(view) = weak.upgrade() {
                        if view.host.borrow().is_some() {
                            view.emit(
                                PreviewPrototypePlayerAction::Exit,
                                view.epoch.get(),
                                view.root.clone().upcast(),
                            );
                        }
                    }
                }
                // Application input gets first refusal. Unhandled shortcuts belong to
                // this presentation, never to the designer's source authoring commands.
                event.set_handled(true);
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        view
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn on_action(
        &self,
        callback: impl Fn(PreviewPrototypePlayerAction, u64, Ref<Control>) -> Result<(), String>
            + 'static,
    ) {
        *self.action.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
        self.error.set_is_visible(!message.is_empty());
    }
    fn emit(&self, action: PreviewPrototypePlayerAction, epoch: u64, anchor: Ref<Control>) {
        let result = self
            .action
            .borrow()
            .clone()
            .ok_or_else(|| "No prototype player host is attached".to_owned())
            .and_then(|callback| callback(action, epoch, anchor));
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
    }

    pub fn update(
        self: &Rc<Self>,
        host: Ref<Control>,
        size: PreviewStageSize,
        title: &str,
        hotspots: &[PreviewPrototypeHotspot],
        can_back: bool,
        epoch: u64,
    ) {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            self.clear();
            self.show_error("The prototype frame has no finite native layout size");
            return;
        }
        let same_host = self
            .host
            .borrow()
            .as_ref()
            .is_some_and(|current| current == &host);
        let same_hotspots = self.epoch.get() == epoch
            && self
                .controls
                .borrow()
                .iter()
                .map(|control| &control.info)
                .eq(hotspots.iter());
        self.title.set_text(Some(title));
        self.dimensions.set_text(Some(&format!(
            "{} × {} logical units\nEach screen retains its live state during this journey.",
            size.width, size.height
        )));
        self.back.set_is_enabled(can_back);
        self.content.set_width(size.width);
        self.content.set_height(size.height);
        if !same_host {
            if let Some(previous) = self.host.borrow_mut().take() {
                self.content.children().remove(&previous);
            }
            self.content.children().insert(0, host.clone());
            *self.host.borrow_mut() = Some(host);
        }
        if same_host && same_hotspots {
            return;
        }
        self.frame_retired.borrow().set(true);
        *self.frame_retired.borrow_mut() = Rc::new(Cell::new(false));
        self.epoch.set(epoch);
        self.hotspots.children().clear();
        self.commands.children().clear();
        self.commands
            .children()
            .add(self.empty.clone().upcast::<Control>());
        let mut controls = Vec::new();
        for (index, hotspot) in hotspots.iter().enumerate() {
            let bounds = hotspot.bounds;
            if ![bounds.x, bounds.y, bounds.width, bounds.height]
                .iter()
                .all(|value| value.is_finite())
                || bounds.width <= 0.0
                || bounds.height <= 0.0
            {
                continue;
            }
            let hit = button("", &format!("PrototypeHotspot{}", hotspot.link));
            AutomationProperties::set_name(&hit, Some(&hotspot.name));
            hit.set_min_width(0.0);
            hit.set_min_height(0.0);
            hit.set_width(bounds.width);
            hit.set_height(bounds.height);
            hit.set_padding(Thickness::uniform(0.0));
            hit.set_font_size(11.0);
            Canvas::set_left(&hit, bounds.x);
            Canvas::set_top(&hit, bounds.y);
            self.hotspots
                .children()
                .add(hit.clone().upcast::<Control>());
            let command = button(
                &format!("{}. {}", index + 1, hotspot.name),
                &format!("PrototypeActivate{}", hotspot.link),
            );
            command.set_horizontal_alignment(HorizontalAlignment::Stretch);
            // A wrapping native text presenter keeps long interaction names usable.
            command.set_content(Some(Control::boxed(text(&format!(
                "{}. {}",
                index + 1,
                hotspot.name
            )))));
            self.commands
                .children()
                .add(command.clone().upcast::<Control>());
            for control in [&hit, &command] {
                let weak = Rc::downgrade(self);
                let anchor = control.downgrade();
                let retired = self.frame_retired.borrow().clone();
                let link = hotspot.link;
                control.click(move |_, event| {
                    if !retired.get() {
                        if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                            view.emit(
                                PreviewPrototypePlayerAction::Activate { link },
                                epoch,
                                anchor.upcast(),
                            );
                        }
                    }
                    event.set_handled(true);
                });
            }
            controls.push(HotspotControl {
                info: hotspot.clone(),
                button: hit,
                command,
            });
        }
        *self.controls.borrow_mut() = controls;
        self.refresh_hints();
        self.filter_commands();
    }

    fn refresh_hints(&self) {
        let show = self.hints.is_checked() == Some(true);
        for (index, control) in self.controls.borrow().iter().enumerate() {
            control.button.set_background(Some(
                SolidColorBrush::parse(if show { "#2660A5FA" } else { "Transparent" })
                    .unwrap()
                    .into(),
            ));
            control.button.set_border_brush(Some(
                SolidColorBrush::parse(if show { "#60A5FA" } else { "Transparent" })
                    .unwrap()
                    .into(),
            ));
            control
                .button
                .set_border_thickness(Thickness::uniform(if show { 1.5 } else { 0.0 }));
            control.button.set_content(
                show.then(|| Rc::new((index + 1).to_string()) as ferroui_base::BoxedValue),
            );
        }
    }

    fn filter_commands(&self) {
        let query = self.search.text().unwrap_or_default().to_lowercase();
        let terms = query.split_whitespace().collect::<Vec<_>>();
        let controls = self.controls.borrow();
        let mut count = 0;
        for control in controls.iter() {
            let visible = terms
                .iter()
                .all(|term| control.info.name.to_lowercase().contains(term));
            control.command.set_is_visible(visible);
            count += usize::from(visible);
        }
        self.empty.set_is_visible(count == 0);
        self.empty.set_text(Some(if controls.is_empty() {
            "This screen has no available interactions. Native controls remain live."
        } else {
            "No interaction matches this search."
        }));
    }

    /// Retire all captured callback epochs before the host drops its playback graph.
    pub fn clear(&self) {
        self.frame_retired.borrow().set(true);
        if let Some(host) = self.host.borrow_mut().take() {
            self.content.children().remove(&host);
        }
        self.hotspots.children().clear();
        self.controls.borrow_mut().clear();
        self.commands.children().clear();
        self.commands
            .children()
            .add(self.empty.clone().upcast::<Control>());
        self.back.set_is_enabled(false);
        self.show_error("");
    }
}

impl Drop for PreviewPrototypePlayerView {
    fn drop(&mut self) {
        self.frame_retired.get_mut().set(true);
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

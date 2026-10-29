//! Native prototype presentation. Application roots and navigation remain host-owned.
use crate::{
    PreviewPrototypeOverlayOptions, PreviewStageId, PreviewStageSize,
    MAX_PREVIEW_PROTOTYPE_OVERLAYS,
};
use ferroui_base::input::{
    FocusManager, IKeyboardNavigationHandler, InputElement, Key, KeyModifiers, KeyboardNavigation,
    KeyboardNavigationHandler, KeyboardNavigationMode, NavigationDirection, NavigationMethod,
    PointerType,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, Stretch, StretchDirection, TextWrapping};
use ferroui_base::{Rect, Ref, Thickness, WeakRef};
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

/// A retained native screen in the base screen's logical coordinate space.
/// The first layer is the base screen; subsequent layers are modal overlays.
#[derive(Clone)]
pub struct PreviewPrototypePlayerLayer {
    pub stage: PreviewStageId,
    pub host: Ref<Control>,
    pub bounds: Rect,
    pub overlay: Option<PreviewPrototypeOverlayOptions>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewPrototypePlayerAction {
    Activate { link: usize },
    Back,
    CloseOverlay,
    DismissOverlay,
    Restart,
    Exit,
}

type Action = Rc<dyn Fn(PreviewPrototypePlayerAction, u64, Ref<Control>) -> Result<(), String>>;

struct HotspotControl {
    info: PreviewPrototypeHotspot,
    button: Ref<Button>,
    command: Ref<Button>,
}

#[derive(Clone)]
enum SavedFocus {
    Native(WeakRef<InputElement>),
    Hotspot { link: usize, command: bool },
}

struct PlayerLayer {
    stage: PreviewStageId,
    host: Ref<Control>,
    container: Ref<Canvas>,
    plane: Ref<Grid>,
    native: Ref<Canvas>,
    scrim: Option<Ref<Border>>,
    options: Option<PreviewPrototypeOverlayOptions>,
    focus: Option<SavedFocus>,
    retired: Rc<Cell<bool>>,
}

pub struct PreviewPrototypePlayerView {
    root: Ref<Grid>,
    _keyboard_navigation: Rc<KeyboardNavigationHandler>,
    title: Ref<TextBlock>,
    dimensions: Ref<TextBlock>,
    back: Ref<Button>,
    close_overlay: Ref<Button>,
    error: Ref<TextBlock>,
    hints: Ref<CheckBox>,
    search: Ref<TextBox>,
    empty: Ref<TextBlock>,
    content: Ref<Grid>,
    hotspots: Ref<Canvas>,
    commands: Ref<StackPanel>,
    host: RefCell<Option<Ref<Control>>>,
    layers: RefCell<Vec<PlayerLayer>>,
    controls: RefCell<Vec<HotspotControl>>,
    hotspot_focus_targets: RefCell<Vec<(usize, WeakRef<InputElement>)>>,
    epoch: Cell<u64>,
    frame_retired: RefCell<Rc<Cell<bool>>>,
    focus_pending: Cell<bool>,
    action: RefCell<Option<Action>>,
}

impl PreviewPrototypePlayerView {
    pub fn new(_palette: &Control) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewPrototypePlayer".into()));
        let keyboard_navigation = KeyboardNavigationHandler::new();
        keyboard_navigation.set_owner(&root.clone().upcast());
        KeyboardNavigation::set_tab_navigation(&root, KeyboardNavigationMode::Cycle);
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
        let close_overlay = button("Close overlay", "PrototypePlayerCloseOverlay");
        close_overlay.set_is_visible(false);
        let restart = button("Restart journey", "PrototypePlayerRestart");
        let exit = button("Exit prototype", "PrototypePlayerExit");
        let hints = CheckBox::new();
        hints.set_name(Some("PrototypePlayerHints".into()));
        hints.set_content(Some(Rc::new("Show hotspots".to_owned())));
        hints.set_is_checked(Some(false));
        AutomationProperties::set_name(&hints, Some("Show prototype hotspots"));
        for control in [
            back.clone().upcast::<Control>(),
            close_overlay.clone().upcast(),
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
            _keyboard_navigation: keyboard_navigation,
            title,
            dimensions,
            back,
            close_overlay,
            error,
            hints,
            search,
            empty,
            content,
            hotspots,
            commands,
            host: RefCell::new(None),
            layers: RefCell::new(Vec::new()),
            controls: RefCell::new(Vec::new()),
            hotspot_focus_targets: RefCell::new(Vec::new()),
            epoch: Cell::new(0),
            frame_retired: RefCell::new(Rc::new(Cell::new(false))),
            focus_pending: Cell::new(false),
            action: RefCell::new(None),
        });
        for (control, action) in [
            (view.back.clone(), PreviewPrototypePlayerAction::Back),
            (
                view.close_overlay.clone(),
                PreviewPrototypePlayerAction::CloseOverlay,
            ),
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
                            let action = if view.layers.borrow().len() > 1 {
                                PreviewPrototypePlayerAction::CloseOverlay
                            } else {
                                PreviewPrototypePlayerAction::Exit
                            };
                            view.emit(action, view.epoch.get(), view.root.clone().upcast());
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
        view.install_modal_input_guards();
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

    /// Bind source controls to the current frame's semantic interaction identities.
    /// Keep the previous frame's bindings until preparation has saved its focus;
    /// the host supplies the new bindings before finalizing the native frame.
    pub(crate) fn set_hotspot_focus_targets(&self, targets: &[(usize, Ref<InputElement>)]) {
        *self.hotspot_focus_targets.borrow_mut() = targets
            .iter()
            .map(|(link, target)| (*link, target.downgrade()))
            .collect();
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
        let layer = PreviewPrototypePlayerLayer {
            stage: 0,
            host,
            bounds: Rect::new(0.0, 0.0, size.width, size.height),
            overlay: None,
        };
        if let Err(error) = self.update_layers(size, &[layer], title, hotspots, can_back, epoch) {
            self.clear();
            self.show_error(&error);
        }
    }

    /// Present a base screen and its modal overlays without rebuilding retained hosts.
    /// Hotspots are local to the topmost layer. A rejected descriptor leaves the
    /// accepted native presentation intact.
    pub fn update_layers(
        self: &Rc<Self>,
        frame_size: PreviewStageSize,
        layers: &[PreviewPrototypePlayerLayer],
        title: &str,
        hotspots: &[PreviewPrototypeHotspot],
        can_back: bool,
        epoch: u64,
    ) -> Result<(), String> {
        self.update_layers_core(frame_size, layers, title, hotspots, can_back, epoch, true)
    }

    /// Attach and measure native layers while their final hotspots are settling.
    /// Focus is saved once and restored by the next successful `update_layers`.
    pub(crate) fn prepare_layers(
        self: &Rc<Self>,
        frame_size: PreviewStageSize,
        layers: &[PreviewPrototypePlayerLayer],
        title: &str,
        hotspots: &[PreviewPrototypeHotspot],
        can_back: bool,
        epoch: u64,
    ) -> Result<(), String> {
        self.update_layers_core(frame_size, layers, title, hotspots, can_back, epoch, false)
    }

    fn update_layers_core(
        self: &Rc<Self>,
        frame_size: PreviewStageSize,
        layers: &[PreviewPrototypePlayerLayer],
        title: &str,
        hotspots: &[PreviewPrototypeHotspot],
        can_back: bool,
        epoch: u64,
        finalize_focus: bool,
    ) -> Result<(), String> {
        if !frame_size.width.is_finite()
            || !frame_size.height.is_finite()
            || frame_size.width <= 0.0
            || frame_size.height <= 0.0
            || layers.is_empty()
            || layers.len() > MAX_PREVIEW_PROTOTYPE_OVERLAYS + 1
        {
            return Err(
                "The prototype frame has no finite native layout size or valid layer stack".into(),
            );
        }
        for (index, layer) in layers.iter().enumerate() {
            let bounds = layer.bounds;
            if ![bounds.x, bounds.y, bounds.width, bounds.height]
                .iter()
                .all(|v| v.is_finite())
                || bounds.width <= 0.0
                || bounds.height <= 0.0
                || (index == 0) != layer.overlay.is_none()
                || layers[..index]
                    .iter()
                    .any(|previous| previous.stage == layer.stage || previous.host == layer.host)
            {
                return Err("Prototype layers need unique native hosts, finite geometry and one base screen".into());
            }
            if index == 0 && bounds != Rect::new(0.0, 0.0, frame_size.width, frame_size.height) {
                return Err("The prototype base screen must fill its logical frame".into());
            }
            if let Some(options) = layer.overlay {
                options.validate()?;
            }
            if layer.host.visual_parent().is_some()
                && !self
                    .layers
                    .borrow()
                    .iter()
                    .any(|current| current.host == layer.host)
            {
                return Err("A prototype view is already mounted by another native parent".into());
            }
        }
        let top = layers.last().unwrap();
        let old_count = self.layers.borrow().len();
        let common = self
            .layers
            .borrow()
            .iter()
            .zip(layers)
            .take_while(|(current, next)| {
                current.stage == next.stage
                    && current.host == next.host
                    && current.options.is_some() == next.overlay.is_some()
            })
            .count();
        let same_host = self
            .layers
            .borrow()
            .last()
            .is_some_and(|current| current.stage == top.stage && current.host == top.host);
        let same_hotspots = self.epoch.get() == epoch
            && self
                .controls
                .borrow()
                .iter()
                .map(|control| &control.info)
                .eq(hotspots.iter());
        let active_changed = !same_host || common != old_count || common != layers.len();
        let captured = if !self.focus_pending.get() && (active_changed || !same_hotspots) {
            self.capture_focus()
        } else {
            None
        };
        if let Some(focus) = captured.clone() {
            if let Some(current) = self.layers.borrow_mut().last_mut() {
                current.focus = Some(focus);
            }
        }
        if (active_changed && (old_count > 1 || layers.len() > 1)) || captured.is_some() {
            self.focus_pending.set(true);
        }
        self.title.set_text(Some(title));
        self.dimensions.set_text(Some(&format!(
            "{} × {} logical units\n{}{}",
            frame_size.width,
            frame_size.height,
            if layers.len() > 1 {
                format!(
                    "{} overlay{} open. ",
                    layers.len() - 1,
                    if layers.len() == 2 { "" } else { "s" }
                )
            } else {
                String::new()
            },
            "Each screen retains its live state during this journey."
        )));
        self.back.set_is_enabled(can_back);
        self.close_overlay.set_is_visible(layers.len() > 1);
        self.content.set_width(frame_size.width);
        self.content.set_height(frame_size.height);

        let rebuild_hotspots = active_changed || !same_hotspots;
        if rebuild_hotspots {
            self.frame_retired.borrow().set(true);
            if let Some(current) = self.layers.borrow().last() {
                current
                    .plane
                    .children()
                    .remove(&self.hotspots.clone().upcast::<Control>());
            }
        }
        let removed = self.layers.borrow_mut().split_off(common);
        for layer in removed.into_iter().rev() {
            layer.retired.set(true);
            self.content
                .children()
                .remove(&layer.container.clone().upcast::<Control>());
            layer.native.children().remove(&layer.host);
        }
        for descriptor in layers.iter().skip(common) {
            let layer = self.create_layer(descriptor);
            self.content
                .children()
                .add(layer.container.clone().upcast::<Control>());
            self.layers.borrow_mut().push(layer);
        }
        {
            let mut accepted = self.layers.borrow_mut();
            for (index, (current, descriptor)) in accepted.iter_mut().zip(layers).enumerate() {
                current.options = descriptor.overlay;
                current.container.set_width(frame_size.width);
                current.container.set_height(frame_size.height);
                let active = index + 1 == layers.len();
                current.container.set_is_hit_test_visible(active);
                KeyboardNavigation::set_tab_navigation(
                    &current.container,
                    if active {
                        if layers.len() > 1 {
                            KeyboardNavigationMode::Cycle
                        } else {
                            KeyboardNavigationMode::Continue
                        }
                    } else {
                        KeyboardNavigationMode::None
                    },
                );
                current.plane.set_width(descriptor.bounds.width);
                current.plane.set_height(descriptor.bounds.height);
                Canvas::set_left(&current.plane, descriptor.bounds.x);
                Canvas::set_top(&current.plane, descriptor.bounds.y);
                if let Some(scrim) = &current.scrim {
                    scrim.set_width(frame_size.width);
                    scrim.set_height(frame_size.height);
                    scrim.set_background(Some(
                        SolidColorBrush::parse(
                            if descriptor
                                .overlay
                                .is_some_and(|options| options.dim_background)
                            {
                                "#66000000"
                            } else {
                                "Transparent"
                            },
                        )
                        .unwrap()
                        .into(),
                    ));
                }
            }
        }
        *self.host.borrow_mut() = Some(top.host.clone());
        if !rebuild_hotspots {
            self.finish_focus_update(finalize_focus);
            return Ok(());
        }
        self.layers
            .borrow()
            .last()
            .unwrap()
            .plane
            .children()
            .add(self.hotspots.clone().upcast::<Control>());
        self.rebuild_hotspots(hotspots, epoch);
        self.finish_focus_update(finalize_focus);
        Ok(())
    }

    fn finish_focus_update(&self, finalize: bool) {
        if finalize && self.focus_pending.replace(false) && self.root.is_attached_to_visual_tree() {
            self.root.update_layout();
            self.restore_active_focus();
        }
    }

    fn create_layer(self: &Rc<Self>, descriptor: &PreviewPrototypePlayerLayer) -> PlayerLayer {
        let container = Canvas::new();
        container.set_name(Some(format!("PrototypePlayerLayer{}", descriptor.stage)));
        container.set_clip_to_bounds(true);
        let plane = Grid::new();
        plane.set_name(Some(format!("PrototypePlayerPlane{}", descriptor.stage)));
        plane.set_clip_to_bounds(true);
        if descriptor.overlay.is_some() {
            // Empty space inside a transparent native overlay is still inside
            // its presentation, so it must not hit the outside-dismiss scrim.
            plane.set_background(Some(SolidColorBrush::parse("Transparent").unwrap().into()));
        }
        plane.set_focusable(true);
        KeyboardNavigation::set_is_tab_stop(&plane, false);
        let retired = Rc::new(Cell::new(false));
        let scrim = descriptor.overlay.map(|_| {
            let scrim = Border::new();
            scrim.set_name(Some(format!("PrototypeOverlayScrim{}", descriptor.stage)));
            let weak = Rc::downgrade(self);
            let anchor = scrim.downgrade();
            let revoked = retired.clone();
            let stage = descriptor.stage;
            scrim.pointer_pressed(move |_, event| {
                // The dismissal press is consumed even when dismissal is disabled.
                // A close cannot click through to the newly exposed native screen.
                event.set_handled(true);
                if revoked.get()
                    || (event.pointer().type_() == PointerType::Mouse
                        && !event
                            .get_current_point(None)
                            .properties
                            .is_left_button_pressed)
                {
                    return;
                }
                if let (Some(view), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    let dismiss = view.layers.borrow().last().is_some_and(|layer| {
                        layer.stage == stage
                            && layer
                                .options
                                .is_some_and(|options| options.dismiss_on_outside)
                    });
                    if dismiss {
                        view.emit(
                            PreviewPrototypePlayerAction::DismissOverlay,
                            view.epoch.get(),
                            anchor.upcast(),
                        );
                    }
                }
            });
            scrim.pointer_released(|_, event| event.set_handled(true));
            scrim.pointer_wheel_changed(|_, event| event.set_handled(true));
            container.children().add(scrim.clone().upcast::<Control>());
            scrim
        });
        // Native stage hosts supply their own finite layout proposal. Measuring
        // them in a Canvas lets fitted content grow beyond the previous frame;
        // the host's resolved size then drives the next positioning pass.
        let native = Canvas::new();
        native.children().add(descriptor.host.clone());
        plane.children().add(native.clone().upcast::<Control>());
        container.children().add(plane.clone().upcast::<Control>());
        PlayerLayer {
            stage: descriptor.stage,
            host: descriptor.host.clone(),
            container,
            plane,
            native,
            scrim,
            options: descriptor.overlay,
            focus: None,
            retired,
        }
    }

    fn rebuild_hotspots(self: &Rc<Self>, hotspots: &[PreviewPrototypeHotspot], epoch: u64) {
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

    fn capture_focus(&self) -> Option<SavedFocus> {
        let focused = FocusManager::get_focus_manager(&self.root)?.get_focused_element()?;
        for control in self.controls.borrow().iter() {
            if contains(&control.button, &focused) {
                return Some(SavedFocus::Hotspot {
                    link: control.info.link,
                    command: false,
                });
            }
            if contains(&control.command, &focused) {
                return Some(SavedFocus::Hotspot {
                    link: control.info.link,
                    command: true,
                });
            }
        }
        self.layers
            .borrow()
            .last()
            .filter(|layer| contains(&layer.plane, &focused))
            .map(|_| {
                self.source_hotspot(&focused)
                    .map(|link| SavedFocus::Hotspot {
                        link,
                        command: false,
                    })
                    .unwrap_or_else(|| SavedFocus::Native(focused.downgrade()))
            })
    }

    fn source_hotspot(&self, element: &InputElement) -> Option<usize> {
        // The host orders source anchors from outermost to innermost, matching
        // hotspot paint order. A nested interactive source owns its own focus.
        self.hotspot_focus_targets
            .borrow()
            .iter()
            .rev()
            .find_map(|(link, target)| {
                let target = target.upgrade()?;
                (target == element.to_ref()
                    || target.is_visual_ancestor_of(element)
                    || target.is_logical_ancestor_of(Some(element)))
                .then_some(*link)
            })
    }

    fn hotspot_focus(&self, link: usize, command: bool) -> Option<Ref<InputElement>> {
        self.controls
            .borrow()
            .iter()
            .find(|control| control.info.link == link)
            .and_then(|control| {
                let hit = control.button.clone().upcast::<InputElement>();
                let command = command.then(|| control.command.clone().upcast::<InputElement>());
                // Search may hide the opener's command while a dialog is open.
                // The same interaction's visible hotspot is the next best target.
                command
                    .filter(|target| FocusManager::can_focus(target))
                    .or_else(|| FocusManager::can_focus(&hit).then_some(hit))
            })
    }

    fn redirect_source_focus(
        &self,
        source: &Ref<InputElement>,
        previous: Option<&Ref<InputElement>>,
        method: NavigationMethod,
        modifiers: KeyModifiers,
    ) -> Option<Ref<InputElement>> {
        let link = self.source_hotspot(source)?;
        let target = self.hotspot_focus(link, false)?;
        if previous != Some(&target) || method != NavigationMethod::Tab {
            return Some(target);
        }
        // The original native source and its overlaid Button are two positions
        // in the native tab traversal but represent one prototype interaction.
        // Skip aliases of the already focused interaction in either direction,
        // otherwise Shift+Tab can redirect back to itself forever.
        let direction = if modifiers.contains(KeyModifiers::SHIFT) {
            NavigationDirection::Previous
        } else {
            NavigationDirection::Next
        };
        let mut cursor = source.clone();
        let mut visited = vec![cursor.clone()];
        for _ in 0..1024 {
            let Some(next) = KeyboardNavigationHandler::get_next(&cursor, direction) else {
                break;
            };
            if visited.contains(&next) {
                break;
            }
            visited.push(next.clone());
            let candidate = self
                .source_hotspot(&next)
                .and_then(|link| self.hotspot_focus(link, false))
                .unwrap_or_else(|| next.clone());
            if candidate != target
                && FocusManager::can_focus(&candidate)
                && !self.blocked_source(Some(candidate.clone()))
            {
                return Some(candidate);
            }
            cursor = next;
        }
        // A custom native navigation provider may cycle or expose an enormous
        // linked subtree. Keep the current valid interaction focus in that case.
        Some(target)
    }

    fn first_active_focus(&self) -> Option<Ref<InputElement>> {
        let layers = self.layers.borrow();
        let layer = layers.last()?;
        FocusManager::find_first_focusable_element_in(&layer.host.clone().upcast())
            .map(|target| {
                self.source_hotspot(&target)
                    .and_then(|link| self.hotspot_focus(link, false))
                    .unwrap_or(target)
            })
            .or_else(|| {
                self.controls
                    .borrow()
                    .iter()
                    .find(|control| FocusManager::can_focus(&control.button))
                    .map(|control| control.button.clone().upcast())
            })
            .or_else(|| FocusManager::can_focus(&layer.plane).then(|| layer.plane.clone().upcast()))
    }

    fn restore_active_focus(&self) {
        let saved = self
            .layers
            .borrow()
            .last()
            .and_then(|layer| layer.focus.clone());
        let target = match saved {
            Some(SavedFocus::Native(element)) => element
                .upgrade()
                .filter(|element| {
                    FocusManager::can_focus(element)
                        && self
                            .layers
                            .borrow()
                            .last()
                            .is_some_and(|layer| contains(&layer.plane, element))
                })
                .map(|element| {
                    self.source_hotspot(&element)
                        .and_then(|link| self.hotspot_focus(link, false))
                        .unwrap_or(element)
                }),
            Some(SavedFocus::Hotspot { link, command }) => self.hotspot_focus(link, command),
            None => None,
        }
        .or_else(|| self.first_active_focus());
        if let Some(target) = target {
            target.focus();
        }
    }

    fn blocked_source(&self, source: Option<Ref<InputElement>>) -> bool {
        let Some(source) = source else {
            return false;
        };
        let layers = self.layers.borrow();
        layers.len() > 1
            && layers[..layers.len() - 1]
                .iter()
                .any(|layer| contains(&layer.plane, &source))
    }

    fn install_modal_input_guards(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.root.add_handler_with(
            InputElement::getting_focus_event(),
            move |_, event| {
                if let Some(view) = weak.upgrade() {
                    let requested = event.new_focused_element();
                    if view.blocked_source(requested.clone()) {
                        if let Some(target) = view.first_active_focus() {
                            event.try_set_new_focused_element(Some(&target));
                        } else {
                            event.try_cancel();
                        }
                    } else if let Some(source) = requested {
                        let active_source = view
                            .layers
                            .borrow()
                            .last()
                            .is_some_and(|layer| contains(&layer.host, &source));
                        if active_source {
                            if let Some(target) = view.redirect_source_focus(
                                &source,
                                event.old_focused_element.as_ref(),
                                event.navigation_method,
                                event.key_modifiers,
                            ) {
                                // Keyboard navigation can reach the original
                                // source control beneath a transparent hotspot.
                                // Give its semantic interaction focus so Enter
                                // and Space cannot bypass prototype navigation.
                                event.try_set_new_focused_element(Some(&target));
                            }
                        }
                    }
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        macro_rules! block_lower_input {
            ($event:expr) => {{
                let weak = Rc::downgrade(self);
                self.root.add_handler_with(
                    $event,
                    move |_, event| {
                        if let Some(view) = weak.upgrade() {
                            if view.blocked_source(
                                event
                                    .source()
                                    .and_then(|source| source.cast::<InputElement>()),
                            ) {
                                event.set_handled(true);
                                view.restore_active_focus();
                            }
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
            }};
        }
        block_lower_input!(InputElement::key_down_event());
        block_lower_input!(InputElement::key_up_event());
        block_lower_input!(InputElement::text_input_event());
        macro_rules! block_lower_pointer {
            ($event:expr) => {{
                let weak = Rc::downgrade(self);
                self.root.add_handler_with(
                    $event,
                    move |_, event| {
                        if let Some(view) = weak.upgrade() {
                            if view.blocked_source(
                                event
                                    .source()
                                    .and_then(|source| source.cast::<InputElement>()),
                            ) {
                                event.set_handled(true);
                                // A host can open an overlay while an application drag
                                // owns capture. Retire that capture without delivering
                                // its release to the now-covered control.
                                event.pointer().capture(None);
                                view.restore_active_focus();
                            }
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
            }};
        }
        block_lower_pointer!(InputElement::pointer_pressed_event());
        block_lower_pointer!(InputElement::pointer_released_event());
        block_lower_pointer!(InputElement::pointer_moved_event());
        block_lower_pointer!(InputElement::pointer_wheel_changed_event());
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
        self.focus_pending.set(false);
        self.host.borrow_mut().take();
        let layers = self.layers.replace(Vec::new());
        for layer in layers.into_iter().rev() {
            layer.retired.set(true);
            layer
                .plane
                .children()
                .remove(&self.hotspots.clone().upcast::<Control>());
            self.content
                .children()
                .remove(&layer.container.clone().upcast::<Control>());
            layer.native.children().remove(&layer.host);
        }
        self.hotspots.children().clear();
        self.controls.borrow_mut().clear();
        self.hotspot_focus_targets.borrow_mut().clear();
        self.commands.children().clear();
        self.commands
            .children()
            .add(self.empty.clone().upcast::<Control>());
        self.back.set_is_enabled(false);
        self.close_overlay.set_is_visible(false);
        self.show_error("");
    }
}

impl Drop for PreviewPrototypePlayerView {
    fn drop(&mut self) {
        self.frame_retired.get_mut().set(true);
        for layer in self.layers.get_mut() {
            layer.retired.set(true);
        }
    }
}

fn contains(root: &Control, element: &InputElement) -> bool {
    root.to_ref().upcast::<InputElement>() == element.to_ref()
        || root.is_visual_ancestor_of(element)
        || root.is_logical_ancestor_of(Some(element))
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

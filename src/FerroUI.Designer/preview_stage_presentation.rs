//! Card camera and resize feedback. These native transforms and scroll offsets do
//! not alter a stage configuration, accepted application graph or authored source.
use super::resize::ResizeGrip;
use crate::{
    PREVIEW_STAGE_ZOOM_LEVELS, PreviewStageCamera, PreviewStageConfiguration, PreviewStageInfo,
    PreviewStageLayout, PreviewStageSize, PreviewTheme,
};
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{SolidColorBrush, Stretch, StretchDirection};
use ferroui_base::reactive::Subscription;
use ferroui_base::{Point, Ref, Size, Thickness, Vector};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Border, Canvas, ColumnDefinitions, ComboBox, Control, Grid, ScrollViewer, TextBlock, Viewbox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(super) struct CardPresentation {
    pub(super) frame: Ref<Grid>,
    pub(super) content: Ref<Grid>,
    pub(super) toolbar: Ref<Grid>,
    pub(super) grip: Rc<ResizeGrip>,
    host: Ref<Control>,
    viewbox: Ref<Viewbox>,
    scroll: Ref<ScrollViewer>,
    zoom_picker: Ref<ComboBox>,
    camera: Rc<Cell<PreviewStageCamera>>,
    restore_offset: Cell<bool>,
    size: Cell<PreviewStageSize>,
    configuration: PreviewStageConfiguration,
    details: Ref<TextBlock>,
    draft_note: Ref<TextBlock>,
    ghost: Ref<Border>,
    size_subscription: Cell<Option<RoutedEventHandlerToken>>,
    camera_layout: RefCell<Option<Subscription>>,
}

impl CardPresentation {
    pub(super) fn new(
        info: &PreviewStageInfo,
        host: Ref<Control>,
        details: Ref<TextBlock>,
        camera: Rc<Cell<PreviewStageCamera>>,
    ) -> Rc<Self> {
        let frame = Grid::new();
        frame.set_name(Some(format!("Stage{}PreviewFrame", info.id)));
        frame.set_height(360.0);
        frame.set_clip_to_bounds(true);
        frame.set_margin(Thickness::new(10.0, 0.0, 10.0, 0.0));
        let content = Grid::new();
        let viewbox = Viewbox::new();
        viewbox.set_name(Some(format!("Stage{}Viewbox", info.id)));
        viewbox.set_child(Some(host.clone()));
        content.children().add(viewbox.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_name(Some(format!("Stage{}Viewport", info.id)));
        scroll.set_bring_into_view_on_focus_change(false);
        scroll.set_content(Some(Control::boxed(content.clone().upcast::<Control>())));
        frame.children().add(scroll.clone().upcast::<Control>());

        let feedback = Canvas::new();
        feedback.set_is_hit_test_visible(false);
        let ghost = Border::new();
        ghost.set_name(Some(format!("Stage{}ResizeDraft", info.id)));
        ghost.set_border_brush(Some(SolidColorBrush::parse("#60A5FA").unwrap().into()));
        ghost.set_border_thickness(Thickness::uniform(2.0));
        ghost.set_background(Some(SolidColorBrush::parse("#1660A5FA").unwrap().into()));
        ghost.set_is_visible(false);
        feedback.children().add(ghost.clone().upcast::<Control>());
        frame.children().add(feedback.upcast::<Control>());

        let toolbar = Grid::new();
        toolbar.set_name(Some(format!("Stage{}Presentation", info.id)));
        toolbar.set_column_definitions(ColumnDefinitions::parse("Auto,96,*,Auto").unwrap());
        toolbar.set_column_spacing(7.0);
        let label = super::text("View");
        label.set_vertical_alignment(VerticalAlignment::Center);
        label.classes().add("designer-muted");
        toolbar.children().add(label.upcast::<Control>());
        let zoom_picker = ComboBox::new();
        zoom_picker.set_name(Some(format!("Stage{}Zoom", info.id)));
        zoom_picker.set_horizontal_alignment(HorizontalAlignment::Stretch);
        AutomationProperties::set_name(&zoom_picker, Some("Preview presentation zoom"));
        zoom_picker.items().add(Some(Rc::new("Fit".to_owned())));
        for zoom in PREVIEW_STAGE_ZOOM_LEVELS {
            zoom_picker.items().add(Some(Rc::new(format!("{zoom}%"))));
        }
        zoom_picker.set_selected_index(zoom_index(camera.get()));
        Grid::set_column(&zoom_picker, 1);
        toolbar
            .children()
            .add(zoom_picker.clone().upcast::<Control>());
        let draft_note = super::text("Drag corner to resize");
        draft_note.set_name(Some(format!("Stage{}ResizeNote", info.id)));
        draft_note.set_vertical_alignment(VerticalAlignment::Center);
        draft_note.set_font_size(11.0);
        draft_note.classes().add("designer-muted");
        Grid::set_column(&draft_note, 2);
        toolbar
            .children()
            .add(draft_note.clone().upcast::<Control>());
        let grip = ResizeGrip::new(&host, &format!("Stage{}ResizeGrip", info.id));
        Grid::set_column(&grip.control(), 3);
        toolbar.children().add(grip.control());

        let presentation = Rc::new(Self {
            frame,
            content,
            toolbar,
            grip,
            host,
            viewbox,
            scroll,
            zoom_picker,
            camera,
            restore_offset: Cell::new(true),
            size: Cell::new(info.layout_size),
            configuration: info.configuration.clone(),
            details,
            draft_note,
            ghost,
            size_subscription: Cell::new(None),
            camera_layout: RefCell::new(None),
        });
        presentation.apply_zoom();
        presentation.update_details();
        let weak = Rc::downgrade(&presentation);
        presentation.zoom_picker.selection_changed(move |_, _| {
            let Some(presentation) = weak.upgrade() else {
                return;
            };
            let Ok(index) = usize::try_from(presentation.zoom_picker.selected_index()) else {
                return;
            };
            let zoom_percent = index.checked_sub(1).and_then(|index| PREVIEW_STAGE_ZOOM_LEVELS.get(index)).copied();
            if zoom_percent != presentation.camera.get().zoom_percent {
                presentation.set_camera(PreviewStageCamera { zoom_percent, ..Default::default() });
            }
        });
        let weak = Rc::downgrade(&presentation);
        let subscription = presentation.host.size_changed(move |_, event| {
            if let Some(presentation) = weak.upgrade() {
                let size = event.new_size();
                presentation.update_size(PreviewStageSize {
                    width: size.width,
                    height: size.height,
                });
            }
        });
        presentation.size_subscription.set(Some(subscription));
        let weak = Rc::downgrade(&presentation);
        presentation.frame.size_changed(move |_, _| {
            if let Some(presentation) = weak.upgrade() {
                presentation.cancel_interaction();
            }
        });
        let weak = Rc::downgrade(&presentation);
        presentation.scroll.scroll_changed(move |_, _| {
            if let Some(presentation) = weak.upgrade() {
                presentation.cancel_interaction();
                if !presentation.restore_offset.get() {
                    presentation.remember_offset();
                }
            }
        });
        let weak = Rc::downgrade(&presentation);
        *presentation.camera_layout.borrow_mut() = Some(presentation.frame.layout_updated(move || {
            if let Some(presentation) = weak.upgrade() {
                presentation.settle_offset();
            }
        }));
        let weak = Rc::downgrade(&presentation);
        presentation.grip.on_draft(move |draft| {
            if let Some(presentation) = weak.upgrade() {
                presentation.show_draft(draft);
            }
        });
        presentation
    }

    pub(super) fn update_size(&self, size: PreviewStageSize) {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return;
        }
        if self.size.replace(size) != size {
            self.cancel_interaction();
            self.apply_zoom();
            self.update_details();
        }
    }

    pub(super) fn set_camera(&self, camera: PreviewStageCamera) {
        let previous = self.camera.replace(camera);
        if previous == camera {
            return;
        }
        self.cancel_interaction();
        self.restore_offset.set(true);
        if previous.zoom_percent != camera.zoom_percent {
            self.zoom_picker.set_selected_index(zoom_index(camera));
            self.apply_zoom();
        }
        self.frame.update_layout();
        self.settle_offset();
    }

    fn settle_offset(&self) {
        if !self.restore_offset.get() {
            return;
        }
        let camera = self.camera.get();
        let viewport = self.scroll.viewport();
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            return;
        }
        self.scroll.set_offset(Vector::new(camera.x, camera.y));
        self.restore_offset.set(false);
        self.remember_offset();
    }

    fn remember_offset(&self) {
        let mut camera = self.camera.get();
        if camera.zoom_percent.is_some() {
            let offset = self.scroll.offset();
            camera.x = offset.x;
            camera.y = offset.y;
            self.camera.set(camera);
        }
    }

    fn apply_zoom(&self) {
        if let Some(zoom_percent) = self.camera.get().zoom_percent {
            let zoom = f64::from(zoom_percent) / 100.0;
            let size = self.size.get();
            self.scroll
                .set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
            self.scroll
                .set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
            self.viewbox.set_stretch(Stretch::Uniform);
            self.viewbox.set_stretch_direction(StretchDirection::Both);
            self.viewbox
                .set_horizontal_alignment(HorizontalAlignment::Left);
            self.viewbox.set_vertical_alignment(VerticalAlignment::Top);
            self.viewbox.set_width(size.width * zoom);
            self.viewbox.set_height(size.height * zoom);
        } else {
            self.scroll
                .set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
            self.scroll
                .set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
            self.viewbox.set_width(f64::NAN);
            self.viewbox.set_height(f64::NAN);
            self.viewbox.set_stretch(Stretch::Uniform);
            self.viewbox
                .set_stretch_direction(StretchDirection::DownOnly);
            self.viewbox
                .set_horizontal_alignment(HorizontalAlignment::Center);
            self.viewbox
                .set_vertical_alignment(VerticalAlignment::Center);
        }
    }

    fn update_details(&self) {
        let size = self.size.get();
        let pointer = self.configuration.pointer_input;
        let pointer_label = if pointer.is_default() { String::new() } else { format!(" · {} input", pointer.label()) };
        self.details.set_text(Some(&format!(
            "{} · {} × {} · {} · {}{pointer_label}{}",
            super::short_uri(&self.configuration.source_uri),
            number(size.width),
            number(size.height),
            match self.configuration.layout {
                PreviewStageLayout::Fixed => "Fixed",
                PreviewStageLayout::SizeThatFits => "Fit content",
                PreviewStageLayout::FixedWidth => "Fit height",
                PreviewStageLayout::FixedHeight => "Fit width",
            },
            match self.configuration.environment.theme {
                PreviewTheme::Light => "Light",
                PreviewTheme::Dark => "Dark",
                PreviewTheme::Inherit => "App theme",
                PreviewTheme::HighContrast => "High contrast",
            },
            if self.configuration.pinned {
                " · Pinned"
            } else {
                ""
            },
        )));
    }

    fn show_draft(&self, size: Option<Size>) {
        let Some(size) = size else {
            self.ghost.set_is_visible(false);
            self.draft_note.set_text(Some("Drag corner to resize"));
            return;
        };
        self.draft_note.set_text(Some(&format!(
            "{} × {} · release to apply",
            number(size.width),
            number(size.height)
        )));
        if let Some(transform) = self.host.transform_to_visual(&self.frame) {
            let start = Point::default().transform(transform);
            let end = Point::new(size.width, size.height).transform(transform);
            Canvas::set_left(&self.ghost, start.x.min(end.x));
            Canvas::set_top(&self.ghost, start.y.min(end.y));
            self.ghost.set_width((end.x - start.x).abs());
            self.ghost.set_height((end.y - start.y).abs());
            self.ghost.set_is_visible(true);
        }
    }

    pub(super) fn cancel_interaction(&self) {
        self.grip.cancel();
    }

    pub(super) fn retire(&self) {
        self.detach_size_subscription();
        self.camera_layout.borrow_mut().take();
        self.grip.retire();
        self.viewbox.set_child(None::<Ref<Control>>);
    }

    fn detach_size_subscription(&self) {
        if let Some(subscription) = self.size_subscription.take() {
            self.host
                .remove_handler(Control::size_changed_event(), subscription);
        }
    }
}

impl Drop for CardPresentation {
    fn drop(&mut self) {
        self.detach_size_subscription();
    }
}

fn zoom_index(camera: PreviewStageCamera) -> i32 {
    camera.zoom_percent.and_then(|zoom| PREVIEW_STAGE_ZOOM_LEVELS.iter().position(|level| *level == zoom))
        .map_or(0, |index| index as i32 + 1)
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        value.to_string()
    } else {
        format!("{value:.1}")
    }
}

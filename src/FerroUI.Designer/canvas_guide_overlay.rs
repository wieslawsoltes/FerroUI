//! Native viewport overlay. Rulers and guide strokes retain screen size at every zoom.
use crate::{CanvasGuide, CanvasGuideAxis, CanvasGuideSettings, CanvasViewport};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush,
    Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    FerroObjectImpl, Matrix, Point, Rect, Ref, Size, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) const RULER_SIZE: f64 = 24.0;

#[repr(C)]
pub(crate) struct CanvasGuideOverlay {
    base: Control,
    settings: RefCell<CanvasGuideSettings>,
    viewport: Cell<CanvasViewport>,
    artboard: Cell<Size>,
    selected: Cell<Option<u64>>,
    draft: Cell<Option<CanvasGuide>>,
}
ferro_class!(CanvasGuideOverlay: Control);
ferro_class_info!(CanvasGuideOverlay {
    new: CanvasGuideOverlay::new
});
ferro_impl_classes!(CanvasGuideOverlay: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl CanvasGuideOverlay {
    pub(crate) fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            settings: RefCell::new(CanvasGuideSettings::default()),
            viewport: Cell::new(CanvasViewport::default()),
            artboard: Cell::new(Size::new(800.0, 520.0)),
            selected: Cell::new(None),
            draft: Cell::new(None),
        });
        this.set_is_hit_test_visible(false);
        this.set_name(Some("CanvasGuideOverlay".into()));
        this
    }

    pub(crate) fn configure(
        &self,
        settings: &CanvasGuideSettings,
        viewport: CanvasViewport,
        artboard: Size,
        selected: Option<u64>,
        draft: Option<CanvasGuide>,
    ) {
        if *self.settings.borrow() == *settings
            && self.viewport.get() == viewport
            && self.artboard.get() == artboard
            && self.selected.get() == selected
            && self.draft.get() == draft
        {
            return;
        }
        *self.settings.borrow_mut() = settings.clone();
        self.viewport.set(viewport);
        self.artboard.set(artboard);
        self.selected.set(selected);
        self.draft.set(draft);
        self.invalidate_visual();
    }
}

impl VisualImpl for CanvasGuideOverlay {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = Rect::from_size(this.bounds().size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }
        let clip = context.push_clip(bounds);
        let settings = this.settings.borrow();
        let viewport = this.viewport.get();
        let offset = viewport.offset();
        let zoom = viewport.zoom();
        let inset = if settings.rulers_visible {
            RULER_SIZE
        } else {
            0.0
        };
        if settings.grid_visible {
            let artboard = this.artboard.get();
            let area = Rect::new(offset.x.max(inset), offset.y.max(inset), 0.0, 0.0);
            let right = (offset.x + artboard.width * zoom).min(bounds.width);
            let bottom = (offset.y + artboard.height * zoom).min(bounds.height);
            if right > area.x && bottom > area.y {
                let step =
                    settings.grid_spacing * (8.0 / (settings.grid_spacing * zoom)).ceil().max(1.0);
                let grid: Rc<dyn IPen> = Pen::from_uint32(0x336b88aa, 1.0).into();
                for vertical in [true, false] {
                    let (start, end, origin) = if vertical {
                        (area.x, right, offset.x)
                    } else {
                        (area.y, bottom, offset.y)
                    };
                    let first = ((start - origin) / zoom / step).ceil();
                    let count =
                        ((end - start) / zoom / step).ceil().clamp(0.0, 4096.0) as usize + 1;
                    for index in 0..count {
                        let pixel = (first + index as f64) * step * zoom + origin;
                        if pixel < start || pixel > end {
                            continue;
                        }
                        if vertical {
                            context.draw_line(
                                &grid,
                                Point::new(pixel, area.y),
                                Point::new(pixel, bottom),
                            );
                        } else {
                            context.draw_line(
                                &grid,
                                Point::new(area.x, pixel),
                                Point::new(right, pixel),
                            );
                        }
                    }
                }
            }
        }
        let draft = this.draft.get();
        if settings.guides_visible || draft.is_some() {
            for guide in settings
                .guides
                .iter()
                .copied()
                .filter(|guide| {
                    settings.guides_visible && draft.is_none_or(|draft| draft.id != guide.id)
                })
                .chain(draft)
            {
                let active = this.selected.get() == Some(guide.id)
                    || draft.is_some_and(|draft| draft.id == guide.id);
                let color = if active { 0xffffc467 } else { 0xff36b9d1 };
                let pen: Rc<dyn IPen> =
                    Pen::from_uint32(color, if active { 1.5 } else { 1.0 }).into();
                let pixel = match guide.axis {
                    CanvasGuideAxis::Vertical => guide.position * zoom + offset.x,
                    CanvasGuideAxis::Horizontal => guide.position * zoom + offset.y,
                };
                let visible = match guide.axis {
                    CanvasGuideAxis::Vertical => pixel >= inset && pixel <= bounds.width,
                    CanvasGuideAxis::Horizontal => pixel >= inset && pixel <= bounds.height,
                };
                if !visible {
                    continue;
                }
                match guide.axis {
                    CanvasGuideAxis::Vertical => context.draw_line(
                        &pen,
                        Point::new(pixel, inset),
                        Point::new(pixel, bounds.height),
                    ),
                    CanvasGuideAxis::Horizontal => context.draw_line(
                        &pen,
                        Point::new(inset, pixel),
                        Point::new(bounds.width, pixel),
                    ),
                }
                if active {
                    let caption = format!(
                        "{} {:.2}",
                        if guide.axis == CanvasGuideAxis::Vertical {
                            "X"
                        } else {
                            "Y"
                        },
                        guide.position
                    );
                    let text = text(&caption, color);
                    let origin = match guide.axis {
                        CanvasGuideAxis::Vertical => Point::new(
                            (pixel + 5.0).min((bounds.width - text.width() - 5.0).max(inset)),
                            inset + 4.0,
                        ),
                        CanvasGuideAxis::Horizontal => Point::new(
                            inset + 5.0,
                            (pixel + 4.0).min((bounds.height - 17.0).max(inset)),
                        ),
                    };
                    let backdrop = brush(0xe6232932);
                    context.draw_rectangle(
                        Some(&backdrop),
                        None,
                        Rect::new(origin.x - 2.0, origin.y - 1.0, text.width() + 4.0, 15.0),
                        2.0,
                        2.0,
                        &BoxShadows::default(),
                    );
                    context.draw_text(&text, origin);
                }
            }
        }
        if settings.rulers_visible {
            let light =
                this.actual_theme_variant() == Some(ferroui_base::styling::ThemeVariant::light());
            let background = brush(if light { 0xffe9edf2 } else { 0xff252a33 });
            let text_color = if light { 0xff394451 } else { 0xffc0cbd8 };
            let pen: Rc<dyn IPen> =
                Pen::from_uint32(if light { 0xff9ba8b8 } else { 0xff5b697c }, 1.0).into();
            context.draw_rectangle(
                Some(&background),
                None,
                Rect::new(0.0, 0.0, bounds.width, RULER_SIZE),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            context.draw_rectangle(
                Some(&background),
                None,
                Rect::new(0.0, 0.0, RULER_SIZE, bounds.height),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            for vertical in [false, true] {
                let (offset, length) = if vertical {
                    (offset.y, bounds.height)
                } else {
                    (offset.x, bounds.width)
                };
                for (pixel, value, major) in crate::canvas_guides::ruler_ticks(offset, zoom, length)
                {
                    if pixel < RULER_SIZE {
                        continue;
                    }
                    let tick = if major { 8.0 } else { 4.0 };
                    if vertical {
                        context.draw_line(
                            &pen,
                            Point::new(RULER_SIZE - tick, pixel),
                            Point::new(RULER_SIZE, pixel),
                        );
                    } else {
                        context.draw_line(
                            &pen,
                            Point::new(pixel, RULER_SIZE - tick),
                            Point::new(pixel, RULER_SIZE),
                        );
                    }
                    if major {
                        let label = if value.abs() < 1e-8 {
                            "0".to_owned()
                        } else if value.abs() < 1.0 {
                            format!("{value:.2}")
                        } else {
                            format!("{value:.0}")
                        };
                        let text = text(&label, text_color);
                        if vertical {
                            let pivot = Point::new(3.0, pixel - 3.0);
                            let transform = context.push_transform(Matrix::create_rotation_at(
                                -std::f64::consts::FRAC_PI_2,
                                pivot,
                            ));
                            context.draw_text(&text, pivot);
                            context.pop(transform);
                        } else {
                            context.draw_text(&text, Point::new(pixel + 3.0, 2.0));
                        }
                    }
                }
            }
            context.draw_rectangle(
                Some(&background),
                None,
                Rect::new(0.0, 0.0, RULER_SIZE, RULER_SIZE),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            context.draw_text(&text("px", text_color), Point::new(5.0, 5.0));
            context.draw_line(
                &pen,
                Point::new(RULER_SIZE, 0.0),
                Point::new(RULER_SIZE, bounds.height),
            );
            context.draw_line(
                &pen,
                Point::new(0.0, RULER_SIZE),
                Point::new(bounds.width, RULER_SIZE),
            );
        }
        context.pop(clip);
    }
}

fn brush(color: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(color).into()
}
fn text(value: &str, color: u32) -> FormattedText {
    FormattedText::new(
        value,
        CultureInfo::current_culture(),
        FlowDirection::LeftToRight,
        Typeface::default(),
        10.0,
        Some(brush(color)),
    )
}

// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_settings::DockProperties;
use ferroui_base::{
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{
        immutable::ImmutableSolidColorBrush, BoxShadows, Brushes, Color, DrawingContext,
        FlowDirection, FormattedText, IBrush, IPen, Pen, Typeface,
    },
    utilities::CultureInfo,
    FerroObjectImpl, FerroObjectImplExt, Point, Rect, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt,
};
use ferroui_controls::{primitives::AdornerLayer, Control, ControlImpl};
use std::{cell::RefCell, rc::Rc};

/// Original diagnostic drawing of dock targets, drag areas and drop areas.
#[repr(C)]
pub(crate) struct DebugOverlayAdorner {
    base: Control,
    pointer_over: RefCell<Option<Ref<Control>>>,
}
ferro_class!(DebugOverlayAdorner: Control);
ferro_impl_classes!(DebugOverlayAdorner: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(DebugOverlayAdorner {
    new: DebugOverlayAdorner::new,
    markup: { methods: [fn SetPointerOver(Option<Ref<Control>>) => |this: &Ref<DebugOverlayAdorner>, control| this.set_pointer_over(control)] },
});
impl FerroObjectImpl for DebugOverlayAdorner {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_hit_test_visible(false);
    }
}
impl VisualImpl for DebugOverlayAdorner {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        Self::parent_render(this, context);
        let Some(root) = AdornerLayer::get_adorned_element(this) else {
            return;
        };
        for visual in root.get_visual_descendants() {
            let Some(control) = visual.cast::<Control>() else {
                continue;
            };
            let Some(position) = control.translate_point(Point::default(), &root) else {
                continue;
            };
            let rect = Rect::from_position_size(position, control.bounds().size());
            if DockProperties::get_is_dock_target(&control) {
                this.draw_area(context, &control, rect, 0);
            }
            if DockProperties::get_is_drag_area(&control) {
                this.draw_area(context, &control, rect, 1);
            }
            if DockProperties::get_is_drop_area(&control) {
                this.draw_area(context, &control, rect, 2);
            }
        }
        let hovered = this.pointer_over.borrow().clone();
        if let Some(hovered) = hovered {
            let text = hovered
                .data_context()
                .as_ref()
                .map(|value| ValueTypes::to_display_string(Some(value)));
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                let formatted = FormattedText::new(
                    &text,
                    CultureInfo::invariant_culture(),
                    FlowDirection::LeftToRight,
                    Typeface::default(),
                    12.0,
                    Some(Brushes::white()),
                );
                let origin = Point::new(
                    this.bounds().width - formatted.width() - 4.0,
                    this.bounds().height - formatted.height() - 4.0,
                );
                context.draw_text(&formatted, origin);
            }
        }
    }
}
struct AreaStyle {
    pen: Rc<dyn IPen>,
    fill: Rc<dyn IBrush>,
}
ferroui_base::ferro_thread_local! {
    static AREA_STYLES: [AreaStyle; 3] = [
        AreaStyle { pen: Pen::with_brush(Some(Brushes::red()), 1.0).into(), fill: Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(80, 255, 0, 0))) },
        AreaStyle { pen: Pen::with_brush(Some(Brushes::green()), 1.0).into(), fill: Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(80, 0, 255, 0))) },
        AreaStyle { pen: Pen::with_brush(Some(Brushes::blue()), 1.0).into(), fill: Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(80, 0, 0, 255))) },
    ];
}
impl DebugOverlayAdorner {
    pub(crate) fn construct() -> Self {
        Self {
            base: Control::construct(),
            pointer_over: RefCell::new(None),
        }
    }
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_pointer_over(&self, control: Option<Ref<Control>>) {
        if *self.pointer_over.borrow() != control {
            self.pointer_over.replace(control);
            self.invalidate_visual();
        }
    }
    fn draw_area(
        &self,
        context: &mut DrawingContext<'_>,
        control: &Ref<Control>,
        rect: Rect,
        index: usize,
    ) {
        let hovered = self.pointer_over.borrow().as_ref() == Some(control);
        AREA_STYLES.with(|styles| {
            let style = &styles[index];
            context.draw_rectangle(
                if hovered { Some(&style.fill) } else { None },
                Some(&style.pen),
                rect,
                0.0,
                0.0,
                &BoxShadows::default(),
            );
        });
    }
}

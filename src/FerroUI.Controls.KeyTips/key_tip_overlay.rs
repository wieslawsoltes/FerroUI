//! The Key Tips of one window, drawn over its content through the drawing context.

use crate::access_key_manager::AccessKeyManager;
use crate::key_tip_math::{place_tip, KeyTipPlacementMode};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    BoxShadows, Color, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Typeface,
};
use ferroui_base::utilities::Locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect,
    Ref, Size, StyledElementImpl, VisualImpl, WeakRef,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;

const TEXT_SIZE: f64 = 12.0;
const PADDING_X: f64 = 5.0;
const PADDING_Y: f64 = 1.0;
const CORNER: f64 = 3.0;

/// A tip to show: its element and the text of its key.
#[derive(Clone)]
pub(crate) struct ShownTip {
    pub target: WeakRef<Control>,
    pub text: String,
}

/// Draws the tips of one window. It takes no input.
#[repr(C)]
pub struct KeyTipOverlay {
    base: Control,
    tips: RefCell<Vec<ShownTip>>,
}

ferro_class!(KeyTipOverlay: Control);
ferro_class_info!(KeyTipOverlay {
    new: KeyTipOverlay::new
});
ferro_impl_classes!(
    KeyTipOverlay: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl KeyTipOverlay {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            tips: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_is_hit_test_visible(false);
        this
    }

    pub(crate) fn set_tips(&self, tips: Vec<ShownTip>) {
        *self.tips.borrow_mut() = tips;
        self.invalidate_visual();
    }

    #[cfg(test)]
    pub(crate) fn tip_texts(&self) -> Vec<String> {
        self.tips
            .borrow()
            .iter()
            .map(|tip| tip.text.clone())
            .collect()
    }
}

fn solid(color: Color) -> Rc<ImmutableSolidColorBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

impl VisualImpl for KeyTipOverlay {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        let bounds = this.bounds().size();
        let tips = this.tips.borrow();
        if tips.is_empty() {
            return;
        }
        // The colors of a tooltip with high contrast, on any theme.
        let fill: Rc<dyn IBrush> = solid(Color::from_rgb(0x20, 0x20, 0x20));
        let text_brush: Rc<dyn IBrush> = solid(Color::from_rgb(0xFF, 0xFF, 0xFF));
        let outline: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(
            Some(solid(Color::from_rgb(0xFF, 0xFF, 0xFF))),
            1.0,
        ));
        for tip in tips.iter() {
            let Some(target) = tip.target.upgrade() else {
                continue;
            };
            let mode = AccessKeyManager::get_key_tip_placement_mode(&target);
            if mode == KeyTipPlacementMode::Hidden {
                continue;
            }
            let Some(transform) = target.transform_to_visual(this) else {
                continue;
            };
            let rect = Rect::from_size(target.bounds().size()).transform_to_aabb(transform);
            let text = FormattedText::new(
                &tip.text,
                Locale::UNKNOWN,
                FlowDirection::LeftToRight,
                Typeface::default_typeface(),
                TEXT_SIZE,
                Some(text_brush.clone()),
            );
            let size = Size::new(
                text.width() + PADDING_X * 2.0,
                text.height() + PADDING_Y * 2.0,
            );
            let at = place_tip(
                rect,
                size,
                mode,
                AccessKeyManager::get_key_tip_horizontal_offset(&target),
                AccessKeyManager::get_key_tip_vertical_offset(&target),
                bounds,
            );
            context.draw_rectangle(
                Some(&fill),
                Some(&outline),
                Rect::new(at.x, at.y, size.width, size.height),
                CORNER,
                CORNER,
                &BoxShadows::default(),
            );
            context.draw_text(&text, Point::new(at.x + PADDING_X, at.y + PADDING_Y));
        }
    }
}

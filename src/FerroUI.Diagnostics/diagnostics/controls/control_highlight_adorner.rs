//! Port of `Diagnostics/Controls/ControlHighlightAdorner.cs`.

use ferroui_base::input::{InputElementImpl, InputElement};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{Color, Colors, DrawingContext, IBrush, IPen, Pen, SolidColorBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    Thickness, Visual, VisualImpl, VisualImplExt,
};
use ferroui_controls::primitives::{AdornerLayer, TemplatedControl};
use ferroui_controls::{Border, Control, ControlImpl, Panel};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    // `_layoutHighlightAdorner`: the frames of the padding, the content and the margin of the
    // highlighted control (one per thread; the objects of the port are not shared between
    // threads).
    static LAYOUT_HIGHLIGHT_ADORNER: Ref<Panel> = ControlHighlightAdorner::create_layout_highlight_adorner();
}

/// The adorner that highlights a control: a frame drawn with a pen around the control, or the
/// frames of its padding, content and margin.
#[repr(C)]
pub struct ControlHighlightAdorner {
    base: Control,
    pen: Rc<dyn IPen>,
}

ferro_class!(ControlHighlightAdorner: Control);
ferro_impl_classes!(
    ControlHighlightAdorner: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(ControlHighlightAdorner {});

impl VisualImpl for ControlHighlightAdorner {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);
        context.draw_rectangle_outline(&this.pen, this.bounds().deflate(2.0), 0.0);
    }
}

impl ControlHighlightAdorner {
    /// The static constructor of the original: creates the shared adorner of the layout.
    fn create_layout_highlight_adorner() -> Ref<Panel> {
        let panel = Panel::new();
        panel.set_clip_to_bounds(false);

        //Padding frame
        let padding_frame = Border::new();
        padding_frame.set_border_brush(Some(SolidColorBrush::with_color_and_opacity(Colors::GREEN, 0.5).into()));
        panel.children().add(padding_frame);

        //Content frame
        let content_frame = Border::new();
        content_frame.set_background(Some(
            SolidColorBrush::with_color_and_opacity(Color::from_rgb(160, 197, 232), 0.5).into(),
        ));
        panel.children().add(content_frame);

        //Margin frame
        let margin_frame = Border::new();
        margin_frame.set_border_brush(Some(SolidColorBrush::with_color_and_opacity(Colors::YELLOW, 0.5).into()));
        panel.children().add(margin_frame);

        AdornerLayer::set_is_clip_enabled(&panel, false);
        panel
    }

    fn construct(pen: Rc<dyn IPen>) -> Self {
        Self { base: Control::construct(), pen }
    }

    /// The private constructor of the original.
    fn new(pen: Rc<dyn IPen>) -> Ref<Self> {
        let this = instantiate(Self::construct(pen));
        this.set_value(Visual::clip_property(), None);
        this
    }

    /// Highlights `owner` with a frame drawn with `highlight_brush`, in the adorner layer of
    /// `owner`. Disposing the result removes the frame; `None` when `owner` has no adorner layer.
    pub fn add(owner: &InputElement, highlight_brush: Rc<dyn IBrush>) -> Option<Rc<dyn IDisposable>> {
        let layer = AdornerLayer::get_adorner_layer(owner)?;

        let pen: Rc<dyn IPen> = Rc::new(Pen::with_brush(Some(highlight_brush), 2.0).to_immutable());
        let adorner = ControlHighlightAdorner::new(pen);
        AdornerLayer::set_adorned_element(&adorner, owner.to_ref());
        layer.children().add(adorner.clone());

        Some(Disposable::create_with_state((layer, adorner), |(layer, adorner)| {
            layer.children().remove(adorner);
        }))
    }

    /// C# `Add(Visual visual, bool visualizeMarginPadding)`: shows the layout of `visual` (the
    /// frames of its padding, content and margin when `visualize_margin_padding` is set) in its
    /// adorner layer. Disposing the result removes the frames; `None` when `visual` has no
    /// adorner layer or its layout is already shown.
    pub fn add_visual(visual: &Visual, visualize_margin_padding: bool) -> Option<Rc<dyn IDisposable>> {
        let layer = AdornerLayer::get_adorner_layer(visual)?;
        let layout_highlight_adorner = LAYOUT_HIGHLIGHT_ADORNER.with(Ref::clone);

        if layer.children().contains(&layout_highlight_adorner.clone().upcast()) {
            return None;
        }
        layer.children().add(layout_highlight_adorner.clone());
        AdornerLayer::set_adorned_element(&layout_highlight_adorner, visual.to_ref());
        let children = layout_highlight_adorner.children();
        let border = |index: usize| children.get(index).cast::<Border>().expect("the frames of the layout are borders");
        let padding_border = border(0);
        let content_border = border(1);
        let margin_border = border(2);
        if visualize_margin_padding {
            padding_border.set_border_thickness(visual.get_value(TemplatedControl::padding_property()));
            content_border.set_margin(visual.get_value(TemplatedControl::padding_property()));
            margin_border.set_border_thickness(visual.get_value(Layoutable::margin_property()));
            margin_border.set_margin(Self::invert_thickness(visual.get_value(Layoutable::margin_property())));
        } else {
            padding_border.set_border_thickness(Thickness::default());
            content_border.set_margin(Thickness::default());
            margin_border.set_border_thickness(Thickness::default());
            margin_border.set_margin(Thickness::default());
        }

        Some(Disposable::create_with_state((layer, layout_highlight_adorner), |(layer, adorner)| {
            layer.children().remove(adorner);
        }))
    }

    fn invert_thickness(input: Thickness) -> Thickness {
        Thickness::new(-input.left, -input.top, -input.right, -input.bottom)
    }
}

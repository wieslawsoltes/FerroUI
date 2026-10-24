//! Port of `Diagnostics/Views/LayoutExplorerView.xaml.cs`: the class of the document
//! `LayoutExplorerView.xaml`.

use crate::diagnostics::controls::ThicknessEditor;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroPropertyChangedEventArgs,
    Point, Rect, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, Canvas, ContentControlImpl, ControlImpl, Grid, UserControl};

/// The layout explorer: the margin, the border and the padding of a control as nested editors
/// around its content, its size with guidelines from the content to the size labels, and its
/// alignments.
#[repr(C)]
pub struct LayoutExplorerView {
    base: UserControl,
}

ferro_class!(LayoutExplorerView: UserControl);
ferro_impl_classes!(
    LayoutExplorerView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(LayoutExplorerView { new: LayoutExplorerView::new });

/// The named elements of the document the guidelines are computed from.
struct LayoutParts {
    border_area: Ref<ThicknessEditor>,
    padding_area: Ref<ThicknessEditor>,
    horizontal_size_begin: Ref<Rectangle>,
    horizontal_size_end: Ref<Rectangle>,
    vertical_size_begin: Ref<Rectangle>,
    vertical_size_end: Ref<Rectangle>,
    layout_root: Ref<Grid>,
    horizontal_size: Ref<Border>,
    vertical_size: Ref<Border>,
    content_area: Ref<Border>,
}

impl LayoutExplorerView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/LayoutExplorerView.xaml";

    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// # Panics
    /// When the document fails to load or lacks one of its named elements (an exception of the
    /// constructor in the managed original).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);

        let content_area = this.get_control::<Border>("ContentArea");

        let mut visual: Option<Ref<Visual>> = Some(content_area.upcast());
        while let Some(current) = visual {
            if current.ptr_eq(&this) {
                break;
            }
            let weak = this.downgrade();
            // The subscription lives as long as the element (never disposed, as in the managed
            // original).
            let _ = current.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if e.property() == Visual::bounds_property().as_property() {
                    if let Some(this) = weak.upgrade() {
                        this.update_size_guidelines();
                    }
                }
            });
            visual = current.visual_parent();
        }

        this
    }

    fn initialize_component(this: &Ref<Self>) {
        crate::register_types();
        if let Err(error) = crate::compiled_diagnostics_views_layout_explorer_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    fn parts(&self) -> LayoutParts {
        LayoutParts {
            border_area: self.get_control::<ThicknessEditor>("BorderArea"),
            padding_area: self.get_control::<ThicknessEditor>("PaddingArea"),
            horizontal_size_begin: self.get_control::<Rectangle>("HorizontalSizeBegin"),
            horizontal_size_end: self.get_control::<Rectangle>("HorizontalSizeEnd"),
            vertical_size_begin: self.get_control::<Rectangle>("VerticalSizeBegin"),
            vertical_size_end: self.get_control::<Rectangle>("VerticalSizeEnd"),
            horizontal_size: self.get_control::<Border>("HorizontalSize"),
            vertical_size: self.get_control::<Border>("VerticalSize"),
            content_area: self.get_control::<Border>("ContentArea"),
            layout_root: self.get_control::<Grid>("LayoutRoot"),
        }
    }

    fn update_size_guidelines(&self) {
        let parts = self.parts();

        if parts.border_area.is_present() {
            Self::update_guidelines(&parts, &parts.border_area);
        } else if parts.padding_area.is_present() {
            Self::update_guidelines(&parts, &parts.padding_area);
        } else {
            Self::update_guidelines(&parts, &parts.content_area);
        }
    }

    // That's what TransformedBounds.Bounds actually was. The original changed
    // `v.TransformedBounds.Bounds` to `GetPseudoTransformedBounds`.
    fn get_pseudo_transformed_bounds(v: &Visual) -> Rect {
        Rect::from_size(v.bounds().size())
    }

    fn translate_to_root(parts: &LayoutParts, point: Point, from: &Visual) -> Point {
        from.translate_point(point, &parts.layout_root).unwrap_or_default()
    }

    fn set_position(rect: &Rectangle, start: Point) {
        Canvas::set_left(rect, start.x);
        Canvas::set_top(rect, start.y);
    }

    fn update_guidelines(parts: &LayoutParts, area: &Visual) {
        let bounds = Self::get_pseudo_transformed_bounds(area);

        // Horizontal guideline
        {
            let size_area = Self::translate_to_root(
                parts,
                Self::get_pseudo_transformed_bounds(&parts.horizontal_size).bottom_left(),
                &parts.horizontal_size,
            );

            let start = Self::translate_to_root(parts, bounds.bottom_left(), area);

            Self::set_position(&parts.horizontal_size_begin, start);

            let end = Self::translate_to_root(parts, bounds.bottom_right(), area);

            Self::set_position(&parts.horizontal_size_end, end.with_x(end.x - 1.0));

            let height = size_area.y - start.y + 2.0;

            parts.horizontal_size_begin.set_height(height);
            parts.horizontal_size_end.set_height(height);
        }

        // Vertical guideline
        {
            let size_area = Self::translate_to_root(
                parts,
                Self::get_pseudo_transformed_bounds(&parts.vertical_size).top_right(),
                &parts.vertical_size,
            );

            let start = Self::translate_to_root(parts, bounds.top_right(), area);

            Self::set_position(&parts.vertical_size_begin, start);

            let end = Self::translate_to_root(parts, bounds.bottom_right(), area);

            Self::set_position(&parts.vertical_size_end, end.with_y(end.y - 1.0));

            let width = size_area.x - start.x + 2.0;

            parts.vertical_size_begin.set_width(width);
            parts.vertical_size_end.set_width(width);
        }
    }
}

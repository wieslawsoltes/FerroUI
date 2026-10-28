//! The stage is a native layout boundary. A Viewbox may measure it with infinity,
//! but application content always receives the configured finite logical proposal.
//! Desired-size invalidation follows the normal native measure/arrange pipeline;
//! no callbacks change dimensions during layout or reconstruct the application.
use super::{PreviewEnvironment, PreviewStageLayout, PreviewStageSize};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::{ControlImpl, Grid, PanelImpl};
use std::cell::Cell;

#[repr(C)]
pub(super) struct StageLayoutHost {
    base: Grid,
    layout: Cell<PreviewStageLayout>,
    proposal: Cell<PreviewStageSize>,
    resolved: Cell<PreviewStageSize>,
}

ferro_class!(StageLayoutHost: Grid);
ferro_impl_classes!(StageLayoutHost: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl LayoutableImpl for StageLayoutHost {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        let proposal = this.proposal.get();
        let measured = <Grid as LayoutableImpl>::measure_override(
            &this.base,
            Size::new(proposal.width, proposal.height),
        );
        assert!(
            measured.width.is_finite() && measured.height.is_finite(),
            "Native preview content returned a nonfinite desired size"
        );
        let layout = this.layout.get();
        let resolved = PreviewStageSize {
            width: if layout.fits_width() {
                measured.width.clamp(1.0, proposal.width)
            } else {
                proposal.width
            },
            height: if layout.fits_height() {
                measured.height.clamp(1.0, proposal.height)
            } else {
                proposal.height
            },
        };
        this.resolved.set(resolved);
        Size::new(resolved.width, resolved.height)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        <Grid as LayoutableImpl>::arrange_override(&this.base, final_size)
    }
}

impl StageLayoutHost {
    pub(super) fn new(layout: PreviewStageLayout, environment: PreviewEnvironment) -> Ref<Self> {
        let proposal = PreviewStageSize {
            width: environment.width,
            height: environment.height,
        };
        let result = instantiate(Self {
            base: Grid::construct(),
            layout: Cell::new(layout),
            proposal: Cell::new(proposal),
            resolved: Cell::new(proposal),
        });
        result.set_clip_to_bounds(true);
        result.set_horizontal_alignment(HorizontalAlignment::Left);
        result.set_vertical_alignment(VerticalAlignment::Top);
        result.configure(layout, environment);
        result
    }

    pub(super) fn configure(&self, layout: PreviewStageLayout, environment: PreviewEnvironment) {
        let proposal = PreviewStageSize {
            width: environment.width,
            height: environment.height,
        };
        self.layout.set(layout);
        self.proposal.set(proposal);
        self.set_width(if layout.fits_width() {
            f64::NAN
        } else {
            proposal.width
        });
        self.set_height(if layout.fits_height() {
            f64::NAN
        } else {
            proposal.height
        });
        self.set_min_width(1.0);
        self.set_min_height(1.0);
        self.set_max_width(proposal.width);
        self.set_max_height(proposal.height);
        self.invalidate_measure();
    }

    pub(super) fn layout_size(&self) -> PreviewStageSize {
        let bounds = self.bounds();
        if self.is_arrange_valid() && bounds.width > 0.0 && bounds.height > 0.0 {
            PreviewStageSize {
                width: bounds.width,
                height: bounds.height,
            }
        } else {
            self.resolved.get()
        }
    }
}

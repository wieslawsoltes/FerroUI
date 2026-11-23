//! Video surface specialization of the portable texture presenter. All imports,
//! resize, generation cancellation and GPU retirement stay in TexturePresenter.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::ControlImpl;
use ferroui_media_controls::TexturePresenter;
#[repr(C)]
pub struct MediaPlayerPresenter {
    base: TexturePresenter,
}
ferro_class!(MediaPlayerPresenter:TexturePresenter);
ferro_class_info!(MediaPlayerPresenter {
    new: MediaPlayerPresenter::new
});
ferro_impl_classes!(MediaPlayerPresenter:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl MediaPlayerPresenter {
    pub fn construct() -> Self {
        Self {
            base: TexturePresenter::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

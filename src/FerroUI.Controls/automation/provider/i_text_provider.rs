use super::{ITextRangeProvider, ProviderError};
use crate::automation::peers::AutomationPeer;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Point, Ref};
use std::rc::Rc;

/// The number of independent text selections supported by a text provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum SupportedTextSelection {
    None = 0,
    Single = 1,
    Multiple = 2,
}

/// A portable text-pattern contract. Text offsets and maximum lengths use UTF-16
/// code units. Points and rectangles use the visual root's coordinate space, as
/// [`AutomationPeer::get_bounding_rectangle`] does; platform bridges convert to
/// native screen coordinates at their boundary.
pub trait ITextProvider {
    fn peer(&self) -> Ref<AutomationPeer>;
    fn document_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn get_selection(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn get_visible_ranges(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn range_from_child(
        &self,
        child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn range_from_point(&self, point: Point) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError>;

    /// Observes content and formatting changes, including replacement of the document.
    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError>;
    /// Observes changes to either selection endpoint, including the insertion point.
    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError>;
}

impl PartialEq for dyn ITextProvider {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

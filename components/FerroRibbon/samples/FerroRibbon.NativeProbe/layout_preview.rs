use ferroribbon_core::layout::{
    RibbonAdaptiveLayout, RibbonAdaptiveLayoutResult, RibbonGroupLayoutInfo,
};

/// Application presentation service. No control or property-system dependencies.
pub trait LayoutPreview {
    fn compute(&self, width: f64) -> RibbonAdaptiveLayoutResult;
}
pub struct SourceLayoutPreview;
impl LayoutPreview for SourceLayoutPreview {
    fn compute(&self, width: f64) -> RibbonAdaptiveLayoutResult {
        let groups = [
            RibbonGroupLayoutInfo::new(vec![200., 140., 90., 45.]),
            RibbonGroupLayoutInfo::new(vec![180., 120., 80., 45.]),
            RibbonGroupLayoutInfo::new(vec![160., 110., 70., 45.]),
        ];
        RibbonAdaptiveLayout::compute_with_spacing(&groups, width, 6.)
    }
}

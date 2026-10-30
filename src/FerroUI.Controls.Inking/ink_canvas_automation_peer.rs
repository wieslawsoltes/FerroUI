use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use ferroui_controls::Control;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Rect, Ref, Visual};

/// Pane peer using FerroUI transforms and ancestor clips, independent of ink content.
#[repr(C)]
pub struct InkCanvasAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(InkCanvasAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(InkCanvasAutomationPeer {});
impl FerroObjectImpl for InkCanvasAutomationPeer {}
impl ControlAutomationPeerImpl for InkCanvasAutomationPeer {}
impl AutomationPeerImpl for InkCanvasAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkCanvas".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        this.visible_bounds()
    }
    fn is_offscreen_core(this: &Self) -> bool {
        let rect = this.visible_bounds();
        rect.width <= 0.0 || rect.height <= 0.0
    }
}
impl InkCanvasAutomationPeer {
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn visible_bounds(&self) -> Rect {
        let Some(owner) = self.try_owner() else {
            return Rect::default();
        };
        let Some(root) = owner.visual_root() else {
            return Rect::default();
        };
        let Some(bounds) = owner.get_transformed_bounds() else {
            return Rect::default();
        };
        let mut visible = bounds
            .bounds
            .transform_to_aabb(bounds.transform)
            .intersect(bounds.clip);
        let mut current: Option<Ref<Visual>> = Some(owner.upcast());
        while let Some(visual) = current {
            if let Some(clip) = visual.clip() {
                let Some(transform) = visual.transform_to_visual(&root) else {
                    return Rect::default();
                };
                visible = visible.intersect(clip.bounds().transform_to_aabb(transform));
            }
            current = visual.visual_parent();
        }
        visible
    }
}

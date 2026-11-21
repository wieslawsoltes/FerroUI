//! Source nearest-scope exclusion and reverse-index visual DFS. Toolbar branch
//! awaits the actual RibbonToolBar; no unrelated native toolbar is substituted.
use super::{Ribbon, RibbonGroup, RibbonItemHelper, RibbonToggleButton};
use ferroui_base::Visual;
pub(super) fn on_checked(button: &RibbonToggleButton) {
    if button.group_name().as_ref().is_none_or(|n| n.is_empty())
        || RibbonItemHelper::get_source_item(button).is_some()
    {
        return;
    }
    let scope = RibbonItemHelper::find_ancestor::<ferroui_base::Ref<RibbonGroup>>(Some(button))
        .map(|v| v.upcast::<Visual>())
        .or_else(|| {
            RibbonItemHelper::find_ancestor::<ferroui_base::Ref<Ribbon>>(Some(button))
                .map(|v| v.upcast::<Visual>())
        })
        .or_else(|| button.visual_parent());
    let Some(scope) = scope else { return };
    let mut stack = vec![scope];
    while let Some(current) = stack.pop() {
        if let Some(other) = current.cast::<RibbonToggleButton>()
            && !other.ptr_eq(&button.to_ref())
            && other.group_name() == button.group_name()
            && other.is_checked() == Some(true)
        {
            other.set_is_checked(Some(false));
        }
        let count = current.visual_children().count();
        for i in 0..count {
            stack.push(current.visual_children().get(i));
        }
    }
}

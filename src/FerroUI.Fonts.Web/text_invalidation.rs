//! Redoing the layout of the text on screen when a font arrives.

use ferroui_base::{Ref, Visual};
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::{Application, TextBlock};

fn invalidate(visual: &Ref<Visual>) {
    if let Some(text) = visual.cast::<TextBlock>() {
        text.invalidate_text_layout();
        text.invalidate_measure();
    } else if let Some(text) = visual.cast::<TextPresenter>() {
        text.invalidate_text_layout();
        text.invalidate_measure();
    }
}

/// Makes the text of every open window or view lay itself out again, once.
pub(crate) fn invalidate_text_layouts() {
    let Some(lifetime) = Application::current().and_then(|application| application.application_lifetime()) else {
        return;
    };
    let mut roots: Vec<Ref<Visual>> = Vec::new();
    if let Some(view) = lifetime.as_single_view_application_lifetime().and_then(|single| single.main_view()) {
        roots.extend(view.cast::<Visual>());
    }
    if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
        roots.extend(desktop.windows().iter().filter_map(|window| window.cast::<Visual>()));
    }
    for root in roots {
        invalidate(&root);
        for visual in root.get_visual_descendants() {
            invalidate(&visual);
        }
    }
}

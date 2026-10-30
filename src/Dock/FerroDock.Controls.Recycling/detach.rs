// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{ContentControl, Control, Decorator, Panel};

pub(crate) fn try_detach_from_parent(visual: &Visual) -> bool {
    if let Some(presenter) = visual
        .visual_parent()
        .and_then(|p| p.cast::<ContentPresenter>())
    {
        return try_detach_from_content_presenter(&presenter, visual);
    }
    let control = visual.to_ref().cast::<Control>();
    let parent = control
        .as_ref()
        .and_then(|c| c.parent())
        .map(|p| p.upcast::<ferroui_base::FerroObject>())
        .or_else(|| visual.visual_parent().map(|p| p.upcast()));
    let Some(parent) = parent else {
        return true;
    };
    if let (Some(panel), Some(control)) = (parent.cast::<Panel>(), control.as_ref()) {
        return panel.children().remove(control);
    }
    if let Some(presenter) = parent.cast::<ContentPresenter>() {
        return try_detach_from_content_presenter(&presenter, visual);
    }
    if let Some(content) = parent.cast::<ContentControl>() {
        if content
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .is_some_and(|c| c.upcast::<Visual>().ptr_eq(&visual.to_ref()))
        {
            content.set_current_value(ContentControl::content_property(), None);
            return true;
        }
    }
    if let Some(decorator) = parent.cast::<Decorator>() {
        if decorator
            .child()
            .is_some_and(|c| c.upcast::<Visual>().ptr_eq(&visual.to_ref()))
        {
            decorator.set_child(None::<ferroui_base::Ref<Control>>);
            return true;
        }
    }
    false
}
fn try_detach_from_content_presenter(presenter: &ContentPresenter, visual: &Visual) -> bool {
    if !presenter
        .child()
        .is_some_and(|c| c.upcast::<Visual>().ptr_eq(&visual.to_ref()))
    {
        return false;
    }
    presenter.set_current_value(ContentPresenter::content_property(), None);
    presenter.update_child();
    visual.visual_parent().is_none()
}

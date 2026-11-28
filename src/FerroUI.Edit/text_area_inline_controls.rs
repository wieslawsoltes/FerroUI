use super::*;
use crate::{VisualLine, VisualLineError};
use ferroui_base::reactive::Subscription;
use ferroui_controls::{Panel, PanelImpl, TextBlock};
use std::collections::HashSet;

#[repr(C)]
pub(super) struct InlineControlLayer {
    base: Panel,
    placements: RefCell<Vec<(Ref<Control>, Rect)>>,
    view: WeakRef<TextArea>,
    baseline_subscription: OnceCell<Subscription>,
}

ferro_class!(InlineControlLayer: Panel);
ferro_impl_classes!(InlineControlLayer: FerroObjectImpl, StyledElementImpl, VisualImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl LayoutableImpl for InlineControlLayer {
    fn measure_override(this: &Self, _available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(Size::new(f64::INFINITY, f64::INFINITY));
        }
        Size::default()
    }

    fn arrange_override(this: &Self, size: Size) -> Size {
        let placements = this.placements.borrow().clone();
        for (child, rect) in placements {
            child.arrange(rect);
        }
        size
    }

    fn on_measure_invalidated(this: &Self) {
        this.invalidate_editor_measure();
    }
}

impl InlineControlLayer {
    fn new(view: &TextArea) -> Ref<Self> {
        let layer = instantiate(Self {
            base: Panel::construct(),
            placements: RefCell::new(Vec::new()),
            view: view.to_ref().downgrade(),
            baseline_subscription: OnceCell::new(),
        });
        layer.set_clip_to_bounds(true);
        layer.set_use_layout_rounding(false);
        let weak = layer.downgrade();
        let subscription =
            TextBlock::baseline_offset_property()
                .changed()
                .subscribe(move |change| {
                    if let Some(layer) = weak.upgrade() {
                        let is_child =
                            change
                                .sender()
                                .downcast_ref::<Control>()
                                .is_some_and(|control| {
                                    control
                                        .visual_parent()
                                        .is_some_and(|parent| parent.ptr_eq(&layer))
                                });
                        if is_child {
                            layer.invalidate_editor_measure();
                        }
                    }
                });
        layer.baseline_subscription.set(subscription).ok();
        layer
    }

    fn invalidate_editor_measure(&self) {
        if let Some(area) = self.view.upgrade() {
            if !area.preparing_inline_controls.get() {
                area.invalidate_line_measure();
            }
        }
    }
}

struct PrepareInlineControls<'a>(&'a Cell<bool>);

impl Drop for PrepareInlineControls<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl TextArea {
    pub(super) fn arrange_inline_controls(&self) {
        if self.inline_controls.get().is_none()
            && self.element_generators().is_empty()
            && self.line_transformers().is_empty()
        {
            return;
        }
        if self.preparing_inline_controls.replace(true) {
            return;
        }
        let _preparing = PrepareInlineControls(&self.preparing_inline_controls);
        let placements = match self.inline_control_placements() {
            Ok(placements) => placements,
            Err(error) => {
                if let Some(editor) = self.editor() {
                    editor.report_error(error.to_string());
                }
                Vec::new()
            }
        };
        let Some(layer) = self.inline_controls.get() else {
            return;
        };
        let visible: HashSet<_> = placements.iter().map(|(child, _)| child).collect();
        for child in layer.children().snapshot().iter() {
            if !visible.contains(child) {
                if child.is_keyboard_focus_within() {
                    self.focus();
                }
                layer.children().remove(child);
            }
        }
        if *layer.placements.borrow() != placements {
            layer.placements.replace(placements);
            layer.invalidate_arrange();
        }
        let viewport = self.content_viewport();
        layer.measure(viewport.size());
        let bounds = if self.uses_native_layers() { Rect::from_size(viewport.size()) } else { viewport };
        layer.arrange(bounds);
    }

    pub(super) fn prepare_inline_line(&self, line: &VisualLine) -> Result<bool, VisualLineError> {
        let mut controls = line
            .elements()
            .iter()
            .filter_map(|element| element.inline_control())
            .peekable();
        if controls.peek().is_none() {
            return Ok(false);
        }
        if !self.preparing_inline_controls.get() {
            // Hit testing and drawing may discover an inline object before
            // layout. Only the layout pass may parent or measure its child.
            self.invalidate_line_measure();
            return Ok(true);
        }
        let layer = self.inline_controls.get_or_init(|| InlineControlLayer::new(self));
        if layer.visual_parent().is_none() {
            self.logical_children().add(layer.clone().upcast());
            if let Some(plane) = self.native_text_plane() {
                plane.visual_children().add(layer.clone().upcast());
            } else {
                self.visual_children().add(layer.clone().upcast());
            }
        }
        for child in controls {
            if !child
                .visual_parent()
                .is_some_and(|parent| parent.ptr_eq(layer))
            {
                if child.visual_parent().is_some()
                    || child.parent().is_some()
                    || child.ptr_eq(&self.to_ref())
                    || child.is_visual_ancestor_of(self)
                {
                    return Err(VisualLineError::Extension(
                        "An inline control must be unparented or owned by this text area".into(),
                    ));
                }
                layer.children().add(child.clone());
            }
            child.measure(Size::new(f64::INFINITY, f64::INFINITY));
        }
        Ok(true)
    }

    pub(crate) fn is_hosted_child_event(
        &self,
        args: &ferroui_base::interactivity::RoutedEventArgs,
    ) -> bool {
        let Some(source) = args.source().and_then(|source| source.cast::<Visual>()) else {
            return false;
        };
        self.is_hosted_child_visual(&source)
    }

    pub(super) fn is_hosted_child_visual(&self, visual: &Ref<Visual>) -> bool {
        self.is_margin_visual(visual)
            || self.is_custom_layer_visual(visual)
            || self.inline_controls.get().is_some_and(|layer| {
                layer.ptr_eq(visual) || layer.is_visual_ancestor_of(visual)
            })
    }
}

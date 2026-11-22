use super::*;
use crate::{EditorAdornmentError, KnownLayer};
use ferroui_controls::{Panel, PanelImpl};

/// The client control's placement relative to a built-in editor layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerInsertionPosition {
    Below,
    Replace,
    Above,
}

/// An editor layer anchor and its relative position in painting order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayerPosition {
    pub layer: KnownLayer,
    pub insertion: LayerInsertionPosition,
}

/// A native control inserted in the editor's layer stack.
#[derive(Clone)]
pub struct EditorLayer {
    pub control: Ref<Control>,
    pub position: LayerPosition,
}

#[derive(Default)]
pub(super) struct LayerHost {
    stack: OnceCell<LayerStack>,
    custom: RefCell<Vec<EditorLayer>>,
    changing: Cell<bool>,
}

struct LayerStack {
    panel: Ref<LayerPanel>,
    planes: [Ref<LayerPlane>; 3],
}

impl LayerStack {
    fn plane(&self, layer: KnownLayer) -> Option<&Ref<LayerPlane>> {
        match layer {
            KnownLayer::Background => None,
            KnownLayer::Selection => Some(&self.planes[0]),
            KnownLayer::Text => Some(&self.planes[1]),
            KnownLayer::Caret => Some(&self.planes[2]),
        }
    }
}

struct LayerMutation<'a>(&'a Cell<bool>);

impl Drop for LayerMutation<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[repr(C)]
struct LayerPanel {
    base: Panel,
}

ferro_class!(LayerPanel: Panel);
ferro_impl_classes!(LayerPanel: FerroObjectImpl, StyledElementImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, VisualImpl, PanelImpl);

impl LayoutableImpl for LayerPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(available);
        }
        Size::default()
    }

    fn arrange_override(this: &Self, size: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.arrange(Rect::from_size(size));
        }
        size
    }
}

#[repr(C)]
struct LayerPlane {
    base: Control,
    view: WeakRef<TextArea>,
    layer: KnownLayer,
}

ferro_class!(LayerPlane: Control);
ferro_impl_classes!(LayerPlane: FerroObjectImpl, StyledElementImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl);

impl ControlImpl for LayerPlane {
    fn automation_target(this: &Self) -> Ref<Control> {
        this.view
            .upgrade()
            .map_or_else(|| this.to_ref().upcast(), |view| view.automation_target())
    }
}

impl VisualImpl for LayerPlane {
    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(view) = this.view.upgrade() {
            view.render_editor_plane(context, this.layer);
        }
    }
}

impl TextArea {
    /// Application layers in back-to-front order. Known layers are addressed
    /// through their semantic anchors.
    pub fn layers(&self) -> Vec<EditorLayer> {
        self.layers.custom.borrow().clone()
    }

    /// Hosts a native control in the content viewport. Inserting a replacement
    /// displaces the previous replacement at the same anchor.
    pub fn insert_layer(
        &self,
        control: Ref<Control>,
        layer: KnownLayer,
        insertion: LayerInsertionPosition,
    ) -> Result<(), EditorAdornmentError> {
        let mutation = self.begin_layer_mutation()?;
        if layer == KnownLayer::Background && insertion != LayerInsertionPosition::Above {
            return Err(EditorAdornmentError::InvalidLayerPosition);
        }
        self.validate_adornment(&control)?;
        let stack = self.layers.stack.get_or_init(|| self.create_layers());
        let position = LayerPosition { layer, insertion };
        let displaced = {
            let mut items = self.layers.custom.borrow_mut();
            let index = if insertion == LayerInsertionPosition::Below {
                items.partition_point(|item| item.position <= position)
            } else {
                items.partition_point(|item| item.position < position)
            };
            let new = EditorLayer { control, position };
            if insertion == LayerInsertionPosition::Replace
                && items
                    .get(index)
                    .is_some_and(|item| item.position == position)
            {
                Some(std::mem::replace(&mut items[index], new))
            } else {
                items.insert(index, new);
                None
            }
        };
        if let Some(old) = displaced {
            self.detach_layer(stack, &old.control);
        }
        if insertion == LayerInsertionPosition::Replace {
            if let Some(plane) = stack.plane(layer) {
                if plane.is_keyboard_focus_within() {
                    self.focus();
                }
                plane.set_is_visible(false);
            }
        }
        self.sync_layer_order(stack);
        drop(mutation);
        self.invalidate_line_measure();
        self.request_redraw();
        Ok(())
    }

    /// Removes a native layer. Removing a replacement restores the editor's
    /// original phase and any native inline children in it.
    pub fn remove_layer(&self, control: &Ref<Control>) -> Result<bool, EditorAdornmentError> {
        let mutation = self.begin_layer_mutation()?;
        let Some(index) = self
            .layers
            .custom
            .borrow()
            .iter()
            .position(|item| item.control == *control)
        else {
            return Ok(false);
        };
        let item = self.layers.custom.borrow_mut().remove(index);
        let stack = self.layers.stack.get().unwrap();
        self.detach_layer(stack, control);
        if item.position.insertion == LayerInsertionPosition::Replace {
            if let Some(plane) = stack.plane(item.position.layer) {
                plane.set_is_visible(true);
            }
        }
        drop(mutation);
        self.invalidate_line_measure();
        self.request_redraw();
        Ok(true)
    }

    fn begin_layer_mutation(&self) -> Result<LayerMutation<'_>, EditorAdornmentError> {
        if self.rendering.get() || self.layers.changing.replace(true) {
            Err(EditorAdornmentError::Busy)
        } else {
            Ok(LayerMutation(&self.layers.changing))
        }
    }

    fn create_layers(&self) -> LayerStack {
        let panel = instantiate(LayerPanel {
            base: Panel::construct(),
        });
        panel.set_clip_to_bounds(true);
        panel.set_use_layout_rounding(false);
        let planes = [KnownLayer::Selection, KnownLayer::Text, KnownLayer::Caret].map(|layer| {
            let plane = instantiate(LayerPlane {
                base: Control::construct(),
                view: self.to_ref().downgrade(),
                layer,
            });
            plane.set_use_layout_rounding(false);
            panel.children().add(plane.clone());
            plane
        });
        self.logical_children().add(panel.clone().upcast());
        self.visual_children().add(panel.clone().upcast());
        if let Some(inline) = self.inline_controls.get() {
            self.visual_children().remove(&inline.clone().upcast());
            planes[1].visual_children().add(inline.clone().upcast());
        }
        LayerStack { panel, planes }
    }

    fn detach_layer(&self, stack: &LayerStack, control: &Ref<Control>) {
        if control.is_keyboard_focus_within() {
            self.focus();
        }
        stack.panel.children().remove(control);
    }

    fn sync_layer_order(&self, stack: &LayerStack) {
        let mut ordered: Vec<_> = self
            .layers()
            .into_iter()
            .map(|item| (item.position, item.control))
            .collect();
        ordered.extend(stack.planes.iter().map(|plane| {
            (
                LayerPosition {
                    layer: plane.layer,
                    insertion: LayerInsertionPosition::Replace,
                },
                plane.clone().upcast(),
            )
        }));
        ordered.sort_by_key(|(position, _)| *position);
        let children = stack.panel.children();
        for (index, (_, control)) in ordered.into_iter().enumerate() {
            match children.index_of(&control) {
                Some(current) if current != index => children.move_item(current, index),
                Some(_) => {}
                None => children.insert(index, control),
            }
        }
    }

    pub(super) fn uses_native_layers(&self) -> bool {
        self.layers.stack.get().is_some()
    }

    pub(super) fn native_text_plane(&self) -> Option<Ref<Visual>> {
        Some(self.layers.stack.get()?.planes[1].clone().upcast())
    }

    pub(super) fn arrange_native_layers(&self) {
        if let Some(stack) = self.layers.stack.get() {
            let Ok(_layout) = self.begin_layer_mutation() else {
                return;
            };
            let viewport = self.content_viewport();
            stack.panel.measure(viewport.size());
            stack.panel.arrange(viewport);
        }
    }

    pub(super) fn invalidate_native_layer_planes(&self) {
        if let Some(stack) = self.layers.stack.get() {
            for plane in &stack.planes {
                plane.invalidate_visual();
            }
        }
    }

    pub(super) fn invalidate_native_layer(&self, layer: KnownLayer) -> bool {
        let Some(stack) = self.layers.stack.get().filter(|_| !self.rendering.get()) else {
            return false;
        };
        if let Some(plane) = stack.plane(layer) {
            plane.invalidate_visual();
        } else {
            self.invalidate_visual();
        }
        true
    }

    pub(super) fn is_custom_layer_visual(&self, source: &Ref<Visual>) -> bool {
        self.layers
            .custom
            .borrow()
            .iter()
            .any(|item| item.control.ptr_eq(source) || item.control.is_visual_ancestor_of(source))
    }
}

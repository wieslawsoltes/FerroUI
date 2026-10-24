//! `XamlCompositionBrushBase` of WinUI (`Microsoft.UI.Xaml.Media`): the base
//! of the brushes of the object model that paint with a composition brush,
//! such as [`AcrylicBrush`](super::AcrylicBrush) (winui-materials.md).

use crate::media::ref_adapter::RefAdapter;
use crate::media::{Brush, BrushImpl, BrushImplExt, Color, IBrush, ServerBrushFactory};
use crate::rendering::composition::brushes::ServerXamlCompositionBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{CompositionBrush, Compositor};
use crate::{ferro_class, ferro_property, FerroObjectImpl, FerroProperty, Ref, StyledProperty};
use std::cell::RefCell;
use std::rc::Rc;

/// What a XAML composition brush sends to its server side with each batch:
/// its fallback colour, the composition brush it paints with and the ids of
/// the XAML lights that target it.
pub(crate) struct XamlBrushChanges {
    pub(crate) fallback_color: Color,
    pub(crate) composition_brush: Option<crate::rendering::composition::server::ServerObjectId>,
    pub(crate) light_target_ids: Vec<String>,
    pub(crate) needs_transparent_window: bool,
}

#[derive(Default)]
struct XamlBrushState {
    /// `CompositionBrush`: what the brush paints with.
    composition_brush: Option<Rc<CompositionBrush>>,
    /// The compositor the brush is connected to.
    compositor: Option<Rc<Compositor>>,
    /// The ids of the XAML lights that target the brush
    /// (`XamlLight.AddTargetBrush`).
    light_target_ids: Vec<String>,
    /// Whether the composition brush shows what is behind the window (a
    /// host backdrop): where the window is opaque the fallback colour is
    /// painted instead.
    needs_transparent_window: bool,
}

/// A brush that paints an area with a composition brush (WinUI
/// `Microsoft.UI.Xaml.Media.XamlCompositionBrushBase`).
///
/// A subclass creates its [`composition_brush`](Self::composition_brush)
/// when the brush is first used in a window that has a compositor
/// (`OnConnected`) and releases it when no element of such a window uses it
/// any more (`OnDisconnected`). The compositor paints the composition brush
/// inside the shape the brush fills, as the brushes of WinUI's composition
/// are painted; where no composition brush is set, and where the brush is
/// drawn without a compositor (an image of a visual drawn by the immediate
/// renderer), the area is filled with [`fallback_color`](Self::fallback_color).
#[repr(C)]
pub struct XamlCompositionBrushBase {
    base: Brush,
    state: RefCell<XamlBrushState>,
}

ferro_class! {
    XamlCompositionBrushBase: Brush, virtuals XamlCompositionBrushBaseImpl: BrushImpl {
        /// `OnConnected`: the brush is used in a window with a compositor for
        /// the first time; the subclass creates its composition brush.
        fn on_connected(this);

        /// `OnDisconnected`: no element of a window with a compositor uses
        /// the brush any more; the subclass releases its composition brush.
        fn on_disconnected(this);
    }
}

crate::ferro_impl_classes!(XamlCompositionBrushBase: FerroObjectImpl);

impl BrushImpl for XamlCompositionBrushBase {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerXamlCompositionBrush::new(c))
    }

    fn on_referenced_from_compositor(this: &Self, c: &Rc<Compositor>) {
        Self::parent_on_referenced_from_compositor(this, c);
        let connect = {
            let mut state = this.state.borrow_mut();
            if state.compositor.is_none() {
                state.compositor = Some(c.clone());
                true
            } else {
                false
            }
        };
        if connect {
            this.on_connected();
        }
    }

    fn on_unreferenced_from_compositor(this: &Self, c: &Rc<Compositor>) {
        Self::parent_on_unreferenced_from_compositor(this, c);
        let disconnect = this.state.borrow().compositor.as_ref().is_some_and(|compositor| Rc::ptr_eq(compositor, c));
        if disconnect {
            this.on_disconnected();
            this.state.borrow_mut().compositor = None;
        }
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        let state = this.state.borrow();
        let composition_brush = state
            .composition_brush
            .as_ref()
            .filter(|brush| std::ptr::eq(&**brush.compositor(), c) && !brush.is_disposed())
            .map(|brush| brush.server());
        writer.write_value(Some(XamlBrushChanges {
            fallback_color: this.fallback_color(),
            composition_brush,
            light_target_ids: state.light_target_ids.clone(),
            needs_transparent_window: state.needs_transparent_window,
        }));
    }
}

impl XamlCompositionBrushBaseImpl for XamlCompositionBrushBase {
    fn on_connected(_this: &Self) {}

    fn on_disconnected(_this: &Self) {}
}

crate::ferro_properties! { impl XamlCompositionBrushBase {
    ferro_property!(
        /// `FallbackColor`: the colour the brush fills with when it has no
        /// composition brush, or cannot use it.
        pub fn fallback_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<XamlCompositionBrushBase, _>("FallbackColor", Color::default())
        }
    );
} }

impl XamlCompositionBrushBase {
    /// Creates the class data; see [`FerroObject::construct`](crate::FerroObject).
    pub fn construct() -> Self {
        Self { base: Brush::construct(), state: RefCell::new(XamlBrushState::default()) }
    }

    /// `FallbackColor`.
    pub fn fallback_color(&self) -> Color {
        self.get_value(Self::fallback_color_property())
    }

    pub fn set_fallback_color(&self, value: Color) {
        self.set_value(Self::fallback_color_property(), value)
    }

    /// `CompositionBrush`: the composition brush the brush paints with, set
    /// by the subclass.
    pub fn composition_brush(&self) -> Option<Rc<CompositionBrush>> {
        self.state.borrow().composition_brush.clone()
    }

    /// Sets the composition brush the brush paints with (`None` paints the
    /// fallback colour). It must belong to the compositor the brush is
    /// connected to.
    pub fn set_composition_brush(&self, brush: Option<Rc<CompositionBrush>>) {
        {
            let mut state = self.state.borrow_mut();
            let same = match (&state.composition_brush, &brush) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if same {
                return;
            }
            state.composition_brush = brush;
        }
        // The server brush paints the new composition brush: the render
        // data of the elements that use the brush is unchanged (and a brush
        // that connects while the renderer draws must not invalidate them).
        self.register_for_serialization();
    }

    /// Whether the composition brush shows what is behind the window (a host
    /// backdrop): where the window of the element is opaque, the platform
    /// shows nothing behind it, and the fallback colour is painted instead.
    pub fn needs_transparent_window(&self) -> bool {
        self.state.borrow().needs_transparent_window
    }

    pub fn set_needs_transparent_window(&self, value: bool) {
        if self.state.borrow().needs_transparent_window == value {
            return;
        }
        self.state.borrow_mut().needs_transparent_window = value;
        self.register_for_serialization();
    }

    /// The compositor the brush is connected to (`None` while it is not).
    pub fn compositor(&self) -> Option<Rc<Compositor>> {
        self.state.borrow().compositor.clone()
    }

    /// Whether the brush is connected (`OnConnected` was called and
    /// `OnDisconnected` was not since).
    pub fn is_connected(&self) -> bool {
        self.state.borrow().compositor.is_some()
    }

    /// The ids of the XAML lights that target the brush.
    pub fn light_target_ids(&self) -> Vec<String> {
        self.state.borrow().light_target_ids.clone()
    }

    /// `XamlLight.AddTargetBrush(id, this)`: the XAML lights of id `id`
    /// light the brush.
    pub fn add_light_target_id(&self, id: &str) {
        {
            let mut state = self.state.borrow_mut();
            if state.light_target_ids.iter().any(|existing| existing == id) {
                return;
            }
            state.light_target_ids.push(id.to_owned());
        }
        self.register_for_serialization();
    }

    /// `XamlLight.RemoveTargetBrush(id, this)`.
    pub fn remove_light_target_id(&self, id: &str) {
        {
            let mut state = self.state.borrow_mut();
            let count = state.light_target_ids.len();
            state.light_target_ids.retain(|existing| existing != id);
            if state.light_target_ids.len() == count {
                return;
            }
        }
        self.register_for_serialization();
    }

    /// The XAML composition brush `brush` is, if it is one.
    pub fn of(brush: &dyn IBrush) -> Option<Ref<XamlCompositionBrushBase>> {
        brush.as_object().and_then(|object| object.downcast_ref::<XamlCompositionBrushBase>()).map(|brush| brush.to_ref())
    }

    /// The brush as `Rc<dyn IBrush>`.
    pub fn as_brush(&self) -> Rc<dyn IBrush> {
        Rc::new(RefAdapter(self.to_ref()))
    }
}

crate::ferro_class_info!(XamlCompositionBrushBase {
    markup: {
        namespace: "FerroUI.Media",
    },
});

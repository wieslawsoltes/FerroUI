//! The server-side counterparts of the WinUI lights (`AmbientLight`,
//! `DistantLight`, `PointLight`, `SpotLight`) and of
//! `VisualUnorderedCollection`, the light math, and the lighting of the
//! content of the visuals they reach.
//!
//! # How a light is drawn
//!
//! WinUI lights flat visuals that face the viewer (+Z) with diffuse
//! (Lambert) lighting: the colour of a lit point is the colour of the content
//! times the sum, over the lights that reach the visual, of the colour of the
//! light times its intensity, its attenuation and `N.L`, the cosine of the
//! angle between the light and the normal of the visual. A visual that a
//! light reaches is dark where no light shines.
//!
//! Here the render walk asks [`LitContent::begin`] before it draws the own
//! content of a visual. When enabled lights reach the visual, the content is
//! drawn into a layer, and a light map over the bounds of the content (a
//! small bitmap of samples of the summed light, see [`LightMap::pixels`]) is
//! drawn over it with the source-atop blending of bitmaps
//! ([`LitContent::end`]), which keeps the alpha of the content.
//!
//! Deviation (winui-composition.md): the drawing contexts blend bitmaps only,
//! and source-over a layer of the content, so the multiplication is
//! approximated by source-atop: a light map pixel is black with the alpha
//! `1 - m` plus the light `L - m` above it (`m` the smallest channel of the
//! light `L`), which gives `C * m + (L - m) * alpha(C)`. That is exact for
//! white and grey light (any content) and for white content (any light); a
//! coloured light tints rather than multiplies coloured content. The light is
//! clamped to 1 per channel (a light can darken, never brighten), sampled on
//! a grid of at most [`LIGHT_MAP_MAX_CELLS`] cells per side and interpolated
//! by the bitmap sampler. Each visual is lit in its own layer (its children
//! after it, each in theirs), the shadow of a sprite visual is lit with its
//! content, and a layer visual itself is not lit (its children are).

use super::{
    impl_server_render_resource, CompositionProperty, IAnimatedServerObject, IServerObject, IServerRenderResource,
    IServerRenderResourceObserver, ServerCompositionVisual, ServerCompositor, ServerLayerVisual, ServerRenderResource,
};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{Color, RenderOptions};
use crate::platform::{AlphaFormat, IDrawingContextImpl, PixelFormats, SharedBitmapImpl};
use crate::rendering::composition::generated::{
    ServerAmbientLightHooks, ServerAmbientLightProps, ServerCompositionLightHooks, ServerCompositionLightProps,
    ServerDistantLightHooks, ServerDistantLightProps, ServerPointLightHooks, ServerPointLightProps,
    ServerSpotLightHooks, ServerSpotLightProps, ServerVisualUnorderedCollectionHooks,
    ServerVisualUnorderedCollectionProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, PixelSize, Point, Rect, Vector, Vector3D};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// The size, in units of the lit visual, of a cell of a light map.
pub const LIGHT_MAP_CELL: f64 = 4.0;

/// The largest number of cells of a light map along a side.
pub const LIGHT_MAP_MAX_CELLS: i32 = 128;

/// How far up the parent chain a light is looked for: a cycle would loop
/// for ever.
const MAX_LIGHT_DEPTH: usize = 4096;

// --- the light math ----------------------------------------------------------------

/// An amount of light per channel: 0 is none, 1 is full.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightColor {
    /// The red channel.
    pub r: f64,
    /// The green channel.
    pub g: f64,
    /// The blue channel.
    pub b: f64,
}

impl LightColor {
    /// No light.
    pub const BLACK: LightColor = LightColor { r: 0.0, g: 0.0, b: 0.0 };

    /// Full white light.
    pub const WHITE: LightColor = LightColor { r: 1.0, g: 1.0, b: 1.0 };

    /// A light of the given channels.
    pub const fn new(r: f64, g: f64, b: f64) -> LightColor {
        LightColor { r, g, b }
    }

    /// The light of a colour at an intensity: the channels and the alpha of
    /// the colour as fractions, times the intensity.
    ///
    /// Deviation (winui-composition.md): the alpha of the colour of a light
    /// scales it (a transparent light lights nothing); WinUI does not say.
    pub fn from_color(color: Color, intensity: f64) -> LightColor {
        let scale = intensity.max(0.0) * color.a as f64 / 255.0;
        LightColor {
            r: color.r as f64 / 255.0 * scale,
            g: color.g as f64 / 255.0 * scale,
            b: color.b as f64 / 255.0 * scale,
        }
    }

    /// The light times a factor.
    pub fn scale(self, factor: f64) -> LightColor {
        LightColor { r: self.r * factor, g: self.g * factor, b: self.b * factor }
    }

    /// The sum of two lights.
    pub fn add(self, other: LightColor) -> LightColor {
        LightColor { r: self.r + other.r, g: self.g + other.g, b: self.b + other.b }
    }

    /// The light between `self` (at 0) and `other` (at 1).
    pub fn lerp(self, other: LightColor, t: f64) -> LightColor {
        LightColor {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }

    /// Every channel limited to 0..1.
    pub fn clamped(self) -> LightColor {
        LightColor { r: self.r.clamp(0.0, 1.0), g: self.g.clamp(0.0, 1.0), b: self.b.clamp(0.0, 1.0) }
    }

    /// The light-map pixel that turns content into content lit by this
    /// light under source-atop blending: premultiplied `(L - m, 1 - m)`, `m`
    /// the smallest channel, as RGBA bytes.
    pub fn atop_pixel(self) -> [u8; 4] {
        let light = self.clamped();
        let m = light.r.min(light.g).min(light.b);
        let byte = |value: f64| (value * 255.0).round().clamp(0.0, 255.0) as u8;
        [byte(light.r - m), byte(light.g - m), byte(light.b - m), byte(1.0 - m)]
    }
}

/// The attenuation of a point or spot light with the distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightAttenuation {
    /// The constant term of the denominator.
    pub constant: f64,
    /// The term proportional to the distance.
    pub linear: f64,
    /// The term proportional to the square of the distance.
    pub quadratic: f64,
    /// The distance up to which the light is attenuated as at this distance.
    pub min_cutoff: f64,
    /// The distance beyond which the light lights nothing.
    pub max_cutoff: f64,
}

impl LightAttenuation {
    /// No attenuation (the default of WinUI: constant 1).
    pub const NONE: LightAttenuation =
        LightAttenuation { constant: 1.0, linear: 0.0, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: f64::MAX };

    /// The factor of the light at `distance`: none beyond the maximum
    /// cutoff, `1 / (constant + linear * d + quadratic * d^2)` with the
    /// distance raised to the minimum cutoff otherwise (1 when the
    /// denominator is not positive).
    ///
    /// Deviation (winui-composition.md): WinUI does not say how the cutoffs
    /// apply; closer than the minimum cutoff the light is attenuated as at
    /// the cutoff, beyond the maximum cutoff it lights nothing.
    pub fn factor(&self, distance: f64) -> f64 {
        if distance > self.max_cutoff {
            return 0.0;
        }
        let d = distance.max(self.min_cutoff).max(0.0);
        let denominator = self.constant + self.linear * d + self.quadratic * d * d;
        if denominator <= 0.0 {
            1.0
        } else {
            1.0 / denominator
        }
    }
}

fn length(v: Vector3D) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

/// `N.L` of a flat visual facing +Z at `point` for a light at `position`:
/// `z / distance`, 0 for a light at or below the plane. Also the distance.
pub fn point_light_incidence(position: Vector3D, point: Point) -> (f64, f64) {
    let to_light = Vector3D::new(position.x - point.x, position.y - point.y, position.z);
    let distance = length(to_light);
    if distance <= 0.0 || position.z <= 0.0 {
        return (0.0, distance);
    }
    (position.z / distance, distance)
}

/// `N.L` of a flat visual facing +Z for a distant light shining in
/// `direction`: `-z / |direction|`, 0 for a light from behind or a zero
/// direction.
pub fn distant_light_incidence(direction: Vector3D) -> f64 {
    let len = length(direction);
    if len <= 0.0 {
        return 0.0;
    }
    (-direction.z / len).max(0.0)
}

fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where `point` is in the cones of a spot light at `position` shining in
/// `direction`: 1 inside the inner cone, 0 outside the outer cone, a smooth
/// step between. The angles are from the direction to the edge of the cone;
/// an inner angle wider than the outer one is the outer one.
pub fn spot_light_cone(
    position: Vector3D,
    direction: Vector3D,
    inner_cone_angle: f64,
    outer_cone_angle: f64,
    point: Point,
) -> f64 {
    let to_point = Vector3D::new(point.x - position.x, point.y - position.y, -position.z);
    let (to_point_length, direction_length) = (length(to_point), length(direction));
    if to_point_length <= 0.0 || direction_length <= 0.0 {
        return 0.0;
    }
    let cos_angle = (to_point.x * direction.x + to_point.y * direction.y + to_point.z * direction.z)
        / (to_point_length * direction_length);
    let outer = outer_cone_angle.clamp(0.0, std::f64::consts::PI);
    let inner = inner_cone_angle.clamp(0.0, outer);
    let (cos_inner, cos_outer) = (inner.cos(), outer.cos());
    if cos_angle >= cos_inner {
        return 1.0;
    }
    if cos_angle <= cos_outer || cos_inner <= cos_outer {
        return 0.0;
    }
    smoothstep((cos_angle - cos_outer) / (cos_inner - cos_outer))
}

/// A light in its coordinate space, on the plane `z = 0` of that space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightSource {
    /// The same light everywhere.
    Ambient { color: LightColor },
    /// A light from far away shining in `direction`.
    Distant { color: LightColor, direction: Vector3D },
    /// A light at `position`.
    Point { color: LightColor, position: Vector3D, attenuation: LightAttenuation },
    /// A light at `position` shining into the cones around `direction`.
    Spot {
        inner_color: LightColor,
        outer_color: LightColor,
        position: Vector3D,
        direction: Vector3D,
        inner_cone_angle: f64,
        outer_cone_angle: f64,
        attenuation: LightAttenuation,
    },
}

impl LightSource {
    /// The light at `point` of the plane `z = 0` (not clamped).
    pub fn light_at(&self, point: Point) -> LightColor {
        match *self {
            LightSource::Ambient { color } => color,
            LightSource::Distant { color, direction } => color.scale(distant_light_incidence(direction)),
            LightSource::Point { color, position, attenuation } => {
                let (incidence, distance) = point_light_incidence(position, point);
                color.scale(incidence * attenuation.factor(distance))
            }
            LightSource::Spot {
                inner_color,
                outer_color,
                position,
                direction,
                inner_cone_angle,
                outer_cone_angle,
                attenuation,
            } => {
                let cone = spot_light_cone(position, direction, inner_cone_angle, outer_cone_angle, point);
                if cone <= 0.0 {
                    return LightColor::BLACK;
                }
                let (incidence, distance) = point_light_incidence(position, point);
                // Deviation (winui-composition.md): between the cones the
                // colour goes from the inner to the outer one and the light
                // falls off to nothing at the edge of the outer cone.
                outer_color.lerp(inner_color, cone).scale(cone * incidence * attenuation.factor(distance))
            }
        }
    }
}

/// The lights that reach a visual, each with the transform from the space
/// of the visual to the space of the light.
#[derive(Clone, Debug, Default)]
pub struct LightMap {
    lights: Vec<(LightSource, Matrix)>,
}

impl LightMap {
    /// A map without lights.
    pub fn new() -> LightMap {
        LightMap::default()
    }

    /// Adds a light; `to_light_space` maps the coordinates of the lit visual
    /// to those of the coordinate space of the light.
    pub fn add(&mut self, source: LightSource, to_light_space: Matrix) {
        self.lights.push((source, to_light_space));
    }

    /// Whether the map has no lights.
    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }

    /// The sum of the lights at `point` of the lit visual, clamped to 0..1.
    pub fn light_at(&self, point: Point) -> LightColor {
        let mut light = LightColor::BLACK;
        for (source, to_light_space) in &self.lights {
            light = light.add(source.light_at(to_light_space.transform(point)));
        }
        light.clamped()
    }

    /// The number of cells of the light map of `rect` along each side: a cell
    /// per [`LIGHT_MAP_CELL`] units, at least 2 and at most
    /// [`LIGHT_MAP_MAX_CELLS`].
    pub fn size_for(rect: Rect) -> PixelSize {
        let cells = |length: f64| ((length / LIGHT_MAP_CELL).ceil() as i32).clamp(2, LIGHT_MAP_MAX_CELLS);
        PixelSize::new(cells(rect.width), cells(rect.height))
    }

    /// The RGBA pixels (premultiplied, row by row from the top) of the light
    /// map of `rect` at `size`: each pixel the [`LightColor::atop_pixel`] of
    /// the light at the centre of its cell. `None` when the light is full
    /// everywhere, so the map would change nothing.
    pub fn pixels(&self, rect: Rect, size: PixelSize) -> Option<Vec<u8>> {
        let (columns, rows) = (size.width.max(1), size.height.max(1));
        let mut pixels = Vec::with_capacity((columns * rows * 4) as usize);
        let mut changes = false;
        for row in 0..rows {
            let y = rect.y + (row as f64 + 0.5) * rect.height / rows as f64;
            for column in 0..columns {
                let x = rect.x + (column as f64 + 0.5) * rect.width / columns as f64;
                let pixel = self.light_at(Point::new(x, y)).atop_pixel();
                changes |= pixel[3] != 0;
                pixels.extend_from_slice(&pixel);
            }
        }
        changes.then_some(pixels)
    }
}

// --- the visual collection -------------------------------------------------------------

fn no_composition_property(_name: &str) -> Option<&'static CompositionProperty> {
    None
}

/// The server-side counterpart of a
/// [`VisualUnorderedCollection`](crate::rendering::composition::VisualUnorderedCollection):
/// a render resource, so a change of the collection invalidates the light
/// that holds it.
pub struct ServerVisualUnorderedCollection {
    base: ServerRenderResource,
    props: ServerVisualUnorderedCollectionProps,
}

impl ServerVisualUnorderedCollection {
    /// Creates the collection, active.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerVisualUnorderedCollection> {
        let collection = Rc::new_cyclic(|this: &Weak<ServerVisualUnorderedCollection>| {
            ServerVisualUnorderedCollection {
                base: ServerRenderResource::new(
                    compositor,
                    this.clone() as Weak<dyn IAnimatedServerObject>,
                    this.clone() as Weak<dyn IServerRenderResource>,
                ),
                props: ServerVisualUnorderedCollectionProps::new(),
            }
        });
        collection.base.object().activate();
        collection
    }

    /// The server-side visuals of the collection.
    pub fn visuals(&self) -> Vec<Rc<ServerCompositionVisual>> {
        self.props
            .list()
            .list()
            .into_iter()
            .filter_map(|visual| visual.into_any_rc().downcast::<ServerCompositionVisual>().ok())
            .collect()
    }

    /// The number of visuals.
    pub fn count(&self) -> usize {
        self.props.list().count()
    }

    /// Whether `visual` is in the collection.
    pub fn contains(&self, visual: &ServerCompositionVisual) -> bool {
        let target = visual as *const ServerCompositionVisual;
        self.props.list().list().iter().any(|item| std::ptr::addr_eq(Rc::as_ptr(item), target))
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The collection behind an object property, if it is one.
    pub fn of(object: &Option<Rc<dyn IServerObject>>) -> Option<&ServerVisualUnorderedCollection> {
        object.as_ref().and_then(|object| object.as_any().downcast_ref::<ServerVisualUnorderedCollection>())
    }
}

impl_server_render_resource!(ServerVisualUnorderedCollection, base, composition_property: no_composition_property);

impl ServerVisualUnorderedCollectionHooks for ServerVisualUnorderedCollection {}

impl IServerObject for ServerVisualUnorderedCollection {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.props.list().clear();
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

// --- the lights ----------------------------------------------------------------------

/// What every server-side light has: the properties of `CompositionLight`,
/// where it shines from and what it lights.
pub trait IServerCompositionLight {
    /// The properties of `CompositionLight`.
    fn light_props(&self) -> &ServerCompositionLightProps;

    /// The bookkeeping of what the light lit.
    fn light_state(&self) -> &ServerLightState;

    fn is_disposed(&self) -> bool;

    /// The visual whose coordinates the light is in; `None` for the targeted
    /// visual (and for an ambient light, which has no position).
    fn coordinate_space(&self) -> Option<Rc<ServerCompositionVisual>>;

    /// The light in its coordinate space.
    fn source(&self) -> LightSource;

    /// Whether `visual` is lit by the light: in the subtree of a target and
    /// not in the subtree of an exclusion (an exclusion that is also a
    /// target is excluded). The target, if it is.
    fn target_of(&self, visual: &ServerCompositionVisual) -> Option<Rc<ServerCompositionVisual>> {
        let props = self.light_props();
        let (targets, exclusions) = (props.targets(), props.exclusions_from_targets());
        let targets = ServerVisualUnorderedCollection::of(&targets)?;
        if targets.count() == 0 {
            return None;
        }
        let exclusions = ServerVisualUnorderedCollection::of(&exclusions);
        let mut current = visual.weak().upgrade();
        let mut depth = 0;
        while let Some(candidate) = current {
            if exclusions.is_some_and(|exclusions| exclusions.contains(&candidate)) {
                return None;
            }
            if targets.contains(&candidate) {
                return Some(candidate);
            }
            depth += 1;
            if depth > MAX_LIGHT_DEPTH {
                return None;
            }
            current = candidate.parent();
        }
        None
    }
}

crate::ferro_thread_local! {
    /// The server-side lights of this (render) thread: what the render walk
    /// asks which lights reach a visual.
    static LIGHTS: RefCell<Vec<Weak<dyn IServerCompositionLight>>> = RefCell::new(Vec::new());
}

fn register_light(light: Weak<dyn IServerCompositionLight>) {
    LIGHTS.with(|lights| {
        let mut lights = lights.borrow_mut();
        lights.retain(|light| light.strong_count() > 0);
        lights.push(light);
    });
}

/// The lights of this thread that are alive, not disposed and enabled.
fn enabled_lights() -> Vec<Rc<dyn IServerCompositionLight>> {
    LIGHTS
        .try_with(|lights| {
            lights.try_borrow().map_or_else(
                |_| Vec::new(),
                |lights| {
                    lights
                        .iter()
                        .filter_map(Weak::upgrade)
                        .filter(|light| !light.is_disposed() && light.light_props().is_enabled())
                        .collect()
                },
            )
        })
        .unwrap_or_default()
}

/// The transform from the coordinates of `visual` to those of the root of
/// its tree: its own transform and those of its ancestors.
fn to_root(visual: &ServerCompositionVisual) -> Matrix {
    let mut matrix = visual.own_transform().unwrap_or(Matrix::IDENTITY);
    let mut parent = visual.parent();
    let mut depth = 0;
    while let Some(current) = parent {
        if let Some(transform) = current.own_transform() {
            matrix = matrix * transform;
        }
        depth += 1;
        if depth > MAX_LIGHT_DEPTH {
            break;
        }
        parent = current.parent();
    }
    matrix
}

/// The lights that reach `visual`, as a light map in the coordinates of the
/// visual; `None` when no enabled light reaches it.
pub fn light_map_of(visual: &ServerCompositionVisual) -> Option<LightMap> {
    let lights = enabled_lights();
    if lights.is_empty() {
        return None;
    }
    let mut map = LightMap::new();
    let mut visual_to_root: Option<Matrix> = None;
    for light in lights {
        let Some(target) = light.target_of(visual) else { continue };
        let space = light.coordinate_space().unwrap_or(target);
        let to_root_of_visual = *visual_to_root.get_or_insert_with(|| to_root(visual));
        // A space that cannot be inverted maps nothing: the light is left out.
        let Some(from_root_to_space) = to_root(&space).try_invert() else { continue };
        map.add(light.source(), to_root_of_visual * from_root_to_space);
    }
    (!map.is_empty()).then_some(map)
}

/// Invalidates the content of `visual` and of its subtree.
fn invalidate_subtree(visual: &Rc<ServerCompositionVisual>) {
    let mut stack = vec![visual.clone()];
    while let Some(current) = stack.pop() {
        current.invalidate_content();
        if let Some(children) = current.children() {
            stack.extend(children.list().iter().cloned());
        }
    }
}

/// Re-renders what a light lights when the light changes: observes the
/// light itself (a render resource), which is invalidated when one of its
/// members changes (an animation included) or its collections change.
struct LightObserver {
    light: Weak<dyn IServerCompositionLight>,
}

impl IServerRenderResourceObserver for LightObserver {
    fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
        if let Some(light) = self.light.upgrade() {
            light.light_state().invalidate_lit_visuals(&*light);
        }
    }
}

/// The bookkeeping of a light: the observer of its changes and the targets
/// it lit, whose subtrees are drawn again when the light changes.
#[derive(Default)]
pub struct ServerLightState {
    observer: RefCell<Option<Rc<dyn IServerRenderResourceObserver>>>,
    lit_targets: RefCell<Vec<Weak<ServerCompositionVisual>>>,
}

impl ServerLightState {
    /// Registers a new light on this thread and observes its changes.
    fn attach(base: &ServerRenderResource, state: &ServerLightState, light: Weak<dyn IServerCompositionLight>) {
        let observer: Rc<dyn IServerRenderResourceObserver> = Rc::new(LightObserver { light: light.clone() });
        base.core().add_observer(&observer);
        *state.observer.borrow_mut() = Some(observer);
        register_light(light);
    }

    /// The targets the light lit when it last changed.
    pub fn lit_targets(&self) -> Vec<Rc<ServerCompositionVisual>> {
        self.lit_targets.borrow().iter().filter_map(Weak::upgrade).collect()
    }

    /// Draws again the subtrees of the targets the light lit and of those it
    /// lights now (none once it is disposed).
    fn invalidate_lit_visuals(&self, light: &dyn IServerCompositionLight) {
        let current = if light.is_disposed() {
            Vec::new()
        } else {
            let targets = light.light_props().targets();
            ServerVisualUnorderedCollection::of(&targets).map(|targets| targets.visuals()).unwrap_or_default()
        };
        let previous = std::mem::take(&mut *self.lit_targets.borrow_mut());
        for visual in previous.iter().filter_map(Weak::upgrade).chain(current.iter().cloned()) {
            invalidate_subtree(&visual);
        }
        *self.lit_targets.borrow_mut() = current.iter().map(Rc::downgrade).collect();
    }

    /// The light is disposed: what it lit is drawn without it.
    fn detach(&self) {
        let previous = std::mem::take(&mut *self.lit_targets.borrow_mut());
        for visual in previous.iter().filter_map(Weak::upgrade) {
            invalidate_subtree(&visual);
        }
        self.observer.borrow_mut().take();
    }
}

/// The visual of an object property, as a server visual.
fn visual_of(object: Option<Rc<dyn IServerObject>>) -> Option<Rc<ServerCompositionVisual>> {
    object.and_then(|visual| visual.into_any_rc().downcast::<ServerCompositionVisual>().ok())
}

fn attenuation_of(constant: f32, linear: f32, quadratic: f32, min_cutoff: f32, max_cutoff: f32) -> LightAttenuation {
    LightAttenuation {
        constant: constant as f64,
        linear: linear as f64,
        quadratic: quadratic as f64,
        min_cutoff: min_cutoff as f64,
        max_cutoff: max_cutoff as f64,
    }
}

/// Declares a server-side light class: the struct, its constructor, the
/// render resource contracts, `IServerObject` and
/// [`IServerCompositionLight`] over the coordinate space and the source of
/// its properties.
macro_rules! server_light {
    (
        $(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident,
        coordinate_space: $coordinate_space:expr,
        source: $source:expr $(,)?
    ) => {
        $(#[$meta])*
        pub struct $name {
            base: ServerRenderResource,
            props: $props,
            state: ServerLightState,
        }

        impl $name {
            /// Creates the light, active and registered with the lights of
            /// the thread: the UI-thread light activates its server object
            /// when it is created.
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                let light = Rc::new_cyclic(|this: &Weak<$name>| $name {
                    base: ServerRenderResource::new(
                        compositor,
                        this.clone() as Weak<dyn IAnimatedServerObject>,
                        this.clone() as Weak<dyn IServerRenderResource>,
                    ),
                    props: $props::new(),
                    state: ServerLightState::default(),
                });
                light.base.object().activate();
                let weak = Rc::downgrade(&light) as Weak<dyn IServerCompositionLight>;
                ServerLightState::attach(&light.base, &light.state, weak);
                light
            }

            /// The properties of the light.
            pub fn props(&self) -> &$props {
                &self.props
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }
        }

        impl_server_render_resource!($name, base, composition_property: $props::get_composition_property);

        impl ServerCompositionLightHooks for $name {}

        impl $hooks for $name {}

        impl IServerCompositionLight for $name {
            fn light_props(&self) -> &ServerCompositionLightProps {
                self.props.base()
            }

            fn light_state(&self) -> &ServerLightState {
                &self.state
            }

            fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }

            fn coordinate_space(&self) -> Option<Rc<ServerCompositionVisual>> {
                let coordinate_space: fn(&$props) -> Option<Rc<dyn IServerObject>> = $coordinate_space;
                visual_of(coordinate_space(&self.props))
            }

            fn source(&self) -> LightSource {
                let source: fn(&$props) -> LightSource = $source;
                source(&self.props)
            }
        }

        impl IServerObject for $name {
            fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
                self.props.deserialize_changes_core(self, reader, committed_at);
            }

            fn values_invalidated(&self) {
                self.base.core().invalidated(self);
            }

            fn dispose(&self) {
                self.state.detach();
                self.base.core().dispose();
            }

            fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
                Some(self)
            }

            fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
                Some(self)
            }

            fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
                self.props.find_props(type_id)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
                self
            }
        }
    };
}

server_light!(
    /// The server-side counterpart of an
    /// [`AmbientLight`](crate::rendering::composition::AmbientLight).
    ServerAmbientLight,
    ServerAmbientLightProps,
    ServerAmbientLightHooks,
    coordinate_space: |_props| None,
    source: |props| LightSource::Ambient { color: LightColor::from_color(props.color(), props.intensity() as f64) },
);

server_light!(
    /// The server-side counterpart of a
    /// [`DistantLight`](crate::rendering::composition::DistantLight).
    ServerDistantLight,
    ServerDistantLightProps,
    ServerDistantLightHooks,
    coordinate_space: |props| props.coordinate_space(),
    source: |props| LightSource::Distant {
        color: LightColor::from_color(props.color(), props.intensity() as f64),
        direction: props.direction(),
    },
);

server_light!(
    /// The server-side counterpart of a
    /// [`PointLight`](crate::rendering::composition::PointLight).
    ServerPointLight,
    ServerPointLightProps,
    ServerPointLightHooks,
    coordinate_space: |props| props.coordinate_space(),
    source: |props| LightSource::Point {
        color: LightColor::from_color(props.color(), props.intensity() as f64),
        position: props.offset(),
        attenuation: attenuation_of(
            props.constant_attenuation(),
            props.linear_attenuation(),
            props.quadratic_attenuation(),
            props.min_attenuation_cutoff(),
            props.max_attenuation_cutoff(),
        ),
    },
);

server_light!(
    /// The server-side counterpart of a
    /// [`SpotLight`](crate::rendering::composition::SpotLight).
    ServerSpotLight,
    ServerSpotLightProps,
    ServerSpotLightHooks,
    coordinate_space: |props| props.coordinate_space(),
    source: |props| LightSource::Spot {
        inner_color: LightColor::from_color(props.inner_cone_color(), props.inner_cone_intensity() as f64),
        outer_color: LightColor::from_color(props.outer_cone_color(), props.outer_cone_intensity() as f64),
        position: props.offset(),
        direction: props.direction(),
        inner_cone_angle: props.inner_cone_angle() as f64,
        outer_cone_angle: props.outer_cone_angle() as f64,
        attenuation: attenuation_of(
            props.constant_attenuation(),
            props.linear_attenuation(),
            props.quadratic_attenuation(),
            props.min_attenuation_cutoff(),
            props.max_attenuation_cutoff(),
        ),
    },
);

// --- the lighting of the content of a visual -----------------------------------------

/// The lighting of the own content of a visual in the render walk: begun
/// before the content is drawn, ended after it (before the children).
pub struct LitContent {
    bitmap: Arc<SharedBitmapImpl>,
    rect: Rect,
    size: PixelSize,
}

impl LitContent {
    /// When enabled lights reach `visual`, pushes the layer the content is
    /// drawn into and returns the light map to draw over it; `None` (nothing
    /// pushed) when no light reaches the visual, it has no content bounds,
    /// the light is full everywhere, the compositor has no render interface
    /// for the bitmap of the map, or the visual is a layer visual (whose
    /// layer stays open for its children, which are lit each by itself).
    pub fn begin(visual: &ServerCompositionVisual, canvas: &mut dyn IDrawingContextImpl) -> Option<LitContent> {
        if visual.content_as::<ServerLayerVisual>().is_some() {
            return None;
        }
        let map = light_map_of(visual)?;
        let rect = visual.compute_own_content_bounds()?.to_rect();
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return None;
        }
        let size = LightMap::size_for(rect);
        let pixels = map.pixels(rect, size)?;
        let compositor = visual.compositor()?;
        let bitmap = compositor.render_interface().with_platform_render_interface(|render_interface| {
            render_interface.load_bitmap_from_pixels(
                PixelFormats::RGBA8888,
                AlphaFormat::Premul,
                &pixels,
                size,
                Vector::new(96.0, 96.0),
                size.width * 4,
            )
        })?;
        canvas.push_layer(rect);
        Some(LitContent { bitmap, rect, size })
    }

    /// Draws the light map over the content and pops the layer.
    pub fn end(self, canvas: &mut dyn IDrawingContextImpl) {
        canvas.push_render_options(RenderOptions {
            bitmap_blending_mode: BitmapBlendingMode::SourceAtop,
            ..RenderOptions::default()
        });
        let source = Rect::new(0.0, 0.0, self.size.width as f64, self.size.height as f64);
        canvas.draw_bitmap(&*self.bitmap, 1.0, source, self.rect);
        canvas.pop_render_options();
        canvas.pop_layer();
    }
}

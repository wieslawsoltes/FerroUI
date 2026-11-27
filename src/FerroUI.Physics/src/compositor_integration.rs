//! First-class integration between `ferroui-physics` and FerroUI's composition
//! engine (`ferroui_base::rendering::composition::ICompositionSimulation`) plus
//! physics-driven GPU WGSL/SkSL custom shader effects.
//!
//! # Highlights
//! - **[`CompositorPhysicsSimulation`]**: implements
//!   [`ICompositionSimulation`](ferroui_base::rendering::composition::ICompositionSimulation)
//!   so physics runs natively on FerroUI's `Compositor` frame loop with zero
//!   `DispatcherTimer` overhead and zero UI-thread layout passes.
//! - **[`PhysicsKineticGlassEffect`]**: a GPU runtime shader (`WGSL` for Apple
//!   Metal / Vello + `SkSL` for Skia + CPU fallback) whose specular light sweep,
//!   kinetic rim glow, and directional micro-refraction are driven directly by
//!   live rigid-body/spring velocity, tilt, and energy.
//! - **[`PhysicsImpactRippleEffect`]**: a GPU runtime shader driven by real
//!   physics collision contacts, shockwaves, and pointer impulses — rendering
//!   radial surface refraction, chromatic dispersion, and glowing wavefront rings
//!   directly on any FerroUI `Visual` or `CompositionVisual`.

use crate::body::{BodyBuilder, BodyHandle};
use crate::material::PhysicsMaterial;
use crate::math::Vec2;
use crate::queries::CollisionEvent;
use crate::shapes::Shape2D;
use crate::solver::SolverConfig;
use crate::ui::{Spring1D, Spring2D, SpringConfig};
use crate::world::PhysicsWorld;
use ferroui_base::media::effects::{
    CustomEffect, EffectParameterValue, IImmutableEffect, ImmutableCustomEffect, ShaderEffectDescription,
};
use ferroui_base::media::Color;
use ferroui_base::rendering::composition::{
    CompositionSimulationContext, CompositionSimulationHandle, Compositor, ICompositionSimulation,
    SimulationEffectUpdate, SimulationMode, SimulationStepResult, SimulationTarget, SimulationVisualPatch,
};
use ferroui_base::{Point, Vector};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Physics-Driven GPU Shader #1: PhysicsKineticGlassEffect
// ---------------------------------------------------------------------------

const KINETIC_GLASS_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float2 velocity;
uniform float energy;
uniform float tilt;
uniform float phase;
uniform float4 accent;
uniform shader content;

half4 sample_clamped(float2 p) {
    float2 hi = max(layer_size - 1.0, float2(0.0));
    float2 q = clamp(floor(p), float2(0.0), hi) + 0.5;
    return content.eval(q);
}

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float2 uv = coord / size;
    float e = clamp(energy, 0.0, 1.0);
    if (e < 0.002) {
        return sample_clamped(coord);
    }

    float vlen = length(velocity);
    float2 dir = vlen > 0.001 ? (velocity / vlen) : float2(cos(tilt), sin(tilt));
    float shift = e * 2.8 * layer_scale;

    half4 base = sample_clamped(coord);
    half4 r_s = sample_clamped(coord + dir * shift);
    half4 b_s = sample_clamped(coord - dir * shift);
    half4 out_c = half4(
        mix(base.r, r_s.r, half(e * 0.55)),
        base.g,
        mix(base.b, b_s.b, half(e * 0.55)),
        base.a
    );

    // Velocity- & tilt-coupled iridescent specular sweep across the visual.
    float angle = atan(dir.y, dir.x) * 0.45 + tilt * 0.7;
    float2 sweep_axis = float2(cos(angle), sin(angle));
    float diag = dot(uv - 0.5, sweep_axis);
    float sweep_pos = clamp(dot(velocity, float2(0.00055, 0.00045)) + sin(phase) * 0.22, -0.55, 0.55);
    float band = exp(-((diag - sweep_pos) * (diag - sweep_pos)) * 28.0);

    // Kinetic perimeter edge glow.
    float2 edge_d = min(uv, 1.0 - uv);
    float edge = 1.0 - smoothstep(0.0, 0.14, min(edge_d.x, edge_d.y));

    float glow_strength = e * (band * 0.42 + edge * 0.34);
    float3 sheen_rgb = mix(float3(1.0, 1.0, 1.0), accent.rgb / max(accent.a, 0.001), 0.52);
    float alpha = float(out_c.a);
    float3 lit = clamp(float3(out_c.rgb) + sheen_rgb * glow_strength * alpha, 0.0, alpha);
    return half4(half3(lit), out_c.a);
}
"#;

const KINETIC_GLASS_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    velocity: vec2<f32>,
    energy: f32,
    tilt: f32,
    phase: f32,
    accent: vec4<f32>,
}

fn sample_clamped(p: vec2<f32>) -> vec4<f32> {
    let hi = max(uniforms.layer_size - vec2<f32>(1.0), vec2<f32>(0.0));
    let q = clamp(floor(p), vec2<f32>(0.0), hi) + vec2<f32>(0.5);
    return sample_content(q);
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let uv = coord / size;
    let e = clamp(uniforms.energy, 0.0, 1.0);
    if (e < 0.002) {
        return sample_clamped(coord);
    }

    let vlen = length(uniforms.velocity);
    let fallback_dir = vec2<f32>(cos(uniforms.tilt), sin(uniforms.tilt));
    let dir = select(fallback_dir, uniforms.velocity / max(vlen, 0.001), vlen > 0.001);
    let shift = e * 2.8 * uniforms.layer_scale;

    let base = sample_clamped(coord);
    let r_s = sample_clamped(coord + dir * shift);
    let b_s = sample_clamped(coord - dir * shift);
    let out_c = vec4<f32>(
        mix(base.r, r_s.r, e * 0.55),
        base.g,
        mix(base.b, b_s.b, e * 0.55),
        base.a,
    );

    // Velocity- & tilt-coupled iridescent specular sweep across the visual.
    let angle = atan2(dir.y, dir.x) * 0.45 + uniforms.tilt * 0.7;
    let sweep_axis = vec2<f32>(cos(angle), sin(angle));
    let diag = dot(uv - vec2<f32>(0.5), sweep_axis);
    let sweep_pos = clamp(
        dot(uniforms.velocity, vec2<f32>(0.00055, 0.00045)) + sin(uniforms.phase) * 0.22,
        -0.55,
        0.55,
    );
    let d_band = diag - sweep_pos;
    let band = exp(-(d_band * d_band) * 28.0);

    // Kinetic perimeter edge glow.
    let edge_d = min(uv, vec2<f32>(1.0) - uv);
    let edge = 1.0 - smoothstep(0.0, 0.14, min(edge_d.x, edge_d.y));

    let glow_strength = e * (band * 0.42 + edge * 0.34);
    let accent_rgb = uniforms.accent.rgb / max(uniforms.accent.a, 0.001);
    let sheen_rgb = mix(vec3<f32>(1.0, 1.0, 1.0), accent_rgb, 0.52);
    let alpha = out_c.a;
    let lit = clamp(out_c.rgb + sheen_rgb * (glow_strength * alpha), vec3<f32>(0.0), vec3<f32>(alpha));
    return vec4<f32>(lit, alpha);
}
"#;

/// Physics-driven kinetic glass GPU shader effect (`WGSL` + `SkSL` + CPU reference).
///
/// Couples live physics velocity, kinetic energy, and tilt to a dynamic specular
/// light sweep, kinetic perimeter glow, and subtle directional chromatic micro-shift.
/// When `energy == 0.0`, the effect is a zero-distortion identity pass-through.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsKineticGlassEffect {
    /// Visual velocity in DIPs/s driving sheen direction & micro-shift.
    pub velocity: Vector,
    /// Normalized kinetic energy (`0.0..=1.0`).
    pub energy: f64,
    /// Tilt / angular rotation in radians.
    pub tilt: f64,
    /// Continuous phase (radians) for subtle sheen shimmer.
    pub phase: f64,
    /// Accent tint color for specular glow and kinetic rim.
    pub accent: Color,
}

impl Default for PhysicsKineticGlassEffect {
    fn default() -> Self {
        Self {
            velocity: Vector::new(0.0, 0.0),
            energy: 0.0,
            tilt: 0.0,
            phase: 0.0,
            accent: Color::from_argb(255, 56, 189, 248),
        }
    }
}

impl CustomEffect for PhysicsKineticGlassEffect {
    const NAME: &'static str = "physics-kinetic-glass";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("velocity", self.velocity.into());
        f("energy", self.energy.into());
        f("tilt", self.tilt.into());
        f("phase", self.phase.into());
        f("accent", self.accent.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "velocity" => value.as_vector().map(|v| self.velocity = v).is_some(),
            "energy" => value.as_float().map(|v| self.energy = v).is_some(),
            "tilt" => value.as_float().map(|v| self.tilt = v).is_some(),
            "phase" => value.as_float().map(|v| self.phase = v).is_some(),
            "accent" => value.as_color().map(|v| self.accent = v).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self::default())
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(KINETIC_GLASS_SKSL, KINETIC_GLASS_WGSL)
                .with_sample_radius(12.0),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let e = self.energy.clamp(0.0, 1.0) as f32;
        if width == 0 || height == 0 || e < 0.002 {
            return true;
        }
        let ar = f32::from(self.accent.r) / 255.0;
        let ag = f32::from(self.accent.g) / 255.0;
        let ab = f32::from(self.accent.b) / 255.0;
        let sheen = [1.0 * 0.48 + ar * 0.52, 1.0 * 0.48 + ag * 0.52, 1.0 * 0.48 + ab * 0.52];
        let inv_w = 1.0 / (width as f32).max(1.0);
        let inv_h = 1.0 / (height as f32).max(1.0);

        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            for x in 0..width {
                let u = (x as f32 + 0.5) * inv_w;
                let idx = (y * width + x) * 4;
                if idx + 3 >= pixels.len() {
                    break;
                }
                let a = f32::from(pixels[idx + 3]) / 255.0;
                if a <= 0.0 {
                    continue;
                }
                let edge_min = u.min(1.0 - u).min(v.min(1.0 - v));
                let edge = 1.0 - (edge_min / 0.14).clamp(0.0, 1.0);
                let g = e * edge * 0.34 * a;
                for ch in 0..3 {
                    let c = f32::from(pixels[idx + ch]) / 255.0;
                    pixels[idx + ch] = ((c + sheen[ch] * g).clamp(0.0, a) * 255.0).round() as u8;
                }
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Physics-Driven GPU Shader #2: PhysicsImpactRippleEffect
// ---------------------------------------------------------------------------

const IMPACT_RIPPLE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float2 origin;
uniform float progress;
uniform float amplitude;
uniform float ring_width;
uniform float4 accent;
uniform shader content;

half4 sample_clamped(float2 p) {
    float2 hi = max(layer_size - 1.0, float2(0.0));
    float2 q = clamp(floor(p), float2(0.0), hi) + 0.5;
    return content.eval(q);
}

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float prog = clamp(progress, 0.0, 1.0);
    float amp = clamp(amplitude, 0.0, 1.0);
    if (amp < 0.002 || prog <= 0.001 || prog >= 0.999) {
        return sample_clamped(coord);
    }

    float2 uv = coord / size;
    float aspect = size.x / size.y;
    float2 delta = float2((uv.x - origin.x) * aspect, uv.y - origin.y);
    float dist = length(delta);
    float2 dir = dist > 0.0001 ? (delta / dist) : float2(1.0, 0.0);

    float radius = prog * max(aspect, 1.0) * 1.15;
    float width = max(ring_width, 0.03);
    float diff = dist - radius;
    float envelope = exp(-((diff * diff) / (width * width))) * (1.0 - prog) * amp;
    float wave = sin(diff * 36.0) * envelope;

    float2 disp = dir * (wave * 11.0 * layer_scale);
    float2 chroma = dir * (envelope * 3.0 * layer_scale);

    half4 base = sample_clamped(coord);
    half4 r_c = sample_clamped(coord + disp + chroma);
    half4 g_c = sample_clamped(coord + disp);
    half4 b_c = sample_clamped(coord + disp - chroma);
    half4 warped = half4(r_c.r, g_c.g, b_c.b, base.a);

    float3 ring_rgb = mix(float3(1.0, 1.0, 1.0), accent.rgb / max(accent.a, 0.001), 0.65);
    float alpha = float(base.a);
    float glow = envelope * 0.68;
    float3 lit = clamp(float3(warped.rgb) + ring_rgb * (glow * alpha), 0.0, alpha);
    return half4(half3(lit), base.a);
}
"#;

const IMPACT_RIPPLE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    origin: vec2<f32>,
    progress: f32,
    amplitude: f32,
    ring_width: f32,
    accent: vec4<f32>,
}

fn sample_clamped(p: vec2<f32>) -> vec4<f32> {
    let hi = max(uniforms.layer_size - vec2<f32>(1.0), vec2<f32>(0.0));
    let q = clamp(floor(p), vec2<f32>(0.0), hi) + vec2<f32>(0.5);
    return sample_content(q);
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let prog = clamp(uniforms.progress, 0.0, 1.0);
    let amp = clamp(uniforms.amplitude, 0.0, 1.0);
    if (amp < 0.002 || prog <= 0.001 || prog >= 0.999) {
        return sample_clamped(coord);
    }

    let uv = coord / size;
    let aspect = size.x / size.y;
    let delta = vec2<f32>((uv.x - uniforms.origin.x) * aspect, uv.y - uniforms.origin.y);
    let dist = length(delta);
    let dir = select(vec2<f32>(1.0, 0.0), delta / max(dist, 0.0001), dist > 0.0001);

    let radius = prog * max(aspect, 1.0) * 1.15;
    let width = max(uniforms.ring_width, 0.03);
    let diff = dist - radius;
    let envelope = exp(-((diff * diff) / (width * width))) * (1.0 - prog) * amp;
    let wave = sin(diff * 36.0) * envelope;

    let disp = dir * (wave * 11.0 * uniforms.layer_scale);
    let chroma = dir * (envelope * 3.0 * uniforms.layer_scale);

    let base = sample_clamped(coord);
    let r_c = sample_clamped(coord + disp + chroma);
    let g_c = sample_clamped(coord + disp);
    let b_c = sample_clamped(coord + disp - chroma);
    let warped = vec4<f32>(r_c.r, g_c.g, b_c.b, base.a);

    let accent_rgb = uniforms.accent.rgb / max(uniforms.accent.a, 0.001);
    let ring_rgb = mix(vec3<f32>(1.0, 1.0, 1.0), accent_rgb, 0.65);
    let alpha = base.a;
    let glow = envelope * 0.68;
    let lit = clamp(warped.rgb + ring_rgb * (glow * alpha), vec3<f32>(0.0), vec3<f32>(alpha));
    return vec4<f32>(lit, alpha);
}
"#;

/// Physics-driven radial collision & shockwave GPU shader effect (`WGSL` + `SkSL` + CPU).
///
/// Triggered automatically by rigid-body collisions, shockwave blasts, or pointer
/// impulses inside [`CompositorPhysicsSimulation`], distorting the visual's content
/// with an expanding refractive ring and chromatic wavefront glow.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsImpactRippleEffect {
    /// Normalized impact origin across the visual (`0..1`, `0..1`).
    pub origin: Point,
    /// Wavefront expansion progress (`0.0..=1.0`).
    pub progress: f64,
    /// Shockwave strength (`0.0..=1.0`), typically scaled by collision impulse.
    pub amplitude: f64,
    /// Gaussian ring width in normalized coordinates (default `0.13`).
    pub ring_width: f64,
    /// Luminous wavefront tint color.
    pub accent: Color,
}

impl Default for PhysicsImpactRippleEffect {
    fn default() -> Self {
        Self {
            origin: Point::new(0.5, 0.5),
            progress: 0.0,
            amplitude: 0.0,
            ring_width: 0.13,
            accent: Color::from_argb(255, 34, 211, 238),
        }
    }
}

impl CustomEffect for PhysicsImpactRippleEffect {
    const NAME: &'static str = "physics-impact-ripple";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("origin", self.origin.into());
        f("progress", self.progress.into());
        f("amplitude", self.amplitude.into());
        f("ring-width", self.ring_width.into());
        f("accent", self.accent.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "origin" => value.as_point().map(|v| self.origin = v).is_some(),
            "progress" => value.as_float().map(|v| self.progress = v).is_some(),
            "amplitude" => value.as_float().map(|v| self.amplitude = v).is_some(),
            "ring-width" => value.as_float().map(|v| self.ring_width = v).is_some(),
            "accent" => value.as_color().map(|v| self.accent = v).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self::default())
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(IMPACT_RIPPLE_SKSL, IMPACT_RIPPLE_WGSL)
                .with_sample_radius(18.0),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let prog = self.progress.clamp(0.0, 1.0) as f32;
        let amp = self.amplitude.clamp(0.0, 1.0) as f32;
        if width == 0 || height == 0 || amp < 0.002 || prog <= 0.001 || prog >= 0.999 {
            return true;
        }
        let aspect = width as f32 / (height as f32).max(1.0);
        let radius = prog * aspect.max(1.0) * 1.15;
        let ring_w = (self.ring_width as f32).max(0.03);
        let ar = f32::from(self.accent.r) / 255.0;
        let ag = f32::from(self.accent.g) / 255.0;
        let ab = f32::from(self.accent.b) / 255.0;
        let ring_rgb = [0.35 + ar * 0.65, 0.35 + ag * 0.65, 0.35 + ab * 0.65];

        for y in 0..height {
            let v = (y as f32 + 0.5) / height as f32;
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let idx = (y * width + x) * 4;
                if idx + 3 >= pixels.len() {
                    break;
                }
                let a = f32::from(pixels[idx + 3]) / 255.0;
                if a <= 0.0 {
                    continue;
                }
                let dx = (u - self.origin.x as f32) * aspect;
                let dy = v - self.origin.y as f32;
                let diff = (dx * dx + dy * dy).sqrt() - radius;
                let env = (-(diff * diff) / (ring_w * ring_w)).exp() * (1.0 - prog) * amp;
                let g = env * 0.68 * a;
                for ch in 0..3 {
                    let c = f32::from(pixels[idx + ch]) / 255.0;
                    pixels[idx + ch] = ((c + ring_rgb[ch] * g).clamp(0.0, a) * 255.0).round() as u8;
                }
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// CompositorPhysicsSimulation + Controller
// ---------------------------------------------------------------------------

/// High-level physics driving strategy inside [`CompositorPhysicsSimulation`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsCompositorBehavior {
    /// Full 2D rigid-body simulation with gravity, container walls, collisions,
    /// impulses, and collision-triggered GPU shockwave shaders.
    RigidBody,
    /// Smooth analytical UI spring choreography (`Spring2D` + rotational tilt +
    /// squash-and-stretch + velocity-driven kinetic glass shader).
    ElasticSprings,
}

/// Which physics-driven GPU shader pipeline is attached to each bound visual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsShaderMode {
    /// Automatically switches between impact ripples on collisions/clicks and
    /// kinetic glass sheen during motion.
    ReactiveAuto,
    /// Velocity/tilt-driven kinetic glass shader only.
    KineticGlassOnly,
    /// Impact/collision radial ripple shader only.
    ImpactRippleOnly,
    /// Pure composition transforms with no GPU shader effect.
    Disabled,
}

/// Live telemetry snapshot exposed by [`CompositorPhysicsController::telemetry`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CompositorPhysicsTelemetry {
    pub bound_visuals: usize,
    pub awake_bodies: usize,
    pub active_ripples: usize,
    pub peak_speed_px: f32,
    pub peak_energy: f32,
    pub total_collisions: u64,
    pub step_count: u64,
    pub behavior: Option<PhysicsCompositorBehavior>,
    pub shader_mode: Option<PhysicsShaderMode>,
}

struct ActiveRipple {
    origin: Point,
    progress: f32,
    amplitude: f32,
    speed: f32,
}

struct BoundVisualEntry {
    target: SimulationTarget,
    /// Layout rest center in container DIPs (used to compute relative `translation`).
    rest_center: Vec2,
    /// Visual width & height in DIPs.
    size: Vec2,
    accent: Color,
    body: BodyHandle,
    /// Spring-mode state (relative offset from `rest_center`, in DIPs).
    spring_pos: Spring2D,
    spring_tilt: Spring1D,
    spring_scale: Spring1D,
    /// Dragging state.
    dragging: bool,
    drag_velocity: Vec2,
    /// Active impact shader ripple on this visual.
    ripple: Option<ActiveRipple>,
    /// Continuous phase for kinetic sheen.
    phase: f32,
    /// Last computed speed in DIPs/s.
    last_speed: f32,
}

struct SharedPhysicsState {
    pixels_per_meter: f32,
    arena_width: f32,
    arena_height: f32,
    behavior: PhysicsCompositorBehavior,
    shader_mode: PhysicsShaderMode,
    gravity: Vec2,
    world: PhysicsWorld,
    walls: Vec<BodyHandle>,
    entries: Vec<BoundVisualEntry>,
    total_collisions: u64,
    step_count: u64,
    compositor: Option<Weak<Compositor>>,
    dirty_wake: bool,
}

impl SharedPhysicsState {
    fn new(arena_width: f32, arena_height: f32, behavior: PhysicsCompositorBehavior) -> Self {
        let ppm = 64.0_f32;
        let gravity = Vec2::new(0.0, 12.5);
        let mut world = PhysicsWorld::new(gravity).with_config(SolverConfig::game_default());
        let walls = Self::create_arena_walls(&mut world, arena_width, arena_height, ppm);
        Self {
            pixels_per_meter: ppm,
            arena_width,
            arena_height,
            behavior,
            shader_mode: PhysicsShaderMode::ReactiveAuto,
            gravity,
            world,
            walls,
            entries: Vec::new(),
            total_collisions: 0,
            step_count: 0,
            compositor: None,
            dirty_wake: true,
        }
    }

    fn create_arena_walls(world: &mut PhysicsWorld, w_px: f32, h_px: f32, ppm: f32) -> Vec<BodyHandle> {
        let w = (w_px / ppm).max(2.0);
        let h = (h_px / ppm).max(2.0);
        let t = 0.5;
        let mat = PhysicsMaterial::new(0.0, 0.35, 0.55);
        let specs = [
            // floor
            (Vec2::new(w * 0.5, h + t * 0.5), w * 0.5 + t, t * 0.5),
            // ceiling
            (Vec2::new(w * 0.5, -t * 0.5), w * 0.5 + t, t * 0.5),
            // left
            (Vec2::new(-t * 0.5, h * 0.5), t * 0.5, h * 0.5 + t),
            // right
            (Vec2::new(w + t * 0.5, h * 0.5), t * 0.5, h * 0.5 + t),
        ];
        specs
            .into_iter()
            .map(|(pos, hx, hy)| {
                world.add_body(
                    BodyBuilder::static_body(Shape2D::rect(hx, hy))
                        .position(pos)
                        .material(mat),
                )
            })
            .collect()
    }

    fn rebuild_walls(&mut self, w_px: f32, h_px: f32) {
        if (w_px - self.arena_width).abs() < 2.0 && (h_px - self.arena_height).abs() < 2.0 {
            return;
        }
        self.arena_width = w_px.max(120.0);
        self.arena_height = h_px.max(120.0);
        for wall in self.walls.drain(..) {
            self.world.remove_body(wall);
        }
        self.walls = Self::create_arena_walls(&mut self.world, self.arena_width, self.arena_height, self.pixels_per_meter);
        self.wake();
    }

    fn wake(&mut self) {
        self.dirty_wake = true;
        if let Some(comp) = self.compositor.as_ref().and_then(Weak::upgrade) {
            comp.wake_simulations();
        }
    }

    fn px_to_world(&self, px: Vec2) -> Vec2 {
        px / self.pixels_per_meter
    }
}

/// Interactive control handle for a [`CompositorPhysicsSimulation`] registered on
/// FerroUI's [`Compositor`].
///
/// Clone freely into pointer handlers, toolbar buttons, or reactive telemetry panels.
#[derive(Clone)]
pub struct CompositorPhysicsController {
    inner: Rc<RefCell<SharedPhysicsState>>,
}

impl CompositorPhysicsController {
    /// Creates a `(CompositorPhysicsSimulation, CompositorPhysicsController)` pair.
    ///
    /// Register the simulation with `compositor.register_simulation(sim)` and use
    /// the controller to bind visuals, switch behavior/shader modes, drag, or fire
    /// shockwaves.
    pub fn create(
        name: impl Into<String>,
        arena_width: f32,
        arena_height: f32,
        behavior: PhysicsCompositorBehavior,
    ) -> (CompositorPhysicsSimulation, Self) {
        let inner = Rc::new(RefCell::new(SharedPhysicsState::new(
            arena_width.max(120.0),
            arena_height.max(120.0),
            behavior,
        )));
        (CompositorPhysicsSimulation { name: name.into(), inner: inner.clone() }, Self { inner })
    }

    /// Convenience helper that creates and immediately registers a
    /// [`CompositorPhysicsSimulation`] on `compositor`.
    pub fn attach_to_compositor(
        compositor: &Rc<Compositor>,
        name: impl Into<String>,
        arena_width: f32,
        arena_height: f32,
        behavior: PhysicsCompositorBehavior,
    ) -> (CompositionSimulationHandle, Self) {
        let (sim, ctrl) = Self::create(name, arena_width, arena_height, behavior);
        let handle = compositor.register_simulation(sim);
        (handle, ctrl)
    }

    /// Binds a FerroUI [`SimulationTarget`] (either a UI `Visual` or pure
    /// `CompositionVisual`) to this physics simulation. Returns its index.
    pub fn bind_visual(&self, target: SimulationTarget, rest_center: Vec2, size: Vec2, accent: Color) -> usize {
        let mut st = self.inner.borrow_mut();
        let pos_m = st.px_to_world(rest_center);
        let hx = (size.x * 0.5 / st.pixels_per_meter).max(0.15);
        let hy = (size.y * 0.5 / st.pixels_per_meter).max(0.15);
        let body = st.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(hx, hy))
                .position(pos_m)
                .material(PhysicsMaterial::new(1.2, 0.3, 0.62))
                .linear_damping(0.45)
                .angular_damping(0.9),
        );
        let idx = st.entries.len();
        st.entries.push(BoundVisualEntry {
            target,
            rest_center,
            size,
            accent,
            body,
            spring_pos: Spring2D::new(Vec2::ZERO, SpringConfig::bouncy()),
            spring_tilt: Spring1D::new(0.0, SpringConfig::snappy()),
            spring_scale: Spring1D::new(1.0, SpringConfig::bouncy()),
            dragging: false,
            drag_velocity: Vec2::ZERO,
            ripple: None,
            phase: idx as f32 * 1.15,
            last_speed: 0.0,
        });
        st.wake();
        idx
    }

    /// Updates container bounds (rebuilds static arena walls for rigid-body mode).
    pub fn set_arena_size(&self, width: f32, height: f32) {
        self.inner.borrow_mut().rebuild_walls(width, height);
    }

    /// Switches between [`PhysicsCompositorBehavior::RigidBody`] and
    /// [`PhysicsCompositorBehavior::ElasticSprings`] seamlessly while preserving
    /// current visual positions and velocities!
    pub fn set_behavior(&self, behavior: PhysicsCompositorBehavior) {
        let mut st = self.inner.borrow_mut();
        if st.behavior == behavior {
            return;
        }
        st.behavior = behavior;
        let ppm = st.pixels_per_meter;
        let mut sync_bodies: Vec<(BodyHandle, Vec2, Vec2)> = Vec::new();
        let mut sync_springs: Vec<(usize, Vec2, Vec2, f32, f32)> = Vec::new();
        for (idx, entry) in st.entries.iter().enumerate() {
            match behavior {
                PhysicsCompositorBehavior::ElasticSprings => {
                    if let Some(body) = st.world.body(entry.body) {
                        let rel = body.position * ppm - entry.rest_center;
                        sync_springs.push((
                            idx,
                            rel,
                            body.linear_velocity * ppm,
                            body.rotation.radians(),
                            body.angular_velocity,
                        ));
                    }
                }
                PhysicsCompositorBehavior::RigidBody => {
                    let world_pos = (entry.rest_center + entry.spring_pos.value()) / ppm;
                    let world_vel = entry.spring_pos.velocity() / ppm;
                    sync_bodies.push((entry.body, world_pos, world_vel));
                }
            }
        }
        for (idx, rel, vel, ang, ang_vel) in sync_springs {
            if let Some(entry) = st.entries.get_mut(idx) {
                entry.spring_pos.x.value = rel.x;
                entry.spring_pos.y.value = rel.y;
                entry.spring_pos.set_velocity(vel);
                entry.spring_pos.set_target(Vec2::ZERO);
                entry.spring_tilt.value = ang;
                entry.spring_tilt.velocity = ang_vel;
                entry.spring_tilt.target = 0.0;
            }
        }
        for (handle, pos, vel) in sync_bodies {
            if let Some(body) = st.world.body_mut(handle) {
                body.position = pos;
                body.linear_velocity = vel;
                body.wake_up();
            }
        }
        st.wake();
    }

    /// Returns the active physics behavior.
    pub fn behavior(&self) -> PhysicsCompositorBehavior {
        self.inner.borrow().behavior
    }

    /// Configures gravity in m/s² (e.g. `(0.0, 12.5)` for earth-like UI drop, `(0.0, 0.0)` for zero-G).
    pub fn set_gravity(&self, gravity: Vec2) {
        let mut st = self.inner.borrow_mut();
        st.gravity = gravity;
        st.world.gravity = gravity;
        let handles: Vec<BodyHandle> = st.entries.iter().map(|e| e.body).collect();
        for h in handles {
            if let Some(b) = st.world.body_mut(h) {
                b.wake_up();
            }
        }
        st.wake();
    }

    /// Returns current gravity vector in m/s².
    pub fn gravity(&self) -> Vec2 {
        self.inner.borrow().gravity
    }

    /// Selects which GPU shader effect mode is driven by the physics state.
    pub fn set_shader_mode(&self, mode: PhysicsShaderMode) {
        let mut st = self.inner.borrow_mut();
        st.shader_mode = mode;
        if mode == PhysicsShaderMode::Disabled {
            for entry in &st.entries {
                entry.target.apply(&SimulationVisualPatch::new().without_effect());
            }
        }
        st.wake();
    }

    /// Returns the active GPU shader mode.
    pub fn shader_mode(&self) -> PhysicsShaderMode {
        self.inner.borrow().shader_mode
    }

    /// Starts an interactive pointer drag on bound visual `index` and triggers
    /// a subtle tactile impact ripple at `normalized_origin`.
    pub fn begin_drag(&self, index: usize, normalized_origin: Point) {
        let mut st = self.inner.borrow_mut();
        let Some(entry) = st.entries.get_mut(index) else { return };
        entry.dragging = true;
        entry.drag_velocity = Vec2::ZERO;
        entry.spring_scale.target = 1.05;
        entry.ripple = Some(ActiveRipple {
            origin: Point::new(normalized_origin.x.clamp(0.0, 1.0), normalized_origin.y.clamp(0.0, 1.0)),
            progress: 0.02,
            amplitude: 0.72,
            speed: 1.55,
        });
        let handle = entry.body;
        if let Some(body) = st.world.body_mut(handle) {
            body.linear_velocity = Vec2::ZERO;
            body.angular_velocity = 0.0;
            body.wake_up();
        }
        st.wake();
    }

    /// Moves a dragged visual by `delta_px` (DIPs) and tracks pointer velocity.
    pub fn drag_by(&self, index: usize, delta_px: Vec2, dt: f32) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let Some(entry) = st.entries.get_mut(index) else { return };
        if !entry.dragging {
            return;
        }
        let inv_dt = 1.0 / dt.max(0.008);
        let inst_vel = delta_px * inv_dt;
        entry.drag_velocity = entry.drag_velocity * 0.55 + inst_vel * 0.45;
        let drag_vel = entry.drag_velocity;
        let handle = entry.body;

        match behavior {
            PhysicsCompositorBehavior::ElasticSprings => {
                let next = entry.spring_pos.value() + delta_px;
                entry.spring_pos.x.value = next.x;
                entry.spring_pos.y.value = next.y;
                entry.spring_pos.set_target(next);
                entry.spring_pos.set_velocity(drag_vel);
                entry.spring_tilt.target = (drag_vel.x * 0.00028).clamp(-0.26, 0.26);
            }
            PhysicsCompositorBehavior::RigidBody => {
                if let Some(body) = st.world.body_mut(handle) {
                    body.position += delta_px / ppm;
                    body.linear_velocity = drag_vel / ppm;
                    body.angular_velocity = (drag_vel.x * 0.0025).clamp(-4.0, 4.0);
                    body.wake_up();
                }
            }
        }
        st.wake();
    }

    /// Releases a dragged visual, imparting its fling momentum to the physics world
    /// or elastic spring!
    pub fn end_drag(&self, index: usize) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let Some(entry) = st.entries.get_mut(index) else { return };
        if !entry.dragging {
            return;
        }
        entry.dragging = false;
        entry.spring_scale.target = 1.0;
        let fling = entry.drag_velocity;
        let handle = entry.body;

        match behavior {
            PhysicsCompositorBehavior::ElasticSprings => {
                entry.spring_pos.set_target(Vec2::ZERO);
                entry.spring_pos.set_velocity(fling);
                entry.spring_tilt.target = 0.0;
            }
            PhysicsCompositorBehavior::RigidBody => {
                if let Some(body) = st.world.body_mut(handle) {
                    let v = fling / ppm;
                    let len = v.length();
                    body.linear_velocity = if len > 22.0 { v * (22.0 / len) } else { v };
                    body.wake_up();
                }
            }
        }
        st.wake();
    }

    /// Triggers an immediate GPU impact ripple shader + impulse on visual `index`.
    pub fn trigger_impact(&self, index: usize, normalized_origin: Point, intensity: f32, impulse_px: Vec2) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let Some(entry) = st.entries.get_mut(index) else { return };
        entry.ripple = Some(ActiveRipple {
            origin: Point::new(normalized_origin.x.clamp(0.0, 1.0), normalized_origin.y.clamp(0.0, 1.0)),
            progress: 0.02,
            amplitude: intensity.clamp(0.15, 1.0),
            speed: 1.35,
        });
        entry.spring_scale.velocity -= intensity * 1.4;
        let handle = entry.body;
        match behavior {
            PhysicsCompositorBehavior::ElasticSprings => {
                let next_v = entry.spring_pos.velocity() + impulse_px;
                entry.spring_pos.set_velocity(next_v);
                entry.spring_tilt.velocity += (impulse_px.x * 0.004).clamp(-3.0, 3.0);
            }
            PhysicsCompositorBehavior::RigidBody => {
                if let Some(body) = st.world.body_mut(handle) {
                    let mass = body.mass;
                    body.apply_impulse(impulse_px / ppm * mass);
                    body.apply_angular_impulse((impulse_px.x * 0.0025).clamp(-2.5, 2.5));
                    body.wake_up();
                }
            }
        }
        st.wake();
    }

    /// Blasts all bound visuals radially outward from `center_px` and fires
    /// synchronized GPU shockwave ripples across every affected visual!
    pub fn shockwave(&self, center_px: Vec2, strength: f32) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let count = st.entries.len();
        for idx in 0..count {
            let (handle, rest, spring_pos) = {
                let e = &st.entries[idx];
                (e.body, e.rest_center, e.spring_pos.value())
            };
            let current_px = match behavior {
                PhysicsCompositorBehavior::RigidBody => {
                    st.world.body(handle).map(|b| b.position * ppm).unwrap_or(rest)
                }
                PhysicsCompositorBehavior::ElasticSprings => rest + spring_pos,
            };
            let diff = current_px - center_px;
            let dist = diff.length().max(24.0);
            let dir = if diff.length_squared() > 1e-4 {
                diff / dist
            } else {
                Vec2::new(((idx as f32) * 1.9).cos(), ((idx as f32) * 1.9).sin())
            };
            let falloff = (1.0 - dist / 900.0).clamp(0.35, 1.0);
            let vel_boost = dir * (strength * falloff);

            let local_origin = Point::new(
                (0.5 - dir.x * 0.42).clamp(0.05, 0.95) as f64,
                (0.5 - dir.y * 0.42).clamp(0.05, 0.95) as f64,
            );

            if let Some(e) = st.entries.get_mut(idx) {
                e.ripple = Some(ActiveRipple {
                    origin: local_origin,
                    progress: 0.02,
                    amplitude: (0.7 + 0.3 * falloff).clamp(0.0, 1.0),
                    speed: 1.25,
                });
                e.spring_scale.velocity -= 1.8 * falloff;
                if behavior == PhysicsCompositorBehavior::ElasticSprings {
                    let nv = e.spring_pos.velocity() + vel_boost;
                    e.spring_pos.set_velocity(nv);
                    e.spring_tilt.velocity += (dir.x * 4.5 * falloff).clamp(-6.0, 6.0);
                }
            }
            if behavior == PhysicsCompositorBehavior::RigidBody {
                if let Some(body) = st.world.body_mut(handle) {
                    body.linear_velocity += vel_boost / ppm;
                    body.angular_velocity += (dir.x * 5.0 * falloff).clamp(-8.0, 8.0);
                    body.wake_up();
                }
            }
        }
        st.wake();
    }

    /// Snaps all visuals cleanly back to their rest layout positions with a crisp spring settle.
    pub fn snap_home(&self) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let resets: Vec<(BodyHandle, Vec2)> = st.entries.iter().map(|e| (e.body, e.rest_center / ppm)).collect();
        for (handle, pos) in resets {
            if let Some(body) = st.world.body_mut(handle) {
                body.position = pos;
                body.rotation = crate::math::Rot2::IDENTITY;
                body.linear_velocity = Vec2::ZERO;
                body.angular_velocity = 0.0;
                body.wake_up();
            }
        }
        for e in &mut st.entries {
            e.spring_pos.set_target(Vec2::ZERO);
            let cur = e.spring_pos.value();
            e.spring_pos.set_velocity(-cur * 6.0);
            e.spring_tilt.target = 0.0;
            e.spring_scale.target = 1.0;
            e.ripple = Some(ActiveRipple {
                origin: Point::new(0.5, 0.5),
                progress: 0.02,
                amplitude: 0.55,
                speed: 1.65,
            });
        }
        st.wake();
    }

    /// Returns a live telemetry snapshot for HUDs and diagnostics.
    pub fn telemetry(&self) -> CompositorPhysicsTelemetry {
        let st = self.inner.borrow();
        let awake_bodies = match st.behavior {
            PhysicsCompositorBehavior::RigidBody => st
                .entries
                .iter()
                .filter(|e| st.world.body(e.body).is_some_and(|b| !b.is_sleeping))
                .count(),
            PhysicsCompositorBehavior::ElasticSprings => st
                .entries
                .iter()
                .filter(|e| !e.spring_pos.is_settled() || e.dragging || e.ripple.is_some())
                .count(),
        };
        let active_ripples = st.entries.iter().filter(|e| e.ripple.is_some()).count();
        let peak_speed_px = st.entries.iter().map(|e| e.last_speed).fold(0.0_f32, f32::max);
        let peak_energy = (peak_speed_px / 650.0).clamp(0.0, 1.0);
        CompositorPhysicsTelemetry {
            bound_visuals: st.entries.len(),
            awake_bodies,
            active_ripples,
            peak_speed_px,
            peak_energy,
            total_collisions: st.total_collisions,
            step_count: st.step_count,
            behavior: Some(st.behavior),
            shader_mode: Some(st.shader_mode),
        }
    }

    /// Resets all bound visual targets back to identity (useful when switching away
    /// to another simulation mode on the same visuals).
    pub fn reset_visuals_to_identity(&self) {
        let st = self.inner.borrow();
        for entry in &st.entries {
            entry.target.reset_to_identity(true);
        }
    }
}

/// [`ICompositionSimulation`] implementation plugging `ferroui-physics` natively
/// into FerroUI's [`Compositor`] and visual effects/shader pipeline.
pub struct CompositorPhysicsSimulation {
    name: String,
    inner: Rc<RefCell<SharedPhysicsState>>,
}

impl ICompositionSimulation for CompositorPhysicsSimulation {
    fn mode(&self) -> SimulationMode {
        match self.inner.borrow().behavior {
            PhysicsCompositorBehavior::RigidBody => SimulationMode::Physics,
            PhysicsCompositorBehavior::ElasticSprings => SimulationMode::SpringDynamics,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn on_attached(&mut self, compositor: &Rc<Compositor>) {
        self.inner.borrow_mut().compositor = Some(Rc::downgrade(compositor));
    }

    fn on_detached(&mut self) {
        let mut st = self.inner.borrow_mut();
        st.compositor = None;
        for entry in &st.entries {
            entry.target.reset_to_identity(true);
        }
    }

    fn step(&mut self, ctx: &mut CompositionSimulationContext<'_>) -> SimulationStepResult {
        let dt = ctx.frame().dt as f32;
        let mut st = self.inner.borrow_mut();
        if st.entries.is_empty() {
            return SimulationStepResult::Idle;
        }
        st.step_count = st.step_count.wrapping_add(1);
        st.dirty_wake = false;

        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let shader_mode = st.shader_mode;
        let mut any_active = false;

        // 1. Step rigid-body physics world & harvest real collisions to trigger GPU impact shaders!
        if behavior == PhysicsCompositorBehavior::RigidBody {
            let dragged_handles: Vec<BodyHandle> =
                st.entries.iter().filter(|e| e.dragging).map(|e| e.body).collect();
            for h in &dragged_handles {
                if let Some(b) = st.world.body_mut(*h) {
                    b.linear_velocity = Vec2::ZERO;
                    b.angular_velocity = 0.0;
                }
            }

            st.world.step(dt);

            // Convert real physics collision events into per-visual GPU impact ripples!
            let collisions: Vec<CollisionEvent> = st.world.collision_events().to_vec();
            for ev in collisions {
                let CollisionEvent::Started { body_a, body_b, point, impact_speed, .. } = ev else {
                    continue;
                };
                if impact_speed < 0.75 {
                    continue;
                }
                st.total_collisions = st.total_collisions.wrapping_add(1);
                let contact_px = point * ppm;
                let intensity = (impact_speed / 8.0).clamp(0.35, 1.0);
                for body_handle in [body_a, body_b] {
                    let body_pos_px = st.world.body(body_handle).map(|b| b.position * ppm);
                    if let (Some(pos_px), Some(entry)) = (
                        body_pos_px,
                        st.entries.iter_mut().find(|e| e.body == body_handle),
                    ) {
                        let w = entry.size.x.max(1.0);
                        let h = entry.size.y.max(1.0);
                        let norm_x = ((contact_px.x - (pos_px.x - w * 0.5)) / w).clamp(0.05, 0.95) as f64;
                        let norm_y = ((contact_px.y - (pos_px.y - h * 0.5)) / h).clamp(0.05, 0.95) as f64;
                        let should_replace = entry
                            .ripple
                            .as_ref()
                            .is_none_or(|r| r.progress > 0.28 || intensity > r.amplitude);
                        if should_replace {
                            entry.ripple = Some(ActiveRipple {
                                origin: Point::new(norm_x, norm_y),
                                progress: 0.02,
                                amplitude: intensity,
                                speed: 1.45,
                            });
                        }
                    }
                }
            }
        }

        // 2. Push composition transforms + physics-driven GPU shader effects for every visual!
        let count = st.entries.len();
        for idx in 0..count {
            let (tx, ty, rot, sx, sy, vel_px, speed_px, awake_this) = match behavior {
                PhysicsCompositorBehavior::RigidBody => {
                    let entry = &mut st.entries[idx];
                    entry.spring_scale.step(dt);
                    let sc = entry.spring_scale.value.clamp(0.88, 1.14) as f64;
                    let handle = entry.body;
                    let rest = entry.rest_center;
                    let dragging = entry.dragging;
                    if let Some(body) = st.world.body(handle) {
                        let pos_px = body.position * ppm;
                        let vel = body.linear_velocity * ppm;
                        let spd = vel.length();
                        let awake = !body.is_sleeping || dragging || spd > 2.5 || body.angular_velocity.abs() > 0.02;
                        (
                            (pos_px.x - rest.x) as f64,
                            (pos_px.y - rest.y) as f64,
                            body.rotation.radians(),
                            sc,
                            sc,
                            vel,
                            spd,
                            awake,
                        )
                    } else {
                        (0.0, 0.0, 0.0, 1.0, 1.0, Vec2::ZERO, 0.0, false)
                    }
                }
                PhysicsCompositorBehavior::ElasticSprings => {
                    let entry = &mut st.entries[idx];
                    if !entry.dragging {
                        entry.spring_pos.step(dt);
                        entry.spring_tilt.target = (entry.spring_pos.velocity().x * 0.00024).clamp(-0.24, 0.24);
                    }
                    entry.spring_tilt.step(dt);
                    entry.spring_scale.step(dt);

                    let pos = entry.spring_pos.value();
                    let vel = entry.spring_pos.velocity();
                    let spd = vel.length();
                    let stretch = (spd / 2800.0).clamp(0.0, 0.075) as f64;
                    let base_s = entry.spring_scale.value.clamp(0.88, 1.14) as f64;
                    let sx = base_s * (1.0 + stretch * 0.65);
                    let sy = base_s * (1.0 - stretch * 0.45);
                    let awake = entry.dragging
                        || !entry.spring_pos.is_settled()
                        || !entry.spring_tilt.is_settled()
                        || !entry.spring_scale.is_settled();
                    (
                        pos.x as f64,
                        pos.y as f64,
                        entry.spring_tilt.value,
                        sx,
                        sy,
                        vel,
                        spd,
                        awake,
                    )
                }
            };

            let entry = &mut st.entries[idx];
            entry.last_speed = speed_px;
            entry.phase = (entry.phase + dt * (1.5 + speed_px * 0.006)) % std::f32::consts::TAU;

            // Advance active impact ripple wavefront.
            let mut ripple_active = false;
            if let Some(r) = &mut entry.ripple {
                r.progress += dt * r.speed;
                if r.progress >= 1.0 {
                    entry.ripple = None;
                } else {
                    ripple_active = true;
                }
            }

            if awake_this || ripple_active {
                any_active = true;
            }

            // Compute physics-driven GPU shader effect update.
            let energy = (speed_px / 620.0).clamp(0.0, 1.0);
            let effect_update = match shader_mode {
                PhysicsShaderMode::Disabled => SimulationEffectUpdate::Keep,
                PhysicsShaderMode::ImpactRippleOnly => {
                    if let Some(r) = &entry.ripple {
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            PhysicsImpactRippleEffect {
                                origin: r.origin,
                                progress: r.progress as f64,
                                amplitude: r.amplitude as f64,
                                ring_width: 0.14,
                                accent: entry.accent,
                            },
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else {
                        SimulationEffectUpdate::Clear
                    }
                }
                PhysicsShaderMode::KineticGlassOnly => {
                    if energy > 0.015 || entry.dragging {
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            PhysicsKineticGlassEffect {
                                velocity: Vector::new(vel_px.x as f64, vel_px.y as f64),
                                energy: if entry.dragging { energy.max(0.38) } else { energy } as f64,
                                tilt: rot as f64,
                                phase: entry.phase as f64,
                                accent: entry.accent,
                            },
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else {
                        SimulationEffectUpdate::Clear
                    }
                }
                PhysicsShaderMode::ReactiveAuto => {
                    if let Some(r) = &entry.ripple {
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            PhysicsImpactRippleEffect {
                                origin: r.origin,
                                progress: r.progress as f64,
                                amplitude: r.amplitude as f64,
                                ring_width: 0.14,
                                accent: entry.accent,
                            },
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else if energy > 0.015 || entry.dragging {
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            PhysicsKineticGlassEffect {
                                velocity: Vector::new(vel_px.x as f64, vel_px.y as f64),
                                energy: if entry.dragging { energy.max(0.38) } else { energy } as f64,
                                tilt: rot as f64,
                                phase: entry.phase as f64,
                                accent: entry.accent,
                            },
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else {
                        SimulationEffectUpdate::Clear
                    }
                }
            };

            let mut patch = SimulationVisualPatch::new()
                .with_translation_2d(tx, ty)
                .with_rotation_angle(rot)
                .with_scale_2d(sx, sy);
            patch.effect = effect_update;

            if let Some(sz) = entry.target.size() {
                if sz.x > 0.0 && sz.y > 0.0 {
                    patch = patch.with_center_point(sz.x * 0.5, sz.y * 0.5, 0.0);
                }
            } else {
                patch = patch.with_center_point(entry.size.x as f64 * 0.5, entry.size.y as f64 * 0.5, 0.0);
            }

            ctx.apply(&entry.target, &patch);
        }

        if any_active || st.dirty_wake {
            SimulationStepResult::Active
        } else {
            SimulationStepResult::Idle
        }
    }
}

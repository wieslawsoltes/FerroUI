//! First-class integration between `ferroui-physics` and FerroUI's composition
//! engine (`ferroui_base::rendering::composition::ICompositionSimulation`) plus
//! physics-driven GPU WGSL/SkSL custom shader effects.
//!
//! # Highlights
//! - **[`CompositorPhysicsSimulation`]**: implements
//!   [`ICompositionSimulation`](ferroui_base::rendering::composition::ICompositionSimulation)
//!   so physics runs natively on FerroUI's `Compositor` frame loop with zero
//!   `DispatcherTimer` overhead, zero UI-thread layout passes, zero draw-list
//!   re-recordings, and `O(1)` collision-to-visual dispatch.
//! - **[`PhysicsKineticGlassEffect`]**: a GPU runtime shader (`WGSL` for Apple
//!   Metal / Vello + `SkSL` for Skia + CPU fallback) whose dual-layer iridescent
//!   specular sweep, Fresnel glass rim glow, and directional chromatic dispersion
//!   are driven directly by live rigid-body/spring velocity, tilt, and energy.
//! - **[`PhysicsImpactRippleEffect`]**: a GPU runtime shader supporting up to **3
//!   simultaneous overlapping concentric shockwaves** driven by real physics
//!   collisions, radial blasts, and pointer impulses — rendering physical wave
//!   interference, surface refraction, chromatic dispersion, and luminous crests
//!   directly on any FerroUI `Visual` or `CompositionVisual`.
//! - **[`PhysicsEnergyFieldEffect`]**: a GPU runtime shader rendering dual-harmonic
//!   plasma fluid refraction and velocity-coupled iridescent aura.

use crate::body::{BodyBuilder, BodyHandle, BodyType};
use crate::material::PhysicsMaterial;
use crate::math::Vec2;
use crate::queries::CollisionEvent;
use crate::shapes::Shape2D;
use crate::solver::SolverConfig;
use crate::ui::{Spring1D, Spring2D, SpringConfig};
use crate::world::PhysicsWorld;
use ferroui_base::media::effects::{
    CustomEffect, EffectParameterValue, IImmutableEffect, ImmutableCustomEffect,
    ShaderEffectDescription,
};
use ferroui_base::media::Color;
use ferroui_base::rendering::composition::{
    CompositionSimulationContext, CompositionSimulationHandle, Compositor, ICompositionSimulation,
    SimulationEffectUpdate, SimulationMode, SimulationStepResult, SimulationTarget,
    SimulationVisualPatch,
};
use ferroui_base::{Point, Vector};
use rustc_hash::FxHashMap;
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
uniform float intensity;
uniform float distortion;
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
    float inten = clamp(intensity, 0.0, 2.5);
    float dist_s = clamp(distortion, 0.0, 2.0);
    if ((e < 0.002 && inten < 0.002) || (inten < 0.002 && dist_s < 0.002)) {
        return sample_clamped(coord);
    }

    float vlen = length(velocity);
    float2 dir = vlen > 0.001 ? (velocity / vlen) : float2(cos(tilt), sin(tilt));
    float shift = (0.35 + e * 5.0) * dist_s * layer_scale;

    half4 base = sample_clamped(coord);
    half4 out_c = base;
    if (shift > 0.05) {
        half4 r_s = sample_clamped(coord + dir * shift);
        half4 b_s = sample_clamped(coord - dir * shift);
        float mix_amt = clamp(e * 0.82 * min(dist_s, 1.25), 0.0, 0.92);
        out_c = half4(
            mix(base.r, r_s.r, half(mix_amt)),
            base.g,
            mix(base.b, b_s.b, half(mix_amt)),
            base.a
        );
    }

    // Velocity- & tilt-coupled dual-layer iridescent specular sweep.
    float angle = atan(dir.y, dir.x) * 0.45 + tilt * 0.88 + 0.52;
    float2 sweep_axis = float2(cos(angle), sin(angle));
    float diag = dot(uv - 0.5, sweep_axis);
    float sweep_pos = clamp(dot(velocity, float2(0.00065, 0.00055)) + sin(phase) * 0.36, -0.64, 0.64);
    float d1 = diag - sweep_pos;
    float band1 = exp(-(d1 * d1) * 20.0);
    float d2 = diag - sweep_pos + 0.14;
    float band2 = exp(-(d2 * d2) * 64.0) * 0.58;

    // Crisp Fresnel glass perimeter rim + inner catchlight.
    float2 edge_d = min(uv, 1.0 - uv);
    float min_edge = min(edge_d.x, edge_d.y);
    float rim = 1.0 - smoothstep(0.0, 0.14, min_edge);
    float crisp_bevel = 1.0 - smoothstep(0.0, 0.032, min_edge);

    float3 accent_rgb = accent.rgb / max(accent.a, 0.001);
    float3 prism_rgb = float3(accent_rgb.b, accent_rgb.r, accent_rgb.g);
    float3 sheen_rgb = mix(float3(1.0, 1.0, 1.0), accent_rgb, 0.52);
    float3 highlight_rgb = mix(sheen_rgb, prism_rgb, clamp(band2 * 0.85, 0.0, 0.65));

    float energy_gain = (0.20 + e * 0.85) * inten;
    float glow_strength = energy_gain * (band1 * 0.52 + band2 * 0.48 + rim * 0.38 + crisp_bevel * 0.32);
    float alpha = float(out_c.a);
    float3 lit = clamp(float3(out_c.rgb) + highlight_rgb * (glow_strength * alpha), 0.0, alpha);
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
    intensity: f32,
    distortion: f32,
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
    let inten = clamp(uniforms.intensity, 0.0, 2.5);
    let dist_s = clamp(uniforms.distortion, 0.0, 2.0);
    if ((e < 0.002 && inten < 0.002) || (inten < 0.002 && dist_s < 0.002)) {
        return sample_clamped(coord);
    }

    let vlen = length(uniforms.velocity);
    let fallback_dir = vec2<f32>(cos(uniforms.tilt), sin(uniforms.tilt));
    let dir = select(fallback_dir, uniforms.velocity / max(vlen, 0.001), vlen > 0.001);
    let shift = (0.35 + e * 5.0) * dist_s * uniforms.layer_scale;

    let base = sample_clamped(coord);
    var out_c = base;
    if (shift > 0.05) {
        let r_s = sample_clamped(coord + dir * shift);
        let b_s = sample_clamped(coord - dir * shift);
        let mix_amt = clamp(e * 0.82 * min(dist_s, 1.25), 0.0, 0.92);
        out_c = vec4<f32>(
            mix(base.r, r_s.r, mix_amt),
            base.g,
            mix(base.b, b_s.b, mix_amt),
            base.a,
        );
    }

    // Velocity- & tilt-coupled dual-layer iridescent specular sweep.
    let angle = atan2(dir.y, dir.x) * 0.45 + uniforms.tilt * 0.88 + 0.52;
    let sweep_axis = vec2<f32>(cos(angle), sin(angle));
    let diag = dot(uv - vec2<f32>(0.5), sweep_axis);
    let sweep_pos = clamp(
        dot(uniforms.velocity, vec2<f32>(0.00065, 0.00055)) + sin(uniforms.phase) * 0.36,
        -0.64,
        0.64,
    );
    let d1 = diag - sweep_pos;
    let band1 = exp(-(d1 * d1) * 20.0);
    let d2 = diag - sweep_pos + 0.14;
    let band2 = exp(-(d2 * d2) * 64.0) * 0.58;

    // Crisp Fresnel glass perimeter rim + inner catchlight.
    let edge_d = min(uv, vec2<f32>(1.0) - uv);
    let min_edge = min(edge_d.x, edge_d.y);
    let rim = 1.0 - smoothstep(0.0, 0.14, min_edge);
    let crisp_bevel = 1.0 - smoothstep(0.0, 0.032, min_edge);

    let accent_rgb = uniforms.accent.rgb / max(uniforms.accent.a, 0.001);
    let prism_rgb = vec3<f32>(accent_rgb.b, accent_rgb.r, accent_rgb.g);
    let sheen_rgb = mix(vec3<f32>(1.0, 1.0, 1.0), accent_rgb, 0.52);
    let highlight_rgb = mix(sheen_rgb, prism_rgb, clamp(band2 * 0.85, 0.0, 0.65));

    let energy_gain = (0.20 + e * 0.85) * inten;
    let glow_strength = energy_gain * (band1 * 0.52 + band2 * 0.48 + rim * 0.38 + crisp_bevel * 0.32);
    let alpha = out_c.a;
    let lit = clamp(out_c.rgb + highlight_rgb * (glow_strength * alpha), vec3<f32>(0.0), vec3<f32>(alpha));
    return vec4<f32>(lit, alpha);
}
"#;

/// Physics-driven kinetic glass GPU shader effect (`WGSL` + `SkSL` + CPU reference).
///
/// Couples live physics velocity, kinetic energy, and tilt to a dual-layer iridescent
/// specular light sweep, Fresnel glass perimeter bevel, and directional chromatic
/// dispersion.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsKineticGlassEffect {
    /// Visual velocity in DIPs/s driving sheen direction & dispersion.
    pub velocity: Vector,
    /// Normalized kinetic energy (`0.0..=1.0`).
    pub energy: f64,
    /// Tilt / angular rotation in radians.
    pub tilt: f64,
    /// Continuous phase (radians) for subtle sheen shimmer.
    pub phase: f64,
    /// Overall specular/rim luminosity multiplier (`0.0..=2.5`, default `1.0`).
    pub intensity: f64,
    /// Directional chromatic dispersion multiplier (`0.0..=2.0`, default `1.0`).
    pub distortion: f64,
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
            intensity: 1.0,
            distortion: 1.0,
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
        f("intensity", self.intensity.into());
        f("distortion", self.distortion.into());
        f("accent", self.accent.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "velocity" => value.as_vector().map(|v| self.velocity = v).is_some(),
            "energy" => value.as_float().map(|v| self.energy = v).is_some(),
            "tilt" => value.as_float().map(|v| self.tilt = v).is_some(),
            "phase" => value.as_float().map(|v| self.phase = v).is_some(),
            "intensity" => value.as_float().map(|v| self.intensity = v).is_some(),
            "distortion" => value.as_float().map(|v| self.distortion = v).is_some(),
            "accent" => value.as_color().map(|v| self.accent = v).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self {
            intensity: 0.0,
            distortion: 0.0,
            ..Self::default()
        })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(KINETIC_GLASS_SKSL, KINETIC_GLASS_WGSL)
                .with_sample_radius(14.0),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let e = self.energy.clamp(0.0, 1.0) as f32;
        let inten = self.intensity.clamp(0.0, 2.5) as f32;
        if width == 0 || height == 0 || (e < 0.002 && inten < 0.002) || inten < 0.002 {
            return true;
        }
        let ar = f32::from(self.accent.r) / 255.0;
        let ag = f32::from(self.accent.g) / 255.0;
        let ab = f32::from(self.accent.b) / 255.0;
        let sheen = [0.48 + ar * 0.52, 0.48 + ag * 0.52, 0.48 + ab * 0.52];
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
                let g = (0.20 + e * 0.85) * inten * edge * 0.34 * a;
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
// Physics-Driven GPU Shader #2: PhysicsImpactRippleEffect (3-Ring Interference)
// ---------------------------------------------------------------------------

const IMPACT_RIPPLE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float2 origin_1;
uniform float2 origin_2;
uniform float2 origin_3;
uniform float progress_1;
uniform float progress_2;
uniform float progress_3;
uniform float amp_1;
uniform float amp_2;
uniform float amp_3;
uniform float ring_width;
uniform float intensity;
uniform float distortion;
uniform float4 accent;
uniform shader content;

half4 sample_clamped(float2 p) {
    float2 hi = max(layer_size - 1.0, float2(0.0));
    float2 q = clamp(floor(p), float2(0.0), hi) + 0.5;
    return content.eval(q);
}

float4 eval_ring(float2 uv, float aspect, float2 orig, float prog_in, float amp_in, float width) {
    float prog = clamp(prog_in, 0.0, 1.0);
    float amp = clamp(amp_in, 0.0, 1.0);
    if (amp < 0.002 || prog <= 0.001 || prog >= 0.999) {
        return float4(0.0);
    }
    float2 delta = float2((uv.x - orig.x) * aspect, uv.y - orig.y);
    float dist = length(delta);
    float2 dir = dist > 0.0001 ? (delta / dist) : float2(1.0, 0.0);
    float radius = prog * max(aspect, 1.0) * 1.18;
    float diff = dist - radius;
    float fade = (1.0 - prog) * (1.0 - prog * 0.35);
    float w2 = width * width;
    float envelope = exp(-((diff * diff) / w2)) * fade * amp;
    float crest = exp(-((diff * diff) / (w2 * 0.22))) * fade * amp;
    float wave = sin(diff * 46.0) * envelope;
    return float4(dir * wave, envelope, crest);
}

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float2 uv = coord / size;
    float aspect = size.x / size.y;
    float width = max(ring_width, 0.03);
    float inten = clamp(intensity, 0.0, 2.5);
    float dist_s = clamp(distortion, 0.0, 2.0);

    float4 total =
        eval_ring(uv, aspect, origin_1, progress_1, amp_1, width) +
        eval_ring(uv, aspect, origin_2, progress_2, amp_2, width) +
        eval_ring(uv, aspect, origin_3, progress_3, amp_3, width);

    if (total.z < 0.0015) {
        return sample_clamped(coord);
    }

    float2 disp = total.xy * (16.5 * dist_s * layer_scale);
    float slope_len = length(total.xy);
    float2 cdir = slope_len > 0.0001 ? (total.xy / slope_len) : float2(1.0, 0.0);
    float2 chroma = cdir * (min(total.z, 1.25) * 5.2 * dist_s * layer_scale);

    half4 base = sample_clamped(coord);
    half4 warped = base;
    if (dist_s > 0.002) {
        half4 r_c = sample_clamped(coord + disp + chroma);
        half4 g_c = sample_clamped(coord + disp);
        half4 b_c = sample_clamped(coord + disp - chroma);
        warped = half4(r_c.r, g_c.g, b_c.b, base.a);
    }

    float3 accent_rgb = accent.rgb / max(accent.a, 0.001);
    float3 body_rgb = mix(float3(1.0, 1.0, 1.0), accent_rgb, 0.62);
    float3 crest_rgb = mix(body_rgb, float3(1.0, 1.0, 1.0), 0.55);
    float alpha = float(base.a);
    float glow_body = min(total.z, 1.35) * 0.62 * inten;
    float glow_crest = min(total.w, 1.25) * 0.68 * inten;
    float3 lit = clamp(
        float3(warped.rgb) + (body_rgb * glow_body + crest_rgb * glow_crest) * alpha,
        0.0,
        alpha
    );
    return half4(half3(lit), base.a);
}
"#;

const IMPACT_RIPPLE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    origin_1: vec2<f32>,
    origin_2: vec2<f32>,
    origin_3: vec2<f32>,
    progress_1: f32,
    progress_2: f32,
    progress_3: f32,
    amp_1: f32,
    amp_2: f32,
    amp_3: f32,
    ring_width: f32,
    intensity: f32,
    distortion: f32,
    accent: vec4<f32>,
}

fn sample_clamped(p: vec2<f32>) -> vec4<f32> {
    let hi = max(uniforms.layer_size - vec2<f32>(1.0), vec2<f32>(0.0));
    let q = clamp(floor(p), vec2<f32>(0.0), hi) + vec2<f32>(0.5);
    return sample_content(q);
}

fn eval_ring(uv: vec2<f32>, aspect: f32, orig: vec2<f32>, prog_in: f32, amp_in: f32, width: f32) -> vec4<f32> {
    let prog = clamp(prog_in, 0.0, 1.0);
    let amp = clamp(amp_in, 0.0, 1.0);
    if (amp < 0.002 || prog <= 0.001 || prog >= 0.999) {
        return vec4<f32>(0.0);
    }
    let delta = vec2<f32>((uv.x - orig.x) * aspect, uv.y - orig.y);
    let dist = length(delta);
    let dir = select(vec2<f32>(1.0, 0.0), delta / max(dist, 0.0001), dist > 0.0001);
    let radius = prog * max(aspect, 1.0) * 1.18;
    let diff = dist - radius;
    let fade = (1.0 - prog) * (1.0 - prog * 0.35);
    let w2 = width * width;
    let envelope = exp(-((diff * diff) / w2)) * fade * amp;
    let crest = exp(-((diff * diff) / (w2 * 0.22))) * fade * amp;
    let wave = sin(diff * 46.0) * envelope;
    return vec4<f32>(dir.x * wave, dir.y * wave, envelope, crest);
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let uv = coord / size;
    let aspect = size.x / size.y;
    let width = max(uniforms.ring_width, 0.03);
    let inten = clamp(uniforms.intensity, 0.0, 2.5);
    let dist_s = clamp(uniforms.distortion, 0.0, 2.0);

    let total =
        eval_ring(uv, aspect, uniforms.origin_1, uniforms.progress_1, uniforms.amp_1, width) +
        eval_ring(uv, aspect, uniforms.origin_2, uniforms.progress_2, uniforms.amp_2, width) +
        eval_ring(uv, aspect, uniforms.origin_3, uniforms.progress_3, uniforms.amp_3, width);

    if (total.z < 0.0015) {
        return sample_clamped(coord);
    }

    let disp = total.xy * (16.5 * dist_s * uniforms.layer_scale);
    let slope_len = length(total.xy);
    let cdir = select(vec2<f32>(1.0, 0.0), total.xy / max(slope_len, 0.0001), slope_len > 0.0001);
    let chroma = cdir * (min(total.z, 1.25) * 5.2 * dist_s * uniforms.layer_scale);

    let base = sample_clamped(coord);
    var warped = base;
    if (dist_s > 0.002) {
        let r_c = sample_clamped(coord + disp + chroma);
        let g_c = sample_clamped(coord + disp);
        let b_c = sample_clamped(coord + disp - chroma);
        warped = vec4<f32>(r_c.r, g_c.g, b_c.b, base.a);
    }

    let accent_rgb = uniforms.accent.rgb / max(uniforms.accent.a, 0.001);
    let body_rgb = mix(vec3<f32>(1.0, 1.0, 1.0), accent_rgb, 0.62);
    let crest_rgb = mix(body_rgb, vec3<f32>(1.0, 1.0, 1.0), 0.55);
    let alpha = base.a;
    let glow_body = min(total.z, 1.35) * 0.62 * inten;
    let glow_crest = min(total.w, 1.25) * 0.68 * inten;
    let lit = clamp(
        warped.rgb + (body_rgb * glow_body + crest_rgb * glow_crest) * alpha,
        vec3<f32>(0.0),
        vec3<f32>(alpha),
    );
    return vec4<f32>(lit, alpha);
}
"#;

/// Physics-driven multi-ring radial collision & shockwave GPU shader effect (`WGSL` + `SkSL` + CPU).
///
/// Supports up to **3 simultaneous overlapping concentric shockwaves** (`origin_1..3`,
/// `progress_1..3`, `amp_1..3`) with superposed physical wave interference, refractive
/// lensing, chromatic prism dispersion, and white-hot leading wavefront crests — all in
/// a single 3-tap GPU pass!
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsImpactRippleEffect {
    /// Normalized impact origin for ring slot 1 (`0..1`, `0..1`).
    pub origin_1: Point,
    /// Normalized impact origin for ring slot 2 (`0..1`, `0..1`).
    pub origin_2: Point,
    /// Normalized impact origin for ring slot 3 (`0..1`, `0..1`).
    pub origin_3: Point,
    /// Wavefront expansion progress for ring slot 1 (`0.0..=1.0`).
    pub progress_1: f64,
    /// Wavefront expansion progress for ring slot 2 (`0.0..=1.0`).
    pub progress_2: f64,
    /// Wavefront expansion progress for ring slot 3 (`0.0..=1.0`).
    pub progress_3: f64,
    /// Shockwave strength for ring slot 1 (`0.0..=1.0`).
    pub amp_1: f64,
    /// Shockwave strength for ring slot 2 (`0.0..=1.0`).
    pub amp_2: f64,
    /// Shockwave strength for ring slot 3 (`0.0..=1.0`).
    pub amp_3: f64,
    /// Gaussian ring width in normalized coordinates (default `0.135`).
    pub ring_width: f64,
    /// Luminous wavefront glow multiplier (`0.0..=2.5`, default `1.0`).
    pub intensity: f64,
    /// Surface refraction & prism dispersion multiplier (`0.0..=2.0`, default `1.0`).
    pub distortion: f64,
    /// Luminous wavefront tint color.
    pub accent: Color,
}

impl Default for PhysicsImpactRippleEffect {
    fn default() -> Self {
        Self {
            origin_1: Point::new(0.5, 0.5),
            origin_2: Point::new(0.5, 0.5),
            origin_3: Point::new(0.5, 0.5),
            progress_1: 0.0,
            progress_2: 0.0,
            progress_3: 0.0,
            amp_1: 0.0,
            amp_2: 0.0,
            amp_3: 0.0,
            ring_width: 0.135,
            intensity: 1.0,
            distortion: 1.0,
            accent: Color::from_argb(255, 34, 211, 238),
        }
    }
}

impl PhysicsImpactRippleEffect {
    /// Convenience constructor for a single shockwave ring.
    pub fn single(origin: Point, progress: f64, amplitude: f64, accent: Color) -> Self {
        Self {
            origin_1: origin,
            progress_1: progress,
            amp_1: amplitude,
            accent,
            ..Self::default()
        }
    }
}

impl CustomEffect for PhysicsImpactRippleEffect {
    const NAME: &'static str = "physics-impact-ripple";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("origin-1", self.origin_1.into());
        f("origin-2", self.origin_2.into());
        f("origin-3", self.origin_3.into());
        f("progress-1", self.progress_1.into());
        f("progress-2", self.progress_2.into());
        f("progress-3", self.progress_3.into());
        f("amp-1", self.amp_1.into());
        f("amp-2", self.amp_2.into());
        f("amp-3", self.amp_3.into());
        f("ring-width", self.ring_width.into());
        f("intensity", self.intensity.into());
        f("distortion", self.distortion.into());
        f("accent", self.accent.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "origin-1" => value.as_point().map(|v| self.origin_1 = v).is_some(),
            "origin-2" => value.as_point().map(|v| self.origin_2 = v).is_some(),
            "origin-3" => value.as_point().map(|v| self.origin_3 = v).is_some(),
            "progress-1" => value.as_float().map(|v| self.progress_1 = v).is_some(),
            "progress-2" => value.as_float().map(|v| self.progress_2 = v).is_some(),
            "progress-3" => value.as_float().map(|v| self.progress_3 = v).is_some(),
            "amp-1" => value.as_float().map(|v| self.amp_1 = v).is_some(),
            "amp-2" => value.as_float().map(|v| self.amp_2 = v).is_some(),
            "amp-3" => value.as_float().map(|v| self.amp_3 = v).is_some(),
            "ring-width" => value.as_float().map(|v| self.ring_width = v).is_some(),
            "intensity" => value.as_float().map(|v| self.intensity = v).is_some(),
            "distortion" => value.as_float().map(|v| self.distortion = v).is_some(),
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
                .with_sample_radius(22.0),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let rings = [
            (
                self.origin_1,
                self.progress_1.clamp(0.0, 1.0) as f32,
                self.amp_1.clamp(0.0, 1.0) as f32,
            ),
            (
                self.origin_2,
                self.progress_2.clamp(0.0, 1.0) as f32,
                self.amp_2.clamp(0.0, 1.0) as f32,
            ),
            (
                self.origin_3,
                self.progress_3.clamp(0.0, 1.0) as f32,
                self.amp_3.clamp(0.0, 1.0) as f32,
            ),
        ];
        let any_active = rings
            .iter()
            .any(|&(_, p, a)| a >= 0.002 && p > 0.001 && p < 0.999);
        if width == 0 || height == 0 || !any_active {
            return true;
        }
        let aspect = width as f32 / (height as f32).max(1.0);
        let ring_w = (self.ring_width as f32).max(0.03);
        let inv_w2 = 1.0 / (ring_w * ring_w);
        let inten = self.intensity.clamp(0.0, 2.5) as f32;
        let ar = f32::from(self.accent.r) / 255.0;
        let ag = f32::from(self.accent.g) / 255.0;
        let ab = f32::from(self.accent.b) / 255.0;
        let ring_rgb = [0.38 + ar * 0.62, 0.38 + ag * 0.62, 0.38 + ab * 0.62];

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
                let mut total_env = 0.0_f32;
                for &(orig, prog, amp) in &rings {
                    if amp < 0.002 || prog <= 0.001 || prog >= 0.999 {
                        continue;
                    }
                    let radius = prog * aspect.max(1.0) * 1.18;
                    let dx = (u - orig.x as f32) * aspect;
                    let dy = v - orig.y as f32;
                    let diff = (dx * dx + dy * dy).sqrt() - radius;
                    total_env += (-(diff * diff) * inv_w2).exp() * (1.0 - prog) * amp;
                }
                if total_env > 0.001 {
                    let g = total_env.min(1.4) * 0.68 * inten * a;
                    for ch in 0..3 {
                        let c = f32::from(pixels[idx + ch]) / 255.0;
                        pixels[idx + ch] =
                            ((c + ring_rgb[ch] * g).clamp(0.0, a) * 255.0).round() as u8;
                    }
                }
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Physics-Driven GPU Shader #3: PhysicsEnergyFieldEffect
// ---------------------------------------------------------------------------

const ENERGY_FIELD_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float2 velocity;
uniform float energy;
uniform float phase;
uniform float intensity;
uniform float distortion;
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
    float inten = clamp(intensity, 0.0, 2.5);
    float dist_s = clamp(distortion, 0.0, 2.0);

    float w1 = sin(uv.x * 10.5 + phase * 1.85 + velocity.x * 0.0038);
    float w2 = cos(uv.y * 8.8 - phase * 1.45 + velocity.y * 0.0038);
    float w3 = sin((uv.x + uv.y) * 7.2 + phase * 1.15);
    float field = clamp((w1 * w2 + w3 * 0.45) * 0.36 + 0.5, 0.0, 1.0);

    float2 warp = float2(w2 + w3 * 0.35, w1 - w3 * 0.35) * ((0.7 + e * 4.4) * dist_s * layer_scale);
    half4 base = sample_clamped(coord);
    half4 sampled = dist_s > 0.002 ? sample_clamped(coord + warp) : base;
    float alpha = float(base.a);

    float3 col_a = accent.rgb / max(accent.a, 0.001);
    float3 col_b = float3(col_a.b, col_a.r, col_a.g);
    float3 aurora = mix(col_a, col_b, field);

    float2 edge_d = min(uv, 1.0 - uv);
    float rim = 1.0 - smoothstep(0.0, 0.22, min(edge_d.x, edge_d.y));
    float strength = (0.15 + e * 0.46) * inten * (0.48 * field + 0.78 * rim);
    float3 lit = clamp(float3(sampled.rgb) + aurora * (strength * alpha), 0.0, alpha);
    return half4(half3(lit), base.a);
}
"#;

const ENERGY_FIELD_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    velocity: vec2<f32>,
    energy: f32,
    phase: f32,
    intensity: f32,
    distortion: f32,
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
    let inten = clamp(uniforms.intensity, 0.0, 2.5);
    let dist_s = clamp(uniforms.distortion, 0.0, 2.0);

    let w1 = sin(uv.x * 10.5 + uniforms.phase * 1.85 + uniforms.velocity.x * 0.0038);
    let w2 = cos(uv.y * 8.8 - uniforms.phase * 1.45 + uniforms.velocity.y * 0.0038);
    let w3 = sin((uv.x + uv.y) * 7.2 + uniforms.phase * 1.15);
    let field = clamp((w1 * w2 + w3 * 0.45) * 0.36 + 0.5, 0.0, 1.0);

    let warp = vec2<f32>(w2 + w3 * 0.35, w1 - w3 * 0.35) * ((0.7 + e * 4.4) * dist_s * uniforms.layer_scale);
    let base = sample_clamped(coord);
    let sampled = select(base, sample_clamped(coord + warp), dist_s > 0.002);
    let alpha = base.a;

    let col_a = uniforms.accent.rgb / max(uniforms.accent.a, 0.001);
    let col_b = vec3<f32>(col_a.b, col_a.r, col_a.g);
    let aurora = mix(col_a, col_b, field);

    let edge_d = min(uv, vec2<f32>(1.0) - uv);
    let rim = 1.0 - smoothstep(0.0, 0.22, min(edge_d.x, edge_d.y));
    let strength = (0.15 + e * 0.46) * inten * (0.48 * field + 0.78 * rim);
    let lit = clamp(sampled.rgb + aurora * (strength * alpha), vec3<f32>(0.0), vec3<f32>(alpha));
    return vec4<f32>(lit, alpha);
}
"#;

/// Physics-driven harmonic plasma / aurora GPU shader effect (`WGSL` + `SkSL` + CPU).
///
/// Renders dual-harmonic fluid wave refraction and a velocity-coupled iridescent energy
/// aura driven continuously by physics velocity, kinetic energy, and phase.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsEnergyFieldEffect {
    pub velocity: Vector,
    pub energy: f64,
    pub phase: f64,
    pub intensity: f64,
    pub distortion: f64,
    pub accent: Color,
}

impl Default for PhysicsEnergyFieldEffect {
    fn default() -> Self {
        Self {
            velocity: Vector::new(0.0, 0.0),
            energy: 0.35,
            phase: 0.0,
            intensity: 1.0,
            distortion: 1.0,
            accent: Color::from_argb(255, 168, 85, 247),
        }
    }
}

impl CustomEffect for PhysicsEnergyFieldEffect {
    const NAME: &'static str = "physics-energy-field";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("velocity", self.velocity.into());
        f("energy", self.energy.into());
        f("phase", self.phase.into());
        f("intensity", self.intensity.into());
        f("distortion", self.distortion.into());
        f("accent", self.accent.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "velocity" => value.as_vector().map(|v| self.velocity = v).is_some(),
            "energy" => value.as_float().map(|v| self.energy = v).is_some(),
            "phase" => value.as_float().map(|v| self.phase = v).is_some(),
            "intensity" => value.as_float().map(|v| self.intensity = v).is_some(),
            "distortion" => value.as_float().map(|v| self.distortion = v).is_some(),
            "accent" => value.as_color().map(|v| self.accent = v).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self {
            energy: 0.0,
            intensity: 0.0,
            distortion: 0.0,
            ..Self::default()
        })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(ENERGY_FIELD_SKSL, ENERGY_FIELD_WGSL)
                .with_sample_radius(14.0),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let e = self.energy.clamp(0.0, 1.0) as f32;
        let inten = self.intensity.clamp(0.0, 2.5) as f32;
        if width == 0 || height == 0 || inten < 0.002 {
            return true;
        }
        let ar = f32::from(self.accent.r) / 255.0;
        let ag = f32::from(self.accent.g) / 255.0;
        let ab = f32::from(self.accent.b) / 255.0;
        let strength = (0.15 + e * 0.46) * inten * 0.55;
        for px in pixels.chunks_exact_mut(4) {
            let a = f32::from(px[3]) / 255.0;
            if a <= 0.0 {
                continue;
            }
            for (ch, acc) in [ar, ag, ab].into_iter().enumerate() {
                let c = f32::from(px[ch]) / 255.0;
                px[ch] = ((c + acc * strength * a).clamp(0.0, a) * 255.0).round() as u8;
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
    /// Automatically switches between multi-ring impact ripples on collisions/clicks
    /// and kinetic glass sheen during motion (holding a resting Fresnel catchlight
    /// at zero CPU cost when settled).
    ReactiveAuto,
    /// Velocity/tilt-driven kinetic glass shader only.
    KineticGlassOnly,
    /// Multi-ring impact/collision radial ripple shader only.
    ImpactRippleOnly,
    /// Continuous velocity/phase-driven aurora energy field shader.
    EnergyFieldOnly,
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

const MAX_SIMULTANEOUS_RIPPLES: usize = 3;

#[derive(Clone, Copy)]
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
    /// Up to 3 simultaneous overlapping GPU shockwave rings!
    ripples: [Option<ActiveRipple>; MAX_SIMULTANEOUS_RIPPLES],
    /// Continuous phase for kinetic sheen / energy field.
    phase: f32,
    /// Last computed speed in DIPs/s.
    last_speed: f32,
    /// Tracks whether `ImpactRippleOnly` already pushed `Clear` so resting visuals emit `Keep`.
    effect_cleared: bool,
    /// Tracks whether a resting `ReactiveAuto` catchlight has already been pushed.
    resting_catchlight_synced: bool,
}

impl BoundVisualEntry {
    /// Pushes a new concentric ripple into the first free slot, or replaces the
    /// oldest (furthest-progressed) active wave when all 3 slots are busy.
    fn push_ripple(&mut self, ripple: ActiveRipple) {
        self.effect_cleared = false;
        self.resting_catchlight_synced = false;
        for slot in &mut self.ripples {
            if slot.is_none() {
                *slot = Some(ripple);
                return;
            }
        }
        // All 3 slots occupied: replace the wave closest to completion.
        let mut oldest_idx = 0;
        let mut max_prog = -1.0_f32;
        for (i, slot) in self.ripples.iter().enumerate() {
            if let Some(r) = slot {
                if r.progress > max_prog {
                    max_prog = r.progress;
                    oldest_idx = i;
                }
            }
        }
        self.ripples[oldest_idx] = Some(ripple);
    }

    fn active_ripple_count(&self) -> usize {
        self.ripples.iter().filter(|r| r.is_some()).count()
    }

    fn has_active_ripples(&self) -> bool {
        self.ripples.iter().any(|r| r.is_some())
    }

    fn advance_ripples(&mut self, dt: f32) -> bool {
        let mut any = false;
        for slot in &mut self.ripples {
            if let Some(r) = slot {
                r.progress += dt * r.speed;
                if r.progress >= 1.0 {
                    *slot = None;
                } else {
                    any = true;
                }
            }
        }
        any
    }

    fn build_ripple_effect(&self, intensity: f32, distortion: f32) -> PhysicsImpactRippleEffect {
        let mut eff = PhysicsImpactRippleEffect {
            ring_width: 0.135,
            intensity: intensity as f64,
            distortion: distortion as f64,
            accent: self.accent,
            ..PhysicsImpactRippleEffect::default()
        };
        if let Some(r) = self.ripples[0] {
            eff.origin_1 = r.origin;
            eff.progress_1 = r.progress as f64;
            eff.amp_1 = r.amplitude as f64;
        }
        if let Some(r) = self.ripples[1] {
            eff.origin_2 = r.origin;
            eff.progress_2 = r.progress as f64;
            eff.amp_2 = r.amplitude as f64;
        }
        if let Some(r) = self.ripples[2] {
            eff.origin_3 = r.origin;
            eff.progress_3 = r.progress as f64;
            eff.amp_3 = r.amplitude as f64;
        }
        eff
    }
}

struct SharedPhysicsState {
    pixels_per_meter: f32,
    arena_width: f32,
    arena_height: f32,
    behavior: PhysicsCompositorBehavior,
    shader_mode: PhysicsShaderMode,
    shader_intensity: f32,
    shader_distortion: f32,
    spring_config: SpringConfig,
    gravity: Vec2,
    world: PhysicsWorld,
    walls: Vec<BodyHandle>,
    entries: Vec<BoundVisualEntry>,
    /// `O(1)` lookup from rigid-body handle to `entries` index during collision dispatch.
    body_to_entry: FxHashMap<BodyHandle, usize>,
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
            shader_intensity: 1.0,
            shader_distortion: 1.0,
            spring_config: SpringConfig::bouncy(),
            gravity,
            world,
            walls,
            entries: Vec::new(),
            body_to_entry: FxHashMap::default(),
            total_collisions: 0,
            step_count: 0,
            compositor: None,
            dirty_wake: true,
        }
    }

    fn create_arena_walls(
        world: &mut PhysicsWorld,
        w_px: f32,
        h_px: f32,
        ppm: f32,
    ) -> Vec<BodyHandle> {
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
                    BodyBuilder::static_body(Shape2D::box_half_extents(hx, hy))
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
        self.walls = Self::create_arena_walls(
            &mut self.world,
            self.arena_width,
            self.arena_height,
            self.pixels_per_meter,
        );
        self.wake();
    }

    fn invalidate_effect_caches(&mut self) {
        for e in &mut self.entries {
            e.effect_cleared = false;
            e.resting_catchlight_synced = false;
        }
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
        (
            CompositorPhysicsSimulation {
                name: name.into(),
                inner: inner.clone(),
            },
            Self { inner },
        )
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
    pub fn bind_visual(
        &self,
        target: SimulationTarget,
        rest_center: Vec2,
        size: Vec2,
        accent: Color,
    ) -> usize {
        let mut st = self.inner.borrow_mut();
        let pos_m = st.px_to_world(rest_center);
        let hx = (size.x * 0.5 / st.pixels_per_meter).max(0.15);
        let hy = (size.y * 0.5 / st.pixels_per_meter).max(0.15);
        let body = st.world.add_body(
            BodyBuilder::dynamic(Shape2D::box_half_extents(hx, hy))
                .position(pos_m)
                .material(PhysicsMaterial::new(1.2, 0.3, 0.62))
                .linear_damping(0.45)
                .angular_damping(0.9),
        );
        let idx = st.entries.len();
        let spring_cfg = st.spring_config;
        st.body_to_entry.insert(body, idx);
        st.entries.push(BoundVisualEntry {
            target,
            rest_center,
            size,
            accent,
            body,
            spring_pos: Spring2D::new(Vec2::ZERO, spring_cfg),
            spring_tilt: Spring1D::new(0.0, SpringConfig::snappy()),
            spring_scale: Spring1D::new(1.0, spring_cfg),
            dragging: false,
            drag_velocity: Vec2::ZERO,
            ripples: [None; MAX_SIMULTANEOUS_RIPPLES],
            phase: idx as f32 * 1.15,
            last_speed: 0.0,
            effect_cleared: false,
            resting_catchlight_synced: false,
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
    /// current visual positions and velocities.
    pub fn set_behavior(&self, behavior: PhysicsCompositorBehavior) {
        let mut st = self.inner.borrow_mut();
        if st.behavior == behavior {
            return;
        }
        st.behavior = behavior;
        let ppm = st.pixels_per_meter;
        let SharedPhysicsState { world, entries, .. } = &mut *st;
        for entry in entries.iter_mut() {
            match behavior {
                PhysicsCompositorBehavior::ElasticSprings => {
                    if let Some(body) = world.body(entry.body) {
                        let rel = body.position * ppm - entry.rest_center;
                        entry.spring_pos.x.value = rel.x;
                        entry.spring_pos.y.value = rel.y;
                        entry.spring_pos.set_velocity(body.linear_velocity * ppm);
                        entry.spring_pos.set_target(Vec2::ZERO);
                        entry.spring_tilt.value = body.rotation.radians();
                        entry.spring_tilt.velocity = body.angular_velocity;
                        entry.spring_tilt.target = 0.0;
                    }
                }
                PhysicsCompositorBehavior::RigidBody => {
                    let world_pos = (entry.rest_center + entry.spring_pos.value()) / ppm;
                    let world_vel = entry.spring_pos.velocity() / ppm;
                    if let Some(body) = world.body_mut(entry.body) {
                        body.body_type = if entry.dragging {
                            BodyType::Kinematic
                        } else {
                            BodyType::Dynamic
                        };
                        body.recompute_mass_properties();
                        body.position = world_pos;
                        body.linear_velocity = world_vel;
                        body.set_angle(entry.spring_tilt.value);
                        body.angular_velocity = entry.spring_tilt.velocity;
                        body.wake_up();
                    }
                }
            }
        }
        st.invalidate_effect_caches();
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
        let SharedPhysicsState { world, entries, .. } = &mut *st;
        for entry in entries.iter() {
            if let Some(b) = world.body_mut(entry.body) {
                b.wake_up();
            }
        }
        st.wake();
    }

    /// Returns current gravity vector in m/s².
    pub fn gravity(&self) -> Vec2 {
        self.inner.borrow().gravity
    }

    /// Updates the analytical spring preset (`SpringConfig::snappy()`, `bouncy()`, `gentle()`, `wobbly()`).
    pub fn set_spring_config(&self, config: SpringConfig) {
        let mut st = self.inner.borrow_mut();
        st.spring_config = config;
        for entry in &mut st.entries {
            entry.spring_pos.x.config = config;
            entry.spring_pos.y.config = config;
            entry.spring_scale.config = config;
        }
        st.wake();
    }

    /// Returns the active analytical spring configuration.
    pub fn spring_config(&self) -> SpringConfig {
        self.inner.borrow().spring_config
    }

    /// Configures live shader luminosity/glow intensity (`0.0..=2.5`, default `1.0`).
    pub fn set_shader_intensity(&self, intensity: f32) {
        let mut st = self.inner.borrow_mut();
        st.shader_intensity = intensity.clamp(0.0, 2.5);
        st.invalidate_effect_caches();
        st.wake();
    }

    /// Returns the current shader intensity multiplier.
    pub fn shader_intensity(&self) -> f32 {
        self.inner.borrow().shader_intensity
    }

    /// Configures live shader refraction & chromatic dispersion strength (`0.0..=2.0`, default `1.0`).
    pub fn set_shader_distortion(&self, distortion: f32) {
        let mut st = self.inner.borrow_mut();
        st.shader_distortion = distortion.clamp(0.0, 2.0);
        st.invalidate_effect_caches();
        st.wake();
    }

    /// Returns the current shader distortion multiplier.
    pub fn shader_distortion(&self) -> f32 {
        self.inner.borrow().shader_distortion
    }

    /// Selects which GPU shader effect mode is driven by the physics state.
    pub fn set_shader_mode(&self, mode: PhysicsShaderMode) {
        let mut st = self.inner.borrow_mut();
        st.shader_mode = mode;
        st.invalidate_effect_caches();
        if mode == PhysicsShaderMode::Disabled {
            for entry in &mut st.entries {
                entry
                    .target
                    .apply(&SimulationVisualPatch::new().without_effect());
                entry.effect_cleared = true;
            }
        }
        st.wake();
    }

    /// Returns the active GPU shader mode.
    pub fn shader_mode(&self) -> PhysicsShaderMode {
        self.inner.borrow().shader_mode
    }

    /// Starts an interactive pointer drag on bound visual `index` and layers a
    /// tactile GPU impact ripple at `normalized_origin`.
    pub fn begin_drag(&self, index: usize, normalized_origin: Point) {
        let mut st = self.inner.borrow_mut();
        let handle = {
            let Some(entry) = st.entries.get_mut(index) else {
                return;
            };
            entry.dragging = true;
            entry.drag_velocity = Vec2::ZERO;
            entry.spring_scale.target = 1.05;
            entry.push_ripple(ActiveRipple {
                origin: Point::new(
                    normalized_origin.x.clamp(0.0, 1.0),
                    normalized_origin.y.clamp(0.0, 1.0),
                ),
                progress: 0.02,
                amplitude: 0.92,
                speed: 0.92,
            });
            entry.body
        };
        if let Some(body) = st.world.body_mut(handle) {
            body.body_type = BodyType::Kinematic;
            body.recompute_mass_properties();
            body.linear_velocity = Vec2::ZERO;
            body.angular_velocity = 0.0;
            body.wake_up();
        }
        st.wake();
    }

    /// Moves a dragged visual by `delta_px` (DIPs) and tracks pointer velocity.
    pub fn drag_by(&self, index: usize, delta_px: Vec2, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 || !delta_px.x.is_finite() || !delta_px.y.is_finite() {
            return;
        }
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let (handle, drag_vel) = {
            let Some(entry) = st.entries.get_mut(index) else {
                return;
            };
            if !entry.dragging {
                return;
            }
            let inv_dt = 1.0 / dt.max(0.008);
            let inst_vel = delta_px * inv_dt;
            entry.drag_velocity = entry.drag_velocity * 0.55 + inst_vel * 0.45;
            if behavior == PhysicsCompositorBehavior::ElasticSprings {
                let next = entry.spring_pos.value() + delta_px;
                entry.spring_pos.x.value = next.x;
                entry.spring_pos.y.value = next.y;
                entry.spring_pos.set_target(next);
                entry.spring_pos.set_velocity(entry.drag_velocity);
                entry.spring_tilt.target = (entry.drag_velocity.x * 0.00028).clamp(-0.26, 0.26);
            }
            (entry.body, entry.drag_velocity)
        };
        if behavior == PhysicsCompositorBehavior::RigidBody {
            if let Some(body) = st.world.body_mut(handle) {
                body.position += delta_px / ppm;
                body.linear_velocity = drag_vel / ppm;
                body.angular_velocity = (drag_vel.x * 0.0025).clamp(-4.0, 4.0);
                body.wake_up();
            }
        }
        st.wake();
    }

    /// Releases a dragged visual, imparting its fling momentum to the physics world
    /// or elastic spring.
    pub fn end_drag(&self, index: usize) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let (handle, fling) = {
            let Some(entry) = st.entries.get_mut(index) else {
                return;
            };
            if !entry.dragging {
                return;
            }
            entry.dragging = false;
            entry.spring_scale.target = 1.0;
            let fling = entry.drag_velocity;
            if behavior == PhysicsCompositorBehavior::ElasticSprings {
                entry.spring_pos.set_target(Vec2::ZERO);
                entry.spring_pos.set_velocity(fling);
                entry.spring_tilt.target = 0.0;
            }
            (entry.body, fling)
        };
        if let Some(body) = st.world.body_mut(handle) {
            body.body_type = BodyType::Dynamic;
            body.recompute_mass_properties();
            if behavior == PhysicsCompositorBehavior::RigidBody {
                let v = fling / ppm;
                let len = v.length();
                body.linear_velocity = if len > 22.0 { v * (22.0 / len) } else { v };
                body.wake_up();
            }
        }
        st.wake();
    }

    /// Layers a new GPU concentric shockwave ripple at `normalized_origin` on visual
    /// `index` (supporting up to 3 simultaneous overlapping waves per visual) without
    /// imparting translational impulse.
    pub fn trigger_ripple_at(&self, index: usize, normalized_origin: Point, intensity: f32) {
        let mut st = self.inner.borrow_mut();
        let Some(entry) = st.entries.get_mut(index) else {
            return;
        };
        entry.push_ripple(ActiveRipple {
            origin: Point::new(
                normalized_origin.x.clamp(0.0, 1.0),
                normalized_origin.y.clamp(0.0, 1.0),
            ),
            progress: 0.02,
            amplitude: intensity.clamp(0.25, 1.0),
            speed: 0.86,
        });
        entry.spring_scale.velocity -= intensity * 0.75;
        st.wake();
    }

    /// Triggers an immediate GPU impact ripple shader + physical impulse on visual `index`.
    pub fn trigger_impact(
        &self,
        index: usize,
        normalized_origin: Point,
        intensity: f32,
        impulse_px: Vec2,
    ) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let handle = {
            let Some(entry) = st.entries.get_mut(index) else {
                return;
            };
            entry.push_ripple(ActiveRipple {
                origin: Point::new(
                    normalized_origin.x.clamp(0.0, 1.0),
                    normalized_origin.y.clamp(0.0, 1.0),
                ),
                progress: 0.02,
                amplitude: intensity.clamp(0.25, 1.0),
                speed: 0.88,
            });
            entry.spring_scale.velocity -= intensity * 1.4;
            if behavior == PhysicsCompositorBehavior::ElasticSprings {
                let next_v = entry.spring_pos.velocity() + impulse_px;
                entry.spring_pos.set_velocity(next_v);
                entry.spring_tilt.velocity += (impulse_px.x * 0.004).clamp(-3.0, 3.0);
            }
            entry.body
        };
        if behavior == PhysicsCompositorBehavior::RigidBody {
            if let Some(body) = st.world.body_mut(handle) {
                let mass = body.mass;
                body.apply_impulse(impulse_px / ppm * mass);
                body.apply_angular_impulse((impulse_px.x * 0.0025).clamp(-2.5, 2.5));
                body.wake_up();
            }
        }
        st.wake();
    }

    /// Blasts all bound visuals radially outward from `center_px` and layers
    /// synchronized GPU shockwave ripples across every affected visual.
    pub fn shockwave(&self, center_px: Vec2, strength: f32) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let behavior = st.behavior;
        let SharedPhysicsState { world, entries, .. } = &mut *st;
        for (idx, e) in entries.iter_mut().enumerate() {
            let current_px = match behavior {
                PhysicsCompositorBehavior::RigidBody => world
                    .body(e.body)
                    .map(|b| b.position * ppm)
                    .unwrap_or(e.rest_center),
                PhysicsCompositorBehavior::ElasticSprings => e.rest_center + e.spring_pos.value(),
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

            e.push_ripple(ActiveRipple {
                origin: local_origin,
                progress: 0.02,
                amplitude: (0.78 + 0.22 * falloff).clamp(0.0, 1.0),
                speed: 0.85,
            });
            e.spring_scale.velocity -= 1.8 * falloff;
            match behavior {
                PhysicsCompositorBehavior::ElasticSprings => {
                    let nv = e.spring_pos.velocity() + vel_boost;
                    e.spring_pos.set_velocity(nv);
                    e.spring_tilt.velocity += (dir.x * 4.5 * falloff).clamp(-6.0, 6.0);
                }
                PhysicsCompositorBehavior::RigidBody => {
                    if let Some(body) = world.body_mut(e.body) {
                        body.linear_velocity += vel_boost / ppm;
                        body.angular_velocity += (dir.x * 5.0 * falloff).clamp(-8.0, 8.0);
                        body.wake_up();
                    }
                }
            }
        }
        st.wake();
    }

    /// Snaps all visuals cleanly back to their rest layout positions with a crisp spring settle.
    pub fn snap_home(&self) {
        let mut st = self.inner.borrow_mut();
        let ppm = st.pixels_per_meter;
        let SharedPhysicsState { world, entries, .. } = &mut *st;
        for e in entries.iter_mut() {
            if let Some(body) = world.body_mut(e.body) {
                body.position = e.rest_center / ppm;
                body.rotation = crate::math::Rot2::IDENTITY;
                body.angle = 0.0;
                body.linear_velocity = Vec2::ZERO;
                body.angular_velocity = 0.0;
                body.wake_up();
            }
            e.spring_pos.set_target(Vec2::ZERO);
            let cur = e.spring_pos.value();
            e.spring_pos.set_velocity(-cur * 6.0);
            e.spring_tilt.target = 0.0;
            e.spring_scale.target = 1.0;
            e.push_ripple(ActiveRipple {
                origin: Point::new(0.5, 0.5),
                progress: 0.02,
                amplitude: 0.55,
                speed: 1.55,
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
                .filter(|e| !e.spring_pos.is_settled() || e.dragging || e.has_active_ripples())
                .count(),
        };
        let active_ripples: usize = st
            .entries
            .iter()
            .map(BoundVisualEntry::active_ripple_count)
            .sum();
        let peak_speed_px = st
            .entries
            .iter()
            .map(|e| e.last_speed)
            .fold(0.0_f32, f32::max);
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
        let shader_intensity = st.shader_intensity;
        let shader_distortion = st.shader_distortion;
        let mut any_active = false;

        // 1. Step rigid-body physics world & harvest real collisions with ZERO per-step
        //    heap allocations and O(1) body_to_entry lookup!
        if behavior == PhysicsCompositorBehavior::RigidBody {
            let SharedPhysicsState {
                world,
                entries,
                body_to_entry,
                total_collisions,
                ..
            } = &mut *st;
            for entry in entries.iter() {
                if entry.dragging {
                    if let Some(b) = world.body_mut(entry.body) {
                        b.linear_velocity = Vec2::ZERO;
                        b.angular_velocity = 0.0;
                    }
                }
            }

            world.step(dt);

            for ev in world.collision_events() {
                let CollisionEvent::Started {
                    body_a,
                    body_b,
                    point,
                    impact_speed,
                    ..
                } = *ev
                else {
                    continue;
                };
                if impact_speed < 0.75 {
                    continue;
                }
                *total_collisions = total_collisions.wrapping_add(1);
                let contact_px = point * ppm;
                let intensity = (impact_speed / 8.0).clamp(0.35, 1.0);
                for body_handle in [body_a, body_b] {
                    if let (Some(&entry_idx), Some(body)) =
                        (body_to_entry.get(&body_handle), world.body(body_handle))
                    {
                        let entry = &mut entries[entry_idx];
                        let pos_px = body.position * ppm;
                        let w = entry.size.x.max(1.0);
                        let h = entry.size.y.max(1.0);
                        let norm_x =
                            ((contact_px.x - (pos_px.x - w * 0.5)) / w).clamp(0.05, 0.95) as f64;
                        let norm_y =
                            ((contact_px.y - (pos_px.y - h * 0.5)) / h).clamp(0.05, 0.95) as f64;
                        entry.push_ripple(ActiveRipple {
                            origin: Point::new(norm_x, norm_y),
                            progress: 0.02,
                            amplitude: intensity,
                            speed: 1.35,
                        });
                    }
                }
            }
        }

        // 2. Push composition transforms + physics-driven GPU shader effects for every visual!
        let SharedPhysicsState { world, entries, .. } = &mut *st;
        for entry in entries.iter_mut() {
            let (tx, ty, rot, sx, sy, vel_px, speed_px, awake_this) = match behavior {
                PhysicsCompositorBehavior::RigidBody => {
                    entry.spring_scale.step(dt);
                    let sc = entry.spring_scale.value.clamp(0.88, 1.14) as f64;
                    if let Some(body) = world.body(entry.body) {
                        let pos_px = body.position * ppm;
                        let vel = body.linear_velocity * ppm;
                        let spd = vel.length();
                        let awake = !body.is_sleeping
                            || entry.dragging
                            || spd > 2.5
                            || body.angular_velocity.abs() > 0.02
                            || !entry.spring_scale.is_settled();
                        (
                            (pos_px.x - entry.rest_center.x) as f64,
                            (pos_px.y - entry.rest_center.y) as f64,
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
                    if !entry.dragging {
                        entry.spring_pos.step(dt);
                        entry.spring_tilt.target =
                            (entry.spring_pos.velocity().x * 0.00024).clamp(-0.24, 0.24);
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

            entry.last_speed = speed_px;
            entry.phase = (entry.phase + dt * (1.5 + speed_px * 0.006)) % std::f32::consts::TAU;

            let ripple_active = entry.advance_ripples(dt);
            if awake_this || ripple_active {
                any_active = true;
            }

            // Compute physics-driven GPU shader effect update (skipping redundant batches when settled!).
            let energy = (speed_px / 520.0).clamp(0.0, 1.0);
            let effect_update = match shader_mode {
                PhysicsShaderMode::Disabled => SimulationEffectUpdate::Keep,
                PhysicsShaderMode::ImpactRippleOnly => {
                    if ripple_active {
                        entry.effect_cleared = false;
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            entry.build_ripple_effect(shader_intensity, shader_distortion),
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else if !entry.effect_cleared {
                        entry.effect_cleared = true;
                        SimulationEffectUpdate::Clear
                    } else {
                        SimulationEffectUpdate::Keep
                    }
                }
                PhysicsShaderMode::KineticGlassOnly => {
                    any_active = true;
                    entry.resting_catchlight_synced = false;
                    let eff: Arc<dyn IImmutableEffect> =
                        Arc::new(ImmutableCustomEffect::new(PhysicsKineticGlassEffect {
                            velocity: Vector::new(vel_px.x as f64, vel_px.y as f64),
                            energy: if entry.dragging {
                                energy.max(0.62)
                            } else {
                                energy.max(0.28)
                            } as f64,
                            tilt: rot as f64,
                            phase: entry.phase as f64,
                            intensity: shader_intensity as f64,
                            distortion: shader_distortion as f64,
                            accent: entry.accent,
                        }));
                    SimulationEffectUpdate::Set(eff)
                }
                PhysicsShaderMode::EnergyFieldOnly => {
                    any_active = true;
                    entry.resting_catchlight_synced = false;
                    let eff: Arc<dyn IImmutableEffect> =
                        Arc::new(ImmutableCustomEffect::new(PhysicsEnergyFieldEffect {
                            velocity: Vector::new(vel_px.x as f64, vel_px.y as f64),
                            energy: if entry.dragging {
                                energy.max(0.75)
                            } else {
                                energy.max(0.22)
                            } as f64,
                            phase: entry.phase as f64,
                            intensity: shader_intensity as f64,
                            distortion: shader_distortion as f64,
                            accent: entry.accent,
                        }));
                    SimulationEffectUpdate::Set(eff)
                }
                PhysicsShaderMode::ReactiveAuto => {
                    if ripple_active {
                        entry.resting_catchlight_synced = false;
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            entry.build_ripple_effect(shader_intensity, shader_distortion),
                        ));
                        SimulationEffectUpdate::Set(eff)
                    } else if awake_this || energy > 0.01 || entry.dragging {
                        entry.resting_catchlight_synced = false;
                        let eff: Arc<dyn IImmutableEffect> =
                            Arc::new(ImmutableCustomEffect::new(PhysicsKineticGlassEffect {
                                velocity: Vector::new(vel_px.x as f64, vel_px.y as f64),
                                energy: if entry.dragging {
                                    energy.max(0.58)
                                } else {
                                    energy.max(0.22)
                                } as f64,
                                tilt: rot as f64,
                                phase: entry.phase as f64,
                                intensity: shader_intensity as f64,
                                distortion: shader_distortion as f64,
                                accent: entry.accent,
                            }));
                        SimulationEffectUpdate::Set(eff)
                    } else if !entry.resting_catchlight_synced {
                        // Push a clean resting Fresnel catchlight once so the compositor can
                        // sleep at 0% CPU until the next pointer interaction or shockwave!
                        entry.resting_catchlight_synced = true;
                        let eff: Arc<dyn IImmutableEffect> =
                            Arc::new(ImmutableCustomEffect::new(PhysicsKineticGlassEffect {
                                velocity: Vector::new(0.0, 0.0),
                                energy: 0.16,
                                tilt: rot as f64,
                                phase: entry.phase as f64,
                                intensity: shader_intensity as f64,
                                distortion: shader_distortion as f64,
                                accent: entry.accent,
                            }));
                        SimulationEffectUpdate::Set(eff)
                    } else {
                        SimulationEffectUpdate::Keep
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
                patch = patch.with_center_point(
                    entry.size.x as f64 * 0.5,
                    entry.size.y as f64 * 0.5,
                    0.0,
                );
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

#[cfg(test)]
mod regression_tests {
    use super::*;
    use ferroui_base::Visual;

    fn controller(behavior: PhysicsCompositorBehavior) -> CompositorPhysicsController {
        let (_, controller) =
            CompositorPhysicsController::create("regression", 640.0, 480.0, behavior);
        controller.bind_visual(
            SimulationTarget::visual(Visual::new()),
            Vec2::new(200.0, 100.0),
            Vec2::new(128.0, 64.0),
            Color::from_argb(255, 0, 120, 215),
        );
        controller
    }

    #[test]
    fn collider_and_wall_bounds_match_visual_sizes() {
        let controller = controller(PhysicsCompositorBehavior::RigidBody);
        let state = controller.inner.borrow();
        let body = state.world.body(state.entries[0].body).unwrap();
        assert_eq!(body.aabb.extents(), Vec2::new(2.0, 1.0));
        let floor = state.world.body(state.walls[0]).unwrap();
        assert_eq!(floor.aabb.min.y, 7.5);
        assert!(floor.aabb.min.x <= 0.0 && floor.aabb.max.x >= 10.0);
    }

    #[test]
    fn pointer_drag_pins_body_and_restores_dynamic_mass_on_release() {
        let controller = controller(PhysicsCompositorBehavior::RigidBody);
        controller.begin_drag(0, Point::new(0.5, 0.5));
        controller.drag_by(0, Vec2::new(10.0, 0.0), 0.01);
        {
            let state = controller.inner.borrow();
            let body = state.world.body(state.entries[0].body).unwrap();
            assert!(body.is_kinematic());
            assert_eq!(body.inv_mass, 0.0);
        }
        controller.end_drag(0);
        let state = controller.inner.borrow();
        let body = state.world.body(state.entries[0].body).unwrap();
        assert!(body.is_dynamic());
        assert!(body.inv_mass > 0.0);
        assert!(body.linear_velocity.x > 0.0);
    }

    #[test]
    fn switching_to_rigid_mode_preserves_spring_angle_and_angular_velocity() {
        let controller = controller(PhysicsCompositorBehavior::ElasticSprings);
        {
            let mut state = controller.inner.borrow_mut();
            state.entries[0].spring_tilt.value = 0.4;
            state.entries[0].spring_tilt.velocity = 0.7;
        }
        controller.set_behavior(PhysicsCompositorBehavior::RigidBody);
        let state = controller.inner.borrow();
        let body = state.world.body(state.entries[0].body).unwrap();
        assert_eq!(body.angle, 0.4);
        assert_eq!(body.angular_velocity, 0.7);
    }
}

//! Hardware-accelerated CPU SIMD kernels for Structure-of-Arrays (SoA) physics integration
//! and batch spatial operations.
//!
//! Automatically selects:
//! - **ARM NEON** (`128-bit`, 4x `f32`) on `aarch64` (Apple Silicon, ARM64 Linux/Android/Windows)
//! - **AVX2 + FMA** (`256-bit`, 8x `f32`) or **SSE2** (`128-bit`, 4x `f32`) on `x86_64`
//! - **Auto-vectorized scalar** fallback on all other targets (e.g., WebAssembly)

/// Active hardware SIMD acceleration backend detected on this CPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimdBackend {
    /// ARM64 NEON 128-bit SIMD (4x f32 lanes).
    Neon,
    /// x86_64 AVX2 256-bit SIMD (8x f32 lanes).
    Avx2,
    /// x86_64 SSE2 128-bit SIMD (4x f32 lanes).
    Sse2,
    /// Portable scalar / compiler auto-vectorized path.
    Scalar,
}

impl SimdBackend {
    /// Returns the fastest SIMD backend available on the current CPU and build configuration.
    #[inline]
    pub fn active() -> Self {
        #[cfg(all(feature = "simd", target_arch = "aarch64"))]
        {
            Self::Neon
        }
        #[cfg(all(feature = "simd", target_arch = "x86_64"))]
        {
            if std::arch::is_x86_feature_detected!("avx2") {
                Self::Avx2
            } else {
                Self::Sse2
            }
        }
        #[cfg(not(all(feature = "simd", any(target_arch = "aarch64", target_arch = "x86_64"))))]
        {
            Self::Scalar
        }
    }

    /// Human-readable label for telemetry HUDs and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Neon => "CPU NEON (128-bit)",
            Self::Avx2 => "CPU AVX2 (256-bit)",
            Self::Sse2 => "CPU SSE2 (128-bit)",
            Self::Scalar => "CPU Scalar",
        }
    }
}

/// Symplectic Euler velocity + position integration over Structure-of-Arrays buffers.
///
/// Updates for each index `i`:
/// ```text
/// vx[i] = (vx[i] + gx * dt) * damping
/// vy[i] = (vy[i] + gy * dt) * damping
/// px[i] += vx[i] * dt
/// py[i] += vy[i] * dt
/// ```
///
/// All slices must have the same length `n`.
pub fn integrate_particles_soa(
    px: &mut [f32],
    py: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    gx: f32,
    gy: f32,
    damping: f32,
    dt: f32,
) {
    let n = px.len().min(py.len()).min(vx.len()).min(vy.len());
    let px = &mut px[..n];
    let py = &mut py[..n];
    let vx = &mut vx[..n];
    let vy = &mut vy[..n];

    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        // SAFETY: aarch64 always supports NEON; all slices have identical length `n`.
        unsafe { integrate_particles_neon(px, py, vx, vy, gx, gy, damping, dt) };
        return;
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 was verified at runtime; slices have identical length `n`.
            unsafe { integrate_particles_avx2(px, py, vx, vy, gx, gy, damping, dt) };
            return;
        }
        // SAFETY: x86_64 always supports SSE2; slices have identical length `n`.
        unsafe { integrate_particles_sse2(px, py, vx, vy, gx, gy, damping, dt) };
        return;
    }

    #[allow(unreachable_code)]
    integrate_particles_scalar(px, py, vx, vy, gx, gy, damping, dt);
}

/// Computes velocity + damping + gravity integration for rigid bodies with per-body `inv_mass`
/// and `gravity_scale` in SoA layout.
pub fn integrate_rigid_velocities_soa(
    vx: &mut [f32],
    vy: &mut [f32],
    fx: &[f32],
    fy: &[f32],
    inv_mass: &[f32],
    gravity_scale: &[f32],
    damping: &[f32],
    gx: f32,
    gy: f32,
    dt: f32,
) {
    let n = vx
        .len()
        .min(vy.len())
        .min(fx.len())
        .min(fy.len())
        .min(inv_mass.len())
        .min(gravity_scale.len())
        .min(damping.len());

    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        // SAFETY: aarch64 always has NEON and lengths are bounded by `n`.
        unsafe {
            integrate_rigid_velocities_neon(
                &mut vx[..n],
                &mut vy[..n],
                &fx[..n],
                &fy[..n],
                &inv_mass[..n],
                &gravity_scale[..n],
                &damping[..n],
                gx,
                gy,
                dt,
            );
        }
        return;
    }

    #[allow(unreachable_code)]
    for i in 0..n {
        let im = inv_mass[i];
        if im > 0.0 {
            let gs = gravity_scale[i];
            let damp = 1.0 / (1.0 + damping[i] * dt);
            vx[i] = (vx[i] + (gx * gs + fx[i] * im) * dt) * damp;
            vy[i] = (vy[i] + (gy * gs + fy[i] * im) * dt) * damp;
        }
    }
}

/// Applies a radial force field (attractor/repulsor) to a SoA particle array using SIMD.
pub fn apply_radial_field_soa(
    px: &[f32],
    py: &[f32],
    vx: &mut [f32],
    vy: &mut [f32],
    cx: f32,
    cy: f32,
    strength: f32,
    radius: f32,
    dt: f32,
) {
    let n = px.len().min(py.len()).min(vx.len()).min(vy.len());
    let r2_max = radius * radius;
    let strength_dt = strength * dt;

    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    {
        // SAFETY: aarch64 guarantees NEON; all slices are clamped to `n`.
        unsafe {
            apply_radial_field_neon(
                &px[..n],
                &py[..n],
                &mut vx[..n],
                &mut vy[..n],
                cx,
                cy,
                strength_dt,
                r2_max,
            );
        }
        return;
    }

    #[allow(unreachable_code)]
    for i in 0..n {
        let dx = cx - px[i];
        let dy = cy - py[i];
        let d2 = dx * dx + dy * dy;
        if d2 > 1e-4 && d2 < r2_max {
            let inv_d = 1.0 / d2.sqrt();
            let falloff = 1.0 - (d2 / r2_max);
            let scale = strength_dt * falloff * inv_d;
            vx[i] += dx * scale;
            vy[i] += dy * scale;
        }
    }
}

#[inline]
pub fn integrate_particles_scalar(
    px: &mut [f32],
    py: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    gx: f32,
    gy: f32,
    damping: f32,
    dt: f32,
) {
    let gxdt = gx * dt;
    let gydt = gy * dt;
    for i in 0..px.len() {
        let nvx = (vx[i] + gxdt) * damping;
        let nvy = (vy[i] + gydt) * damping;
        vx[i] = nvx;
        vy[i] = nvy;
        px[i] += nvx * dt;
        py[i] += nvy * dt;
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn integrate_particles_neon(
    px: &mut [f32],
    py: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    gx: f32,
    gy: f32,
    damping: f32,
    dt: f32,
) {
    use std::arch::aarch64::*;

    let n = px.len();
    let chunks = n / 4;
    let vgxdt = vdupq_n_f32(gx * dt);
    let vgydt = vdupq_n_f32(gy * dt);
    let vdamp = vdupq_n_f32(damping);
    let vdt = vdupq_n_f32(dt);

    for c in 0..chunks {
        let i = c * 4;
        let cur_vx = vld1q_f32(vx.as_ptr().add(i));
        let cur_vy = vld1q_f32(vy.as_ptr().add(i));
        let cur_px = vld1q_f32(px.as_ptr().add(i));
        let cur_py = vld1q_f32(py.as_ptr().add(i));

        let nvx = vmulq_f32(vaddq_f32(cur_vx, vgxdt), vdamp);
        let nvy = vmulq_f32(vaddq_f32(cur_vy, vgydt), vdamp);
        let npx = vfmaq_f32(cur_px, nvx, vdt);
        let npy = vfmaq_f32(cur_py, nvy, vdt);

        vst1q_f32(vx.as_mut_ptr().add(i), nvx);
        vst1q_f32(vy.as_mut_ptr().add(i), nvy);
        vst1q_f32(px.as_mut_ptr().add(i), npx);
        vst1q_f32(py.as_mut_ptr().add(i), npy);
    }

    let rem = chunks * 4;
    if rem < n {
        integrate_particles_scalar(
            &mut px[rem..],
            &mut py[rem..],
            &mut vx[rem..],
            &mut vy[rem..],
            gx,
            gy,
            damping,
            dt,
        );
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn integrate_rigid_velocities_neon(
    vx: &mut [f32],
    vy: &mut [f32],
    fx: &[f32],
    fy: &[f32],
    inv_mass: &[f32],
    gravity_scale: &[f32],
    damping: &[f32],
    gx: f32,
    gy: f32,
    dt: f32,
) {
    use std::arch::aarch64::*;

    let n = vx.len();
    let chunks = n / 4;
    let vgx = vdupq_n_f32(gx);
    let vgy = vdupq_n_f32(gy);
    let vdt = vdupq_n_f32(dt);
    let vone = vdupq_n_f32(1.0);
    let vzero = vdupq_n_f32(0.0);

    for c in 0..chunks {
        let i = c * 4;
        let im = vld1q_f32(inv_mass.as_ptr().add(i));
        let dynamic_mask = vcgtq_f32(im, vzero);

        let cur_vx = vld1q_f32(vx.as_ptr().add(i));
        let cur_vy = vld1q_f32(vy.as_ptr().add(i));
        let vfx = vld1q_f32(fx.as_ptr().add(i));
        let vfy = vld1q_f32(fy.as_ptr().add(i));
        let vgs = vld1q_f32(gravity_scale.as_ptr().add(i));
        let vd = vld1q_f32(damping.as_ptr().add(i));

        let ax = vfmaq_f32(vmulq_f32(vgx, vgs), vfx, im);
        let ay = vfmaq_f32(vmulq_f32(vgy, vgs), vfy, im);
        let denom = vfmaq_f32(vone, vd, vdt);
        let damp = vdivq_f32(vone, denom);

        let nvx = vmulq_f32(vfmaq_f32(cur_vx, ax, vdt), damp);
        let nvy = vmulq_f32(vfmaq_f32(cur_vy, ay, vdt), damp);

        let out_vx = vbslq_f32(dynamic_mask, nvx, cur_vx);
        let out_vy = vbslq_f32(dynamic_mask, nvy, cur_vy);

        vst1q_f32(vx.as_mut_ptr().add(i), out_vx);
        vst1q_f32(vy.as_mut_ptr().add(i), out_vy);
    }

    for i in (chunks * 4)..n {
        let im = inv_mass[i];
        if im > 0.0 {
            let gs = gravity_scale[i];
            let damp = 1.0 / (1.0 + damping[i] * dt);
            vx[i] = (vx[i] + (gx * gs + fx[i] * im) * dt) * damp;
            vy[i] = (vy[i] + (gy * gs + fy[i] * im) * dt) * damp;
        }
    }
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn apply_radial_field_neon(
    px: &[f32],
    py: &[f32],
    vx: &mut [f32],
    vy: &mut [f32],
    cx: f32,
    cy: f32,
    strength_dt: f32,
    r2_max: f32,
) {
    use std::arch::aarch64::*;

    let n = px.len();
    let chunks = n / 4;
    let vcx = vdupq_n_f32(cx);
    let vcy = vdupq_n_f32(cy);
    let vr2_max = vdupq_n_f32(r2_max);
    let vinv_r2_max = vdupq_n_f32(1.0 / r2_max.max(1e-6));
    let veps = vdupq_n_f32(1e-4);
    let vone = vdupq_n_f32(1.0);
    let vzero = vdupq_n_f32(0.0);
    let vstr = vdupq_n_f32(strength_dt);

    for c in 0..chunks {
        let i = c * 4;
        let x = vld1q_f32(px.as_ptr().add(i));
        let y = vld1q_f32(py.as_ptr().add(i));
        let dx = vsubq_f32(vcx, x);
        let dy = vsubq_f32(vcy, y);
        let d2 = vfmaq_f32(vmulq_f32(dx, dx), dy, dy);

        let in_min = vcgtq_f32(d2, veps);
        let in_max = vcltq_f32(d2, vr2_max);
        let active = vandq_u32(in_min, in_max);

        let safe_d2 = vmaxq_f32(d2, veps);
        let inv_d = vdivq_f32(vone, vsqrtq_f32(safe_d2));
        let falloff = vsubq_f32(vone, vmulq_f32(safe_d2, vinv_r2_max));
        let scale = vbslq_f32(active, vmulq_f32(vmulq_f32(vstr, falloff), inv_d), vzero);

        let cur_vx = vld1q_f32(vx.as_ptr().add(i));
        let cur_vy = vld1q_f32(vy.as_ptr().add(i));
        vst1q_f32(vx.as_mut_ptr().add(i), vfmaq_f32(cur_vx, dx, scale));
        vst1q_f32(vy.as_mut_ptr().add(i), vfmaq_f32(cur_vy, dy, scale));
    }

    for i in (chunks * 4)..n {
        let dx = cx - px[i];
        let dy = cy - py[i];
        let d2 = dx * dx + dy * dy;
        if d2 > 1e-4 && d2 < r2_max {
            let inv_d = 1.0 / d2.sqrt();
            let falloff = 1.0 - (d2 / r2_max);
            let scale = strength_dt * falloff * inv_d;
            vx[i] += dx * scale;
            vy[i] += dy * scale;
        }
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn integrate_particles_avx2(
    px: &mut [f32],
    py: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    gx: f32,
    gy: f32,
    damping: f32,
    dt: f32,
) {
    use std::arch::x86_64::*;

    let n = px.len();
    let chunks = n / 8;
    let vgxdt = _mm256_set1_ps(gx * dt);
    let vgydt = _mm256_set1_ps(gy * dt);
    let vdamp = _mm256_set1_ps(damping);
    let vdt = _mm256_set1_ps(dt);

    for c in 0..chunks {
        let i = c * 8;
        let cur_vx = _mm256_loadu_ps(vx.as_ptr().add(i));
        let cur_vy = _mm256_loadu_ps(vy.as_ptr().add(i));
        let cur_px = _mm256_loadu_ps(px.as_ptr().add(i));
        let cur_py = _mm256_loadu_ps(py.as_ptr().add(i));

        let nvx = _mm256_mul_ps(_mm256_add_ps(cur_vx, vgxdt), vdamp);
        let nvy = _mm256_mul_ps(_mm256_add_ps(cur_vy, vgydt), vdamp);
        let npx = _mm256_add_ps(cur_px, _mm256_mul_ps(nvx, vdt));
        let npy = _mm256_add_ps(cur_py, _mm256_mul_ps(nvy, vdt));

        _mm256_storeu_ps(vx.as_mut_ptr().add(i), nvx);
        _mm256_storeu_ps(vy.as_mut_ptr().add(i), nvy);
        _mm256_storeu_ps(px.as_mut_ptr().add(i), npx);
        _mm256_storeu_ps(py.as_mut_ptr().add(i), npy);
    }

    let rem = chunks * 8;
    if rem < n {
        integrate_particles_scalar(
            &mut px[rem..],
            &mut py[rem..],
            &mut vx[rem..],
            &mut vy[rem..],
            gx,
            gy,
            damping,
            dt,
        );
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn integrate_particles_sse2(
    px: &mut [f32],
    py: &mut [f32],
    vx: &mut [f32],
    vy: &mut [f32],
    gx: f32,
    gy: f32,
    damping: f32,
    dt: f32,
) {
    use std::arch::x86_64::*;

    let n = px.len();
    let chunks = n / 4;
    let vgxdt = _mm_set1_ps(gx * dt);
    let vgydt = _mm_set1_ps(gy * dt);
    let vdamp = _mm_set1_ps(damping);
    let vdt = _mm_set1_ps(dt);

    for c in 0..chunks {
        let i = c * 4;
        let cur_vx = _mm_loadu_ps(vx.as_ptr().add(i));
        let cur_vy = _mm_loadu_ps(vy.as_ptr().add(i));
        let cur_px = _mm_loadu_ps(px.as_ptr().add(i));
        let cur_py = _mm_loadu_ps(py.as_ptr().add(i));

        let nvx = _mm_mul_ps(_mm_add_ps(cur_vx, vgxdt), vdamp);
        let nvy = _mm_mul_ps(_mm_add_ps(cur_vy, vgydt), vdamp);
        let npx = _mm_add_ps(cur_px, _mm_mul_ps(nvx, vdt));
        let npy = _mm_add_ps(cur_py, _mm_mul_ps(nvy, vdt));

        _mm_storeu_ps(vx.as_mut_ptr().add(i), nvx);
        _mm_storeu_ps(vy.as_mut_ptr().add(i), nvy);
        _mm_storeu_ps(px.as_mut_ptr().add(i), npx);
        _mm_storeu_ps(py.as_mut_ptr().add(i), npy);
    }

    let rem = chunks * 4;
    if rem < n {
        integrate_particles_scalar(
            &mut px[rem..],
            &mut py[rem..],
            &mut vx[rem..],
            &mut vy[rem..],
            gx,
            gy,
            damping,
            dt,
        );
    }
}

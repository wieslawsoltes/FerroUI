// Ported from:
// https://svn.webkit.org/repository/webkit/trunk/Source/WebCore/platform/graphics/SpringSolver.h
/*
 * Copyright (C) 2016 Apple Inc. All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY APPLE INC. AND ITS CONTRIBUTORS ``AS IS''
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
 * THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL APPLE INC. OR ITS CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF
 * THE POSSIBILITY OF SUCH DAMAGE.
 */

use std::f64::consts::PI;

/// Solves the motion of a damped spring: maps a time to the position of a
/// spring that is released at `0` and settles at `1`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpringSolver {
    w0: f64,
    zeta: f64,
    wd: f64,
    a: f64,
    b: f64,
}

impl SpringSolver {
    /// Creates a solver from the period of the undamped oscillation in
    /// seconds, the damping ratio and the initial velocity.
    pub fn from_period(period_seconds: f64, zeta: f64, initial_velocity: f64) -> Self {
        // T is time period [s]
        // T = (2*PI / sqrt(k)) * sqrt(m)
        // wn is natural frequency of the system [Hz] [1/s]
        // wn = 2*PI / T
        Self::from_natural_frequency(2.0 * PI / period_seconds, zeta, initial_velocity)
    }

    /// Creates a solver from the mass `m`, the stiffness `k`, the damping
    /// coefficient `c` and the initial velocity.
    pub fn new(m: f64, k: f64, c: f64, initial_velocity: f64) -> Self {
        // wn is natural frequency of the system [Hz] [1/s]
        // wn = sqrt(k / m)
        // Cc is critical damping coefficient
        // Cc = 2 * sqrt(k * m)
        // Cc = 2 * m * wn
        // Cc = 2 * m * sqrt(k / m)
        // zeta is damping ratio
        // zeta = c / Cc
        Self::from_natural_frequency((k / m).sqrt(), c / (2.0 * (k * m).sqrt()), initial_velocity)
    }

    /// Creates a solver from the natural frequency, the damping ratio and
    /// the initial velocity.
    pub fn from_natural_frequency(wn: f64, zeta: f64, initial_velocity: f64) -> Self {
        let w0 = wn;
        if zeta < 1.0 {
            // Under-damped.
            let wd = w0 * (1.0 - zeta * zeta).sqrt();
            Self { w0, zeta, wd, a: 1.0, b: (zeta * w0 + -initial_velocity) / wd }
        } else {
            // Critically damped (ignoring over-damped case for now).
            Self { w0, zeta, wd: 0.0, a: 1.0, b: -initial_velocity + w0 }
        }
    }

    /// The position of the spring at time `t`.
    pub fn solve(&self, t: f64) -> f64 {
        let t = if self.zeta < 1.0 {
            // Under-damped
            (-t * self.zeta * self.w0).exp() * (self.a * (self.wd * t).cos() + self.b * (self.wd * t).sin())
        } else {
            // Critically damped (ignoring over-damped case for now).
            (self.a + self.b * t) * (-t * self.w0).exp()
        };

        // Map range from [1..0] to [0..1].
        1.0 - t
    }
}

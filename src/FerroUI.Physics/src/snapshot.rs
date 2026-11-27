//! Deterministic world snapshot & ring-buffer time-travel history for rollback netcode,
//! instant replay, and studio timeline scrubbing.

use crate::body::{BodyHandle, RigidBody};
use crate::joints::JointHandle;
use crate::math::{Rot2, Vec2};
use crate::world::PhysicsWorld;
use std::collections::VecDeque;

/// Compact snapshot of a single rigid body's dynamic state.
#[derive(Clone, Copy, Debug)]
pub struct BodySnapshot {
    pub handle: BodyHandle,
    pub position: Vec2,
    pub angle: f32,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub is_sleeping: bool,
    pub sleep_time: f32,
}

/// Complete deterministic snapshot of a [`PhysicsWorld`] at a single simulation tick.
#[derive(Clone, Debug)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub bodies: Vec<BodySnapshot>,
    pub joint_broken: Vec<(JointHandle, bool)>,
}

impl WorldSnapshot {
    pub fn capture(world: &PhysicsWorld, tick: u64) -> Self {
        let bodies = world
            .bodies()
            .map(|b: &RigidBody| BodySnapshot {
                handle: b.handle,
                position: b.position,
                angle: b.angle,
                linear_velocity: b.linear_velocity,
                angular_velocity: b.angular_velocity,
                is_sleeping: b.is_sleeping,
                sleep_time: b.sleep_time,
            })
            .collect();
        let joint_broken = world.all_joints().map(|j| (j.handle, j.broken)).collect();
        Self { tick, bodies, joint_broken }
    }

    pub fn restore(&self, world: &mut PhysicsWorld) {
        for snap in &self.bodies {
            if let Some(b) = world.body_mut(snap.handle) {
                b.position = snap.position;
                b.prev_position = snap.position;
                b.angle = snap.angle;
                b.rotation = Rot2::from_radians(snap.angle);
                b.prev_rotation = b.rotation;
                b.linear_velocity = snap.linear_velocity;
                b.angular_velocity = snap.angular_velocity;
                b.force = Vec2::ZERO;
                b.torque = 0.0;
                b.is_sleeping = snap.is_sleeping;
                b.sleep_time = snap.sleep_time;
                b.aabb = b.shape.compute_aabb(b.transform());
            }
        }
        for &(jh, broken) in &self.joint_broken {
            if let Some(j) = world.joint_mut(jh) {
                j.broken = broken;
            }
        }
    }
}

/// Ring-buffer history supporting smooth timeline scrubbing, step-back, and rollback.
#[derive(Clone, Debug)]
pub struct WorldHistory {
    frames: VecDeque<WorldSnapshot>,
    capacity: usize,
    cursor: usize,
    tick_counter: u64,
}

impl WorldHistory {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(8);
        Self { frames: VecDeque::with_capacity(cap), capacity: cap, cursor: 0, tick_counter: 0 }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.cursor = 0;
        self.tick_counter = 0;
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Records the current world state, truncating any future redo branch if we scrubbed back.
    pub fn record(&mut self, world: &PhysicsWorld) {
        if self.cursor + 1 < self.frames.len() {
            self.frames.truncate(self.cursor + 1);
        }
        if self.frames.len() >= self.capacity {
            self.frames.pop_front();
        }
        self.tick_counter += 1;
        self.frames.push_back(WorldSnapshot::capture(world, self.tick_counter));
        self.cursor = self.frames.len().saturating_sub(1);
    }

    /// Steps one frame backward in recorded history and restores `world`.
    pub fn step_back(&mut self, world: &mut PhysicsWorld) -> bool {
        if self.frames.is_empty() || self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.frames[self.cursor].restore(world);
        true
    }

    /// Steps one frame forward in recorded history (if previously stepped back).
    pub fn step_forward(&mut self, world: &mut PhysicsWorld) -> bool {
        if self.cursor + 1 >= self.frames.len() {
            return false;
        }
        self.cursor += 1;
        self.frames[self.cursor].restore(world);
        true
    }

    /// Scrubs to an exact normalized position `t` in `0.0..=1.0`.
    pub fn scrub(&mut self, world: &mut PhysicsWorld, normalized_t: f32) -> bool {
        if self.frames.is_empty() {
            return false;
        }
        let max_idx = self.frames.len() - 1;
        let idx = ((normalized_t.clamp(0.0, 1.0) * max_idx as f32).round() as usize).min(max_idx);
        self.cursor = idx;
        self.frames[idx].restore(world);
        true
    }

    pub fn normalized_cursor(&self) -> f32 {
        if self.frames.len() <= 1 {
            1.0
        } else {
            self.cursor as f32 / (self.frames.len() - 1) as f32
        }
    }
}

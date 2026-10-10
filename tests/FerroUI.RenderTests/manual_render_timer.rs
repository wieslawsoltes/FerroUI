//! Port of upstream's `ManualRenderTimer.cs` of the render tests.

use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default)]
pub struct ManualRenderTimer {
    tick: Mutex<Option<RenderTimerTick>>,
}

impl ManualRenderTimer {
    pub fn new() -> ManualRenderTimer {
        ManualRenderTimer::default()
    }

    pub fn trigger_tick(&self) {
        let tick = self.tick.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(tick) = tick {
            tick(Duration::ZERO);
        }
    }
}

impl IRenderTimer for ManualRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.tick.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        *self.tick.lock().unwrap_or_else(|e| e.into_inner()) = value;
    }

    fn runs_in_background(&self) -> bool {
        false
    }
}

//! Small, read-only evidence for native multi-click routing. No document data
//! is copied and no allocation occurs while an input event is being handled.
use super::*;

const CAPACITY: usize = 8;

#[derive(Clone, Copy)]
struct PointerPress {
    serial: u64,
    timestamp: u64,
    click_count: i32,
    buttons: u8,
    screen: Point,
    world: Point,
    viewport: Viewport,
    selection_count: usize,
    frame: u64,
}

pub(super) struct PointerPresses {
    serial: u64,
    entries: [Option<PointerPress>; CAPACITY],
}

impl Default for PointerPresses {
    fn default() -> Self { Self { serial: 0, entries: [None; CAPACITY] } }
}

impl PointerPresses {
    pub(super) fn record(&mut self, event: &PointerPressedEventArgs, screen: Point,
        world: Point, viewport: Viewport, selection_count: usize, frame: u64) {
        let properties = event.properties();
        // The DOM buttons convention makes the native and browser records
        // comparable without copying an event or retaining its input target.
        let buttons = u8::from(properties.is_left_button_pressed)
            | (u8::from(properties.is_right_button_pressed) << 1)
            | (u8::from(properties.is_middle_button_pressed) << 2)
            | (u8::from(properties.is_x_button_1_pressed) << 3)
            | (u8::from(properties.is_x_button_2_pressed) << 4);
        self.serial = self.serial.saturating_add(1);
        let index = ((self.serial - 1) % CAPACITY as u64) as usize;
        self.entries[index] = Some(PointerPress {
            serial: self.serial, timestamp: event.timestamp(), click_count: event.click_count(),
            buttons, screen, world, viewport, selection_count, frame,
        });
    }

    pub(super) fn json(&self) -> serde_json::Value {
        let mut entries: Vec<_> = self.entries.iter().flatten().collect();
        entries.sort_by_key(|entry| entry.serial);
        serde_json::json!({"serial": self.serial, "capacity": CAPACITY,
            "timestampUnit": "native monotonic milliseconds", "entries": entries.into_iter().map(|entry| {
                serde_json::json!({"serial":entry.serial,"timestamp":entry.timestamp,
                    "clickCount":entry.click_count,"buttons":entry.buttons,
                    "screen":{"x":entry.screen.x,"y":entry.screen.y},
                    "world":{"x":entry.world.x,"y":entry.world.y},
                    "zoom":entry.viewport.zoom,"pan":{"x":entry.viewport.pan.x,"y":entry.viewport.pan.y},
                    "selectionCountBefore":entry.selection_count,"renderedFramesBefore":entry.frame})
            }).collect::<Vec<_>>()})
    }
}

//! EWMH client geometry is in physical root pixels. The WM applies size hints and owns
//! decorations; clients request a rectangle using static gravity and client dimensions.
use super::{Session, SnapSide};
use ferroui_base::PixelRect;
use ferroui_x11::x11_structs::Gravity;
use ferroui_x11::xlib::{self, XID};

impl Session {
    pub fn toggle_maximized(&self, id: u32) -> bool {
        let clients = self.clients.borrow();
        let Some(Some(client)) = clients.get(&XID::from(id)) else {
            return false;
        };
        if client.fullscreen || !client.can_maximize {
            return false;
        }
        self.pending_snaps.borrow_mut().remove(&id.into());
        self.set_maximized(id.into(), !client.maximized)
    }

    pub fn toggle_fullscreen(&self, id: u32) -> bool {
        let clients = self.clients.borrow();
        let Some(Some(client)) = clients.get(&XID::from(id)) else {
            return false;
        };
        let a = self.platform.info().atoms();
        self.pending_snaps.borrow_mut().remove(&id.into());
        client.can_fullscreen
            && self.message(
                id.into(),
                a._NET_WM_STATE,
                &[XID::from(!client.fullscreen), a._NET_WM_STATE_FULLSCREEN, 0, 2],
            )
    }

    pub fn toggle_above(&self, id: u32) -> bool {
        let clients = self.clients.borrow();
        let Some(Some(client)) = clients.get(&XID::from(id)) else {
            return false;
        };
        let a = self.platform.info().atoms();
        client.can_raise
            && self.message(
                id.into(),
                a._NET_WM_STATE,
                &[XID::from(!client.above), a._NET_WM_STATE_ABOVE, 0, 2],
            )
    }

    fn set_maximized(&self, id: XID, maximize: bool) -> bool {
        let a = self.platform.info().atoms();
        self.message(
            id,
            a._NET_WM_STATE,
            &[
                XID::from(maximize),
                a._NET_WM_STATE_MAXIMIZED_HORZ,
                a._NET_WM_STATE_MAXIMIZED_VERT,
                2,
            ],
        )
    }

    pub fn snap(&self, id: u32, side: SnapSide) -> bool {
        let id = XID::from(id);
        let clients = self.clients.borrow();
        let Some(Some(client)) = clients.get(&id) else {
            return false;
        };
        if !client.can_snap || client.fullscreen {
            return false;
        }
        if client.partially_maximized {
            // A maximized window can have different frame extents. Wait for the WM's property
            // notification before using the restored frame and choosing its monitor.
            if !self.set_maximized(id, false) {
                return false;
            }
            self.pending_snaps.borrow_mut().insert(id, side);
            true
        } else {
            self.place_half(id, side)
        }
    }

    pub(super) fn place_half(&self, id: XID, side: SnapSide) -> bool {
        let info = self.platform.info();
        let display = info.display();
        let Some(geometry) = xlib::x_get_geometry(display, id) else {
            return false;
        };
        let Some((x, y, _)) = xlib::x_translate_coordinates(display, id, info.root_window(), 0, 0) else {
            return false;
        };
        let bounds = PixelRect::new(x, y, geometry.width, geometry.height);
        let screens = self.platform.screens();
        let Some(monitor) = screens.screen_from_rect(bounds).or_else(|| {
            // An output may have just disconnected, leaving the old frame offscreen.
            let all = screens.all_screens();
            all.iter()
                .find(|screen| screen.is_primary())
                .or_else(|| all.first())
                .cloned()
        }) else {
            return false;
        };
        let a = info.atoms();
        let workspaces = self.longs(info.root_window(), a._NET_WORKAREA, a.CARDINAL, 4 * 256);
        let desktop = self
            .clients
            .borrow()
            .get(&id)
            .and_then(Option::as_ref)
            .and_then(|window| window.workspace)
            .unwrap_or_else(|| self.snapshot.borrow().current_workspace);
        let monitor_bounds = monitor.bounds();
        let per_output = (screens.screen_count() > 1)
            .then(|| {
                let atom = xlib::x_intern_atom(display, &format!("_GTK_WORKAREAS_D{desktop}"), true);
                if atom == 0 {
                    return None;
                }
                self.longs(info.root_window(), atom, a.CARDINAL, 4 * 256)
                    .chunks_exact(4)
                    .map(|wa| PixelRect::new(wa[0] as i32, wa[1] as i32, wa[2] as i32, wa[3] as i32))
                    .filter(|rect| rect.width > 0 && rect.height > 0 && monitor_bounds.contains_rect(*rect))
                    .max_by_key(|rect| i64::from(rect.width) * i64::from(rect.height))
            })
            .flatten();
        let work = per_output
            .or_else(|| {
                workspaces.chunks_exact(4).nth(desktop).and_then(|wa| {
                    let rect = monitor_bounds.intersect(PixelRect::new(
                        wa[0] as i32,
                        wa[1] as i32,
                        wa[2] as i32,
                        wa[3] as i32,
                    ));
                    (rect.width > 0 && rect.height > 0).then_some(rect)
                })
            })
            .unwrap_or_else(|| monitor.working_area());
        let extents = self.longs(id, a._NET_FRAME_EXTENTS, a.CARDINAL, 4);
        let Some(client) = half_client(work, &extents, side) else {
            return false;
        };
        // Bits 8–11 mark all four fields valid. Source 2 is a pager/desktop-shell request.
        let flags = Gravity::StaticGravity as XID | (0xf << 8) | (2 << 12);
        self.message(
            id,
            a._NET_MOVERESIZE_WINDOW,
            &[
                flags,
                client.x as XID,
                client.y as XID,
                client.width as XID,
                client.height as XID,
            ],
        )
    }
}

fn half_client(work: PixelRect, extents: &[XID], side: SnapSide) -> Option<PixelRect> {
    let (left, right, top, bottom) = match extents {
        [l, r, t, b] => (
            i32::try_from(*l).ok()?,
            i32::try_from(*r).ok()?,
            i32::try_from(*t).ok()?,
            i32::try_from(*b).ok()?,
        ),
        _ => (0, 0, 0, 0),
    };
    let split = work.width / 2;
    let (offset, frame_width) = match side {
        SnapSide::Left => (0, split),
        SnapSide::Right => (split, work.width - split),
    };
    let width = frame_width.checked_sub(left)?.checked_sub(right)?;
    let height = work.height.checked_sub(top)?.checked_sub(bottom)?;
    (width > 0 && height > 0).then_some(PixelRect::new(
        work.x.checked_add(offset)?.checked_add(left)?,
        work.y.checked_add(top)?,
        width,
        height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn halves_share_an_edge_in_physical_pixels_on_an_offset_monitor() {
        let work = PixelRect::new(-1601, 64, 1601, 900);
        let extents = [5, 3, 31, 4];
        assert_eq!(
            half_client(work, &extents, SnapSide::Left),
            Some(PixelRect::new(-1596, 95, 792, 865))
        );
        assert_eq!(
            half_client(work, &extents, SnapSide::Right),
            Some(PixelRect::new(-796, 95, 793, 865))
        );
        assert!(half_client(PixelRect::new(0, 0, 10, 10), &extents, SnapSide::Right).is_none());
    }
}

//! Window placement, docking and sizing.
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::window::Window;
use wry::WebView;
pub(crate) struct Panel {
    pub window: Window,
    pub webview: WebView,
}
pub(crate) type VRect = (i32, i32, i32, i32);
#[cfg(target_os = "windows")]
pub(crate) fn visible_rect(window: &Window) -> Option<VRect> {
    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
    };
    let mut r = RECT::default();
    let h = HWND(window.hwnd() as *mut _);
    let ok = unsafe {
        DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_ok();
    if ok && r.right > r.left && r.bottom > r.top {
        Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
    } else {
        let p = window.outer_position().ok()?;
        let z = window.outer_size();
        Some((p.x, p.y, z.width as i32, z.height as i32))
    }
}
#[cfg(not(target_os = "windows"))]
pub(crate) fn visible_rect(window: &Window) -> Option<VRect> {
    let p = window.outer_position().ok()?;
    let z = window.outer_size();
    Some((p.x,p.y,z.width as i32,z.height as i32))
}
pub(crate) fn set_visible_position(window: &Window, x: i32, y: i32) {
    if let (Some(r), Ok(p)) = (visible_rect(window), window.outer_position()) {
        window.set_outer_position(tao::dpi::PhysicalPosition::new(
            p.x + x - r.0,
            p.y + y - r.1,
        ));
    }
}
pub(crate) fn base_size(name: &str) -> (f64, f64) {
    match name {
        "player" => (365.0, 155.0),
        "equalizer" => (365.0, 180.0),
        "playlist" => (365.0, 470.0),
        _ => (420.0, 470.0),
    }
}
pub(crate) fn dock_default(panels: &HashMap<String, Panel>) {
    if let Some(p) = visible_rect(&panels["player"].window) {
        set_visible_position(&panels["equalizer"].window, p.0, p.1 + p.3);
        if let Some(e) = visible_rect(&panels["equalizer"].window) {
            set_visible_position(&panels["playlist"].window, e.0, e.1 + e.3);
        }
        set_visible_position(&panels["library"].window, p.0 + p.2, p.1);
    }
}
pub(crate) fn snap_window(
    mut x: i32,
    mut y: i32,
    w: i32,
    h: i32,
    others: &[(i32, i32, i32, i32)],
    margin: i32,
) -> (i32, i32) {
    for &(ox, oy, ow, oh) in others {
        let horizontal_overlap = x < ox + ow + margin && x + w > ox - margin;
        let vertical_overlap = y < oy + oh + margin && y + h > oy - margin;
        if horizontal_overlap && (y - (oy + oh)).abs() <= margin {
            y = oy + oh;
            if (x - ox).abs() <= margin {
                x = ox;
            }
            return (x, y);
        } else if horizontal_overlap && ((y + h) - oy).abs() <= margin {
            y = oy - h;
            if (x - ox).abs() <= margin {
                x = ox;
            }
            return (x, y);
        } else if vertical_overlap && (x - (ox + ow)).abs() <= margin {
            x = ox + ow;
            if (y - oy).abs() <= margin {
                y = oy;
            }
            return (x, y);
        } else if vertical_overlap && ((x + w) - ox).abs() <= margin {
            x = ox - w;
            if (y - oy).abs() <= margin {
                y = oy;
            }
            return (x, y);
        }
    }
    (x, y)
}
pub(crate) fn intersects(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}
pub(crate) fn free_position(
    rect: (i32, i32, i32, i32),
    others: &[(i32, i32, i32, i32)],
    screen: Option<(i32, i32, i32, i32)>,
) -> Option<(i32, i32)> {
    if !others.iter().any(|&other| intersects(rect, other)) {
        return None;
    }
    let (x, y, w, h) = rect;
    let mut candidates = Vec::new();
    for &(ox, oy, ow, oh) in others {
        candidates.extend([(ox, oy + oh), (ox, oy - h), (ox + ow, oy), (ox - w, oy)]);
    }
    candidates.retain(|&(cx, cy)| {
        !others
            .iter()
            .any(|&other| intersects((cx, cy, w, h), other))
    });
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by_key(|&(cx, cy)| {
        let outside = screen
            .map(|(sx, sy, sw, sh)| cx < sx || cy < sy || cx + w > sx + sw || cy + h > sy + sh)
            .unwrap_or(false);
        let dx = (cx as i64) - (x as i64);
        let dy = (cy as i64) - (y as i64);
        (outside, dx * dx + dy * dy)
    });
    candidates.first().copied()
}
pub(crate) fn arrange_panel(panels: &HashMap<String, Panel>, name: &str) {
    let Some(panel) = panels.get(name) else {
        return;
    };
    if !panel.window.is_visible() {
        return;
    }
    let Some(pos) = visible_rect(&panel.window) else {
        return;
    };
    let others: Vec<_> = panels
        .iter()
        .filter(|(other, p)| other.as_str() != name && p.window.is_visible())
        .filter_map(|(_, p)| visible_rect(&p.window))
        .collect();
    let screen = panel.window.current_monitor().map(|m| {
        let p = m.position();
        let s = m.size();
        (p.x, p.y, s.width as i32, s.height as i32)
    });
    if let Some((x, y)) = free_position(pos, &others, screen) {
        set_visible_position(&panel.window, x, y);
    }
}
pub(crate) fn fifo_position(
    last: (i32, i32, i32, i32),
    new_size: (i32, i32),
    others: &[(i32, i32, i32, i32)],
    screen: Option<(i32, i32, i32, i32)>,
) -> (i32, i32) {
    let (x, y, _w, h) = last;
    let (nw, nh) = new_size;
    let mut candidates = vec![(x, y + h)];
    for &(ox, oy, ow, oh) in others.iter().rev() {
        candidates.extend([(ox, oy + oh), (ox + ow, oy), (ox - nw, oy), (ox, oy - nh)]);
    }
    let fits = |cx: i32, cy: i32| {
        screen
            .map(|(sx, sy, sw, sh)| {
                cx >= sx && cy >= sy && cx + nw <= sx + sw && cy + nh <= sy + sh
            })
            .unwrap_or(true)
    };
    candidates
        .iter()
        .copied()
        .find(|&(cx, cy)| fits(cx, cy) && !others.iter().any(|&r| intersects((cx, cy, nw, nh), r)))
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .find(|&(cx, cy)| !others.iter().any(|&r| intersects((cx, cy, nw, nh), r)))
        })
        .unwrap_or((x, y + h))
}
/// Move panels docked directly below a resized panel, keeping their edges joined.
pub(crate) fn shift_stack_below(
    panels: &HashMap<String, Panel>,
    name: &str,
    old: VRect,
    delta: i32,
) {
    if delta == 0 {
        return;
    }
    let mut edge = old.1 + old.3;
    let mut visited = vec![name.to_owned()];
    loop {
        let next = panels
            .iter()
            .filter(|(n, p)| !visited.contains(n) && p.window.is_visible())
            .find_map(|(n, p)| {
                visible_rect(&p.window)
                    .filter(|r| (r.1 - edge).abs() <= 2 && r.0 < old.0 + old.2 && r.0 + r.2 > old.0)
                    .map(|r| (n.clone(), r))
            });
        let Some((n, r)) = next else { break };
        visited.push(n.clone());
        edge = r.1 + r.3;
        set_visible_position(&panels[&n].window, r.0, r.1 + delta);
    }
}
pub(crate) fn open_panel_fifo(
    panels: &HashMap<String, Panel>,
    name: &str,
    order: &mut Vec<String>,
    last_player_pos: &mut Option<tao::dpi::PhysicalPosition<i32>>,
) {
    let Some(panel) = panels.get(name) else {
        return;
    };
    let mut visible: Vec<_> = order
        .iter()
        .filter(|n| n.as_str() != name)
        .filter_map(|n| panels.get(n))
        .filter(|p| p.window.is_visible())
        .filter_map(|p| visible_rect(&p.window))
        .collect();
    if let Some(mut last) = visible.last().copied() {
        let size = visible_rect(&panel.window)
            .map(|r| (r.2, r.3))
            .unwrap_or((365, 180));
        let screen = panel.window.current_monitor().map(|m| {
            let p = m.position();
            let s = m.size();
            (p.x, p.y, s.width as i32, s.height as i32)
        });
        // Move the stack up when that creates enough room to append the new panel below.
        if let Some((_, sy, _, sh)) = screen {
            let overflow = (last.1 + last.3 + size.1 - (sy + sh)).max(0);
            let top = visible.iter().map(|r| r.1).min().unwrap_or(sy);
            if overflow > 0 && top - overflow >= sy {
                for p in panels.values() {
                    if let Ok(pt) = p.window.outer_position() {
                        p.window.set_outer_position(tao::dpi::PhysicalPosition::new(
                            pt.x,
                            pt.y - overflow,
                        ));
                    }
                }
                if let Some(pt) = last_player_pos.as_mut() {
                    pt.y -= overflow;
                }
                for rect in &mut visible {
                    rect.1 -= overflow;
                }
                last.1 -= overflow;
            }
        }
        let (x, y) = fifo_position(last, size, &visible, screen);
        set_visible_position(&panel.window, x, y);
    }
    panel.window.set_visible(true);
    panel.window.set_focus();
    order.retain(|n| n != name);
    order.push(name.to_string());
}

#[cfg(test)]
mod layout_tests {
    use super::{fifo_position, free_position, intersects, snap_window};
    #[test]
    fn snaps_below_and_aligns_edges() {
        assert_eq!(
            snap_window(104, 258, 365, 180, &[(100, 100, 365, 155)], 12),
            (100, 255)
        );
    }
    #[test]
    fn preserves_free_movement_far_away() {
        assert_eq!(
            snap_window(400, 500, 365, 180, &[(100, 100, 365, 155)], 12),
            (400, 500)
        );
    }
    #[test]
    fn moves_overlapping_window_to_free_edge() {
        let r = (100, 250, 365, 180);
        let others = [(100, 100, 365, 155)];
        let p = free_position(r, &others, Some((0, 0, 1920, 1080))).unwrap();
        assert!(!intersects((p.0, p.1, r.2, r.3), others[0]));
        assert_eq!(p, (100, 255));
    }
    #[test]
    fn leaves_non_overlapping_window_in_place() {
        assert_eq!(
            free_position((100, 255, 365, 180), &[(100, 100, 365, 155)], None),
            None
        );
    }
    #[test]
    fn opens_below_last_panel_in_order() {
        let p = (10, 165, 365, 200);
        assert_eq!(
            fifo_position(
                p,
                (365, 180),
                &[(10, 10, 365, 155), p],
                Some((0, 0, 1920, 1080))
            ),
            (10, 365)
        );
    }
    #[test]
    fn uses_side_if_below_would_be_offscreen() {
        let p = (10, 800, 365, 200);
        let q = fifo_position(p, (365, 180), &[p], Some((0, 0, 1920, 1080)));
        assert_eq!(q, (375, 800));
    }
    #[test]
    fn snaps_to_right_edge() {
        assert_eq!(
            snap_window(469, 103, 365, 180, &[(100, 100, 365, 155)], 12),
            (465, 100)
        );
    }
}

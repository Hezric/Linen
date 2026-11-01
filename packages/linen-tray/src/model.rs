pub type Rect = [i32; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Debug)]
pub struct Lifecycle {
    pub phase: Phase,
    pub focus_hide: Option<u64>,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self {
            phase: Phase::Closed,
            focus_hide: None,
        }
    }
}

impl Lifecycle {
    pub fn present(&mut self) -> bool {
        if self.phase != Phase::Opening {
            return false;
        }
        self.phase = Phase::Open;
        true
    }
    pub fn toggle(&mut self, now: u64) -> Phase {
        if matches!(self.phase, Phase::Opening | Phase::Open) {
            self.phase = Phase::Closing;
        } else if self.phase == Phase::Closing {
            // Consume this click while the focus-loss exit animation is still running.
            self.focus_hide = None;
        } else if self.phase == Phase::Closed {
            if self
                .focus_hide
                .take()
                .is_some_and(|hidden| now.saturating_sub(hidden) < 300)
            {
                return Phase::Closed;
            }
            self.phase = Phase::Opening;
        }
        self.phase
    }
    pub fn dismiss(&mut self, now: u64, focus: bool) {
        if self.phase != Phase::Closed {
            self.phase = Phase::Closing;
            if focus {
                self.focus_hide = Some(now);
            }
        }
    }
}

pub fn taskbar_small(current: Option<u32>, legacy: Option<u32>) -> bool {
    current.map_or(legacy == Some(0), |value| value == 1)
}

pub fn icon_index(scale: f64) -> usize {
    [1.125, 1.375, 1.75, 2.5]
        .iter()
        .position(|limit| scale < *limit)
        .unwrap_or(4)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub bounds: Rect,
    pub slide: [i32; 2],
    pub overflow: bool,
}

// The reported work area can lag a taskbar resize in either direction.
pub fn dock_work_area(full: Rect, mut work: Rect, taskbar: Rect, edge: Edge) -> Rect {
    let (axis, value) = match edge {
        Edge::Top => (1, taskbar[3]),
        Edge::Right => (2, taskbar[0]),
        Edge::Bottom => (3, taskbar[1]),
        Edge::Left => (0, taskbar[2]),
    };
    if taskbar[0] < full[2]
        && taskbar[2] > full[0]
        && taskbar[1] < full[3]
        && taskbar[3] > full[1]
        && taskbar[0] < taskbar[2]
        && taskbar[1] < taskbar[3]
    {
        work[axis] = value.clamp(full[axis % 2], full[axis % 2 + 2]);
    }
    work
}

pub fn placement(
    work: Rect,
    taskbar: Rect,
    icon: Option<Rect>,
    _edge: Edge,
    _rtl: bool,
    scale: f64,
    height: f64,
) -> Placement {
    let gap = (8.0 * scale).round() as i32;
    let width = (360.0 * scale)
        .round()
        .min((work[2] - work[0] - 2 * gap).max(1) as f64) as i32;
    let height = (height * scale)
        .round()
        .clamp(1.0, (work[3] - work[1] - 2 * gap).max(1) as f64) as i32;
    let overflow = icon.is_none_or(|r| {
        r[0] < taskbar[0]
            || r[1] < taskbar[1]
            || r[2] > taskbar[2]
            || r[3] > taskbar[3]
            || r[2] <= r[0]
            || r[3] <= r[1]
    });
    let x = (work[2] - width - gap).max(work[0] + gap);
    let y = (work[3] - height - gap).max(work[1] + gap);
    let slide = [((width + gap) as f64 / scale).round() as i32, 0];
    Placement {
        bounds: [x, y, x + width, y + height],
        slide,
        overflow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_tray_location_uses_the_work_area_bottom_right_and_enters_from_the_right() {
        let work = [-1920, 0, 0, 1040];
        for (scale, right, bottom, width, slide) in [
            (1.0, -8, 1032, 360, 368),
            (1.25, -10, 1030, 450, 368),
            (1.5, -12, 1028, 540, 368),
        ] {
            for height in [200.0, 400.0, 2000.0] {
                for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
                    for icon in [
                        Some([-970, 1040, -950, 1080]),
                        Some([-300, 800, -280, 820]),
                        None,
                    ] {
                        for rtl in [false, true] {
                            let p = placement(
                                work,
                                [-1920, 1040, 0, 1080],
                                icon,
                                edge,
                                rtl,
                                scale,
                                height,
                            );
                            assert_eq!(p.bounds[2], right);
                            assert_eq!(p.bounds[3], bottom);
                            assert_eq!(p.bounds[2] - p.bounds[0], width);
                            assert!(p.bounds[1] >= work[1]);
                            assert_eq!(p.slide, [slide, 0]);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn current_taskbar_edges_replace_stale_work_area_reservations() {
        let full = [-1920, -1080, 0, 0];
        for (edge, work, bar, expected) in [
            (
                Edge::Bottom,
                [-1920, -1080, 0, -48],
                [-1920, -32, 0, 0],
                [-1920, -1080, 0, -32],
            ),
            (
                Edge::Bottom,
                [-1920, -1080, 0, -32],
                [-1920, -72, 0, 0],
                [-1920, -1080, 0, -72],
            ),
            (
                Edge::Top,
                [-1920, -1032, 0, 0],
                [-1920, -1080, 0, -1048],
                [-1920, -1048, 0, 0],
            ),
            (
                Edge::Right,
                [-1920, -1080, -48, 0],
                [-32, -1080, 0, 0],
                [-1920, -1080, -32, 0],
            ),
            (
                Edge::Left,
                [-1872, -1080, 0, 0],
                [-1920, -1080, -1888, 0],
                [-1888, -1080, 0, 0],
            ),
        ] {
            let resolved = dock_work_area(full, work, bar, edge);
            assert_eq!(resolved, expected);
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let p = placement(resolved, bar, None, edge, false, scale, 400.0);
                assert_eq!(p.bounds[2], expected[2] - (8.0 * scale).round() as i32);
                assert_eq!(p.bounds[3], expected[3] - (8.0 * scale).round() as i32);
            }
        }
        let work = [-1920, -1080, 0, -48];
        assert_eq!(dock_work_area(full, work, [0, 0, 1920, 48], Edge::Bottom), work);
        assert_eq!(dock_work_area(full, work, [0, 0, 0, 0], Edge::Bottom), work);
    }
    #[test]
    fn cancelled_presentation_cannot_reopen_a_closing_flyout() {
        let mut state = Lifecycle::default();
        state.toggle(0);
        state.dismiss(1, true);
        assert!(!state.present());
        assert_eq!(state.phase, Phase::Closing);
        assert_eq!(state.focus_hide, Some(1));
    }

    #[test]
    fn closing_tray_click_is_consumed_before_the_next_open() {
        let mut state = Lifecycle::default();
        state.toggle(0);
        state.present();
        state.dismiss(1000, true);
        assert_eq!(state.toggle(1050), Phase::Closing);
        state.phase = Phase::Closed;
        assert_eq!(state.toggle(1250), Phase::Opening);
    }
    #[test]
    fn tray_click_after_focus_loss_closes_instead_of_reopening() {
        let mut state = Lifecycle::default();
        assert_eq!(state.toggle(0), Phase::Opening);
        state.phase = Phase::Open;
        state.dismiss(1000, true);
        state.phase = Phase::Closed;
        assert_eq!(state.toggle(1050), Phase::Closed);
        assert_eq!(state.toggle(1500), Phase::Opening);
        state.phase = Phase::Open;
        assert_eq!(state.toggle(2000), Phase::Closing);
        assert_eq!(state.toggle(2010), Phase::Closing);
    }

    #[test]
    fn overflow_docks_at_the_notification_corner_and_reverses_its_entry() {
        let work = [0, 0, 1920, 1040];
        let bar = [0, 1040, 1920, 1080];
        for icon in [Some([1650, 850, 1670, 870]), None] {
            let p = placement(work, bar, icon, Edge::Bottom, false, 1.0, 400.0);
            assert!(p.overflow);
            assert_eq!(p.bounds, [1552, 632, 1912, 1032]);
            assert_eq!(p.slide, [368, 0]);
        }
        let rtl = placement(work, bar, None, Edge::Bottom, true, 1.0, 400.0);
        assert_eq!(rtl.bounds, [1552, 632, 1912, 1032]);
        assert_eq!(rtl.slide, [368, 0]);
    }

    #[test]
    fn small_work_areas_keep_the_panel_visible() {
        let p = placement(
            [0, 0, 280, 300],
            [0, 300, 280, 340],
            None,
            Edge::Bottom,
            false,
            1.0,
            2000.0,
        );
        assert_eq!(p.bounds, [8, 8, 272, 292]);
        assert_eq!(p.slide, [272, 0]);
    }
    #[test]
    fn tray_mode_and_dpi_selection_follow_shell_values() {
        for (current, legacy, small) in [
            (Some(0), Some(0), false),
            (Some(1), None, true),
            (Some(7), Some(0), false),
            (None, Some(0), true),
            (None, Some(1), false),
            (None, None, false),
        ] {
            assert_eq!(taskbar_small(current, legacy), small);
        }
        for (scale, index) in [
            (1.0, 0),
            (1.125, 1),
            (1.25, 1),
            (1.5, 2),
            (2.0, 3),
            (3.0, 4),
        ] {
            assert_eq!(icon_index(scale), index);
        }
    }

    #[test]
    fn dismissal_is_idempotent_and_explicit_dismiss_does_not_suppress_a_new_open() {
        let mut state = Lifecycle::default();
        state.toggle(0);
        state.dismiss(1, false);
        state.dismiss(2, false);
        assert_eq!(state.phase, Phase::Closing);
        state.phase = Phase::Closed;
        assert_eq!(state.toggle(3), Phase::Opening);
    }
}

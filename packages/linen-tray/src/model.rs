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
    pub fn toggle(&mut self, now: u64) -> Phase {
        if matches!(self.phase, Phase::Opening | Phase::Open) {
            self.phase = Phase::Closing;
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

pub fn placement(
    work: Rect,
    taskbar: Rect,
    icon: Option<Rect>,
    edge: Edge,
    rtl: bool,
    scale: f64,
    height: f64,
) -> Placement {
    let gap = (12.0 * scale).round() as i32;
    let width = (360.0 * scale)
        .round()
        .min((work[2] - work[0] - 2 * gap).max(1) as f64) as i32;
    let height = (height * scale)
        .round()
        .clamp(1.0, (work[3] - work[1] - 2 * gap).max(1) as f64) as i32;
    let anchor = icon.unwrap_or([work[2], work[3], work[2], work[3]]);
    let overflow = icon.is_none_or(|r| {
        r[0] < taskbar[0]
            || r[1] < taskbar[1]
            || r[2] > taskbar[2]
            || r[3] > taskbar[3]
            || r[2] <= r[0]
            || r[3] <= r[1]
    });
    let mut x = (anchor[0] + anchor[2] - width) / 2;
    let mut y = anchor[3] - height;
    match edge {
        Edge::Top => y = work[1] + gap,
        Edge::Right => x = work[2] - width - gap,
        Edge::Bottom => y = work[3] - height - gap,
        Edge::Left => x = work[0] + gap,
    }
    let mut slide = match edge {
        Edge::Top => [0, -12],
        Edge::Right => [12, 0],
        Edge::Bottom => [0, 12],
        Edge::Left => [-12, 0],
    };
    if overflow {
        match edge {
            Edge::Top | Edge::Bottom => {
                x = if rtl {
                    work[0] + gap
                } else {
                    work[2] - width - gap
                };
                let distance = ((width + gap) as f64 / scale).round() as i32;
                slide = [if rtl { -distance } else { distance }, 0];
            }
            Edge::Left | Edge::Right => {
                y = if rtl {
                    work[1] + gap
                } else {
                    work[3] - height - gap
                };
                let distance = ((width + gap) as f64 / scale).round() as i32;
                slide = [
                    if edge == Edge::Left {
                        -distance
                    } else {
                        distance
                    },
                    0,
                ];
            }
        }
    }
    x = x.clamp(work[0] + gap, (work[2] - width - gap).max(work[0] + gap));
    y = y.clamp(work[1] + gap, (work[3] - height - gap).max(work[1] + gap));
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
            assert_eq!(p.bounds, [1548, 628, 1908, 1028]);
            assert_eq!(p.slide, [372, 0]);
        }
        let rtl = placement(work, bar, None, Edge::Bottom, true, 1.0, 400.0);
        assert_eq!(rtl.bounds, [12, 628, 372, 1028]);
        assert_eq!(rtl.slide, [-372, 0]);
    }

    #[test]
    fn taskbar_placement_centers_clamps_and_preserves_scaled_anchor() {
        for scale in [1.0, 1.5] {
            let work = [-1920, 0, 0, 1040];
            let bar = [-1920, 1040, 0, 1080];
            let p = placement(
                work,
                bar,
                Some([-970, 1040, -950, 1080]),
                Edge::Bottom,
                false,
                scale,
                400.0,
            );
            assert!(!p.overflow);
            assert_eq!((p.bounds[0] + p.bounds[2]) / 2, -960);
            assert_eq!(p.bounds[3], 1040 - (12.0 * scale) as i32);
            for icon in [[-1920, 1040, -1900, 1080], [-20, 1040, 0, 1080]] {
                let p = placement(work, bar, Some(icon), Edge::Bottom, false, scale, 2000.0);
                assert!(p.bounds[0] >= work[0] && p.bounds[2] <= work[2]);
                assert!(p.bounds[1] >= work[1] && p.bounds[3] <= work[3]);
            }
        }
        let work = [0, 0, 1600, 1000];
        for (bar, edge, slide) in [
            ([0, -40, 1600, 0], Edge::Top, [0, -12]),
            ([1600, 0, 1640, 1000], Edge::Right, [12, 0]),
            ([-40, 0, 0, 1000], Edge::Left, [-12, 0]),
        ] {
            let icon = match edge {
                Edge::Top => [500, -30, 520, -10],
                Edge::Right => [1610, 500, 1630, 520],
                _ => [-30, 500, -10, 520],
            };
            let p = placement(work, bar, Some(icon), edge, false, 1.0, 400.0);
            assert!(!p.overflow);
            assert_eq!(p.slide, slide);
            let p = placement(work, bar, None, edge, false, 1.0, 400.0);
            assert!(p.overflow);
            assert_eq!(p.bounds[3], if edge == Edge::Top { 412 } else { 988 });
        }
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

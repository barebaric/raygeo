use super::Ops;
use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::get_arc_length;
use crate::geo::shape::bezier::get_bezier_length;
use crate::geo::shape::line::get_line_segment_length;
use crate::geo::types::{Point, Point3D};
use crate::ops::types::{MoveCmd, OpCategory, OpNode, StateCmd};

impl Ops {
    pub fn invalidate_time_cache(&mut self) {
        self.time_dirty = true;
    }

    /// Total simulated execution time (seconds).
    ///
    /// Computed in a single streaming pass over the commands without
    /// building or caching a cumulative index, so it is suitable for
    /// one-off estimates where the index would only be discarded.
    /// Identical to the last entry of
    /// :meth:`build_cumulative_time_index`.
    pub fn estimate_time(
        &self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> f64 {
        if self.commands.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        let mut last_point = Point3D::new(0.0, 0.0, 0.0);
        let mut feed_rate = default_feed_rate;
        let mut rapid_rate = default_rapid_rate;
        for node in self.commands.iter() {
            total += command_duration(
                node,
                &mut last_point,
                &mut feed_rate,
                &mut rapid_rate,
                acceleration,
            );
        }
        total
    }

    /// Estimated execution time (seconds) of each individual command.
    ///
    /// Returns one entry per command. Moving commands (MoveTo, LineTo,
    /// ArcTo, etc.) yield their estimated execution time in seconds.
    /// Dwell commands yield their dwell duration in seconds. Other
    /// non-moving commands (state changes, markers) yield 0.0.
    pub fn estimate_command_times(
        &self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> Vec<f64> {
        let mut times = Vec::with_capacity(self.commands.len());
        let mut last_point = Point3D::new(0.0, 0.0, 0.0);
        let mut feed_rate = default_feed_rate;
        let mut rapid_rate = default_rapid_rate;

        for node in self.commands.iter() {
            let cmd_time = command_duration(
                node,
                &mut last_point,
                &mut feed_rate,
                &mut rapid_rate,
                acceleration,
            );
            times.push(cmd_time);
        }
        times
    }

    /// Cumulative execution time (seconds) of every command.
    ///
    /// Returns a slice with one entry per command, where entry *i* is
    /// the total simulated time elapsed once command *i* has executed.
    /// State changes (except dwells) and markers contribute zero time.
    /// The result is cached per parameter set and invalidated when the
    /// ops are mutated.
    pub fn build_cumulative_time_index(
        &mut self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> &[f64] {
        let params = (default_feed_rate, default_rapid_rate, acceleration);
        if self.time_dirty
            || self.time_params != Some(params)
            || self.cached_time_index.len() != self.commands.len()
        {
            self.cached_time_index = compute_cumulative_time_index(
                self,
                default_feed_rate,
                default_rapid_rate,
                acceleration,
            );
            self.cached_time =
                self.cached_time_index.last().copied().unwrap_or(0.0);
            self.time_dirty = false;
            self.time_params = Some(params);
        }
        &self.cached_time_index
    }

    /// Find the command index in effect at simulated time *t* (seconds).
    ///
    /// Returns the largest index whose cumulative execution time is
    /// <= *t*, clamped to ``[0, len-1]``. Returns 0 for an empty ops
    /// or for times before the first command's completion.
    pub fn find_index_at_time(
        &mut self,
        t: f64,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> usize {
        let index = self.build_cumulative_time_index(
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        );
        if index.is_empty() {
            return 0;
        }
        let pos = index.partition_point(|&c| c <= t);
        pos.saturating_sub(1).min(index.len() - 1)
    }

    /// Cumulative simulated time (seconds) up to and including command *idx*.
    ///
    /// Out-of-range indices clamp to the nearest valid command; empty
    /// ops yield 0.0.
    pub fn get_cumulative_time_at(
        &mut self,
        idx: usize,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> f64 {
        let index = self.build_cumulative_time_index(
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        );
        match index.get(idx) {
            Some(&v) => v,
            None => index.last().copied().unwrap_or(0.0),
        }
    }
}

fn command_duration(
    node: &OpNode,
    last_point: &mut Point3D,
    feed_rate: &mut f64,
    rapid_rate: &mut f64,
    acceleration: f64,
) -> f64 {
    match &node.category {
        OpCategory::State(StateCmd::SetFeedRate(s)) => {
            *feed_rate = *s as f64;
            0.0
        }
        OpCategory::State(StateCmd::SetRapidRate(s)) => {
            *rapid_rate = *s as f64;
            0.0
        }
        OpCategory::State(StateCmd::Dwell(ms)) => *ms / 1000.0,
        OpCategory::Moving { end, cmd } => {
            let distance = move_distance(cmd, *last_point, *end);

            if distance < EPSILON_COLLINEAR {
                *last_point = *end;
                0.0
            } else {
                let speed = if matches!(cmd, MoveCmd::MoveTo) {
                    *rapid_rate
                } else {
                    *feed_rate
                };

                let speed_mm_per_sec = speed / 60.0;
                let move_time = if acceleration > 0.0 {
                    let accel_time = speed_mm_per_sec / acceleration;
                    let accel_distance =
                        0.5 * acceleration * accel_time * accel_time;
                    if distance < 2.0 * accel_distance {
                        2.0 * (distance / acceleration).sqrt()
                    } else {
                        let cruise_distance = distance - 2.0 * accel_distance;
                        let cruise_time = cruise_distance / speed_mm_per_sec;
                        2.0 * accel_time + cruise_time
                    }
                } else {
                    distance / speed_mm_per_sec
                };

                *last_point = *end;
                move_time
            }
        }
        _ => 0.0,
    }
}

fn move_distance(cmd: &MoveCmd, last_point: Point3D, end: Point3D) -> f64 {
    match cmd {
        MoveCmd::ArcTo(data) => get_arc_length(
            Point::new(last_point.x, last_point.y),
            Point::new(end.x, end.y),
            data.center,
            data.cw,
        ),
        MoveCmd::BezierTo(data) => get_bezier_length(
            Point::new(last_point.x, last_point.y),
            Point::new(data.control1.x, data.control1.y),
            Point::new(data.control2.x, data.control2.y),
            Point::new(end.x, end.y),
        ),
        MoveCmd::QuadraticBezierTo { control } => {
            let c = *control;
            get_bezier_length(
                Point::new(last_point.x, last_point.y),
                Point::new(
                    (last_point.x + 2.0 * c.x) / 3.0,
                    (last_point.y + 2.0 * c.y) / 3.0,
                ),
                Point::new(
                    (end.x + 2.0 * c.x) / 3.0,
                    (end.y + 2.0 * c.y) / 3.0,
                ),
                Point::new(end.x, end.y),
            )
        }
        _ => get_line_segment_length(
            Point::new(last_point.x, last_point.y),
            Point::new(end.x, end.y),
        ),
    }
}

fn compute_cumulative_time_index(
    ops: &Ops,
    default_feed_rate: f64,
    default_rapid_rate: f64,
    acceleration: f64,
) -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(ops.commands.len());
    let mut acc = 0.0;
    let mut last_point = Point3D::new(0.0, 0.0, 0.0);
    let mut feed_rate = default_feed_rate;
    let mut rapid_rate = default_rapid_rate;

    for node in ops.commands.iter() {
        acc += command_duration(
            node,
            &mut last_point,
            &mut feed_rate,
            &mut rapid_rate,
            acceleration,
        );
        cumulative.push(acc);
    }
    cumulative
}

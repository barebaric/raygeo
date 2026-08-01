use super::Ops;
use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::get_arc_length;
use crate::geo::shape::bezier::get_bezier_length;
use crate::geo::shape::line::get_line_segment_length;
use crate::geo::types::{Point, Point3D};
use crate::ops::types::{MoveCmd, OpCategory, StateCmd};

impl Ops {
    pub fn invalidate_time_cache(&mut self) {
        self.time_dirty = true;
    }

    pub fn estimate_time(
        &mut self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> f64 {
        if self.commands.is_empty() {
            return 0.0;
        }
        let params = (default_feed_rate, default_rapid_rate, acceleration);
        if !self.time_dirty && self.time_params == Some(params) {
            return self.cached_time;
        }
        let total = estimate_time_core(
            self,
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        );
        self.cached_time = total;
        self.time_dirty = false;
        self.time_params = Some(params);
        total
    }

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

        for node in &self.commands {
            let cmd_time = match &node.category {
                OpCategory::State(StateCmd::SetFeedRate(s)) => {
                    feed_rate = *s as f64;
                    0.0
                }
                OpCategory::State(StateCmd::SetRapidRate(s)) => {
                    rapid_rate = *s as f64;
                    0.0
                }
                OpCategory::Moving { end, cmd } => {
                    let distance = move_distance(cmd, last_point, *end);

                    if distance < EPSILON_COLLINEAR {
                        last_point = *end;
                        0.0
                    } else {
                        let speed = if matches!(cmd, MoveCmd::MoveTo) {
                            rapid_rate
                        } else {
                            feed_rate
                        };

                        let speed_mm_per_sec = speed / 60.0;
                        let move_time = if acceleration > 0.0 {
                            let accel_time = speed_mm_per_sec / acceleration;
                            let accel_distance =
                                0.5 * acceleration * accel_time * accel_time;
                            if distance < 2.0 * accel_distance {
                                2.0 * (distance / acceleration).sqrt()
                            } else {
                                let cruise_distance =
                                    distance - 2.0 * accel_distance;
                                let cruise_time =
                                    cruise_distance / speed_mm_per_sec;
                                2.0 * accel_time + cruise_time
                            }
                        } else {
                            distance / speed_mm_per_sec
                        };

                        last_point = *end;
                        move_time
                    }
                }
                _ => 0.0,
            };
            times.push(cmd_time);
        }
        times
    }
}

fn move_distance(cmd: &MoveCmd, last_point: Point3D, end: Point3D) -> f64 {
    match cmd {
        MoveCmd::ArcTo { center, cw } => get_arc_length(
            Point::new(last_point.x, last_point.y),
            Point::new(end.x, end.y),
            *center,
            *cw,
        ),
        MoveCmd::BezierTo { control1, control2 } => get_bezier_length(
            Point::new(last_point.x, last_point.y),
            Point::new(control1.x, control1.y),
            Point::new(control2.x, control2.y),
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

fn estimate_time_core(
    ops: &Ops,
    default_feed_rate: f64,
    default_rapid_rate: f64,
    acceleration: f64,
) -> f64 {
    ops.estimate_command_times(
        default_feed_rate,
        default_rapid_rate,
        acceleration,
    )
    .into_iter()
    .sum()
}

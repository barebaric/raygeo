use crate::ops::Ops;

impl Ops {
    pub fn get_frame(
        &self,
        power: Option<f64>,
        feed_rate: Option<f64>,
    ) -> Self {
        let Some(rect) = self.rect(false) else {
            return Ops::new();
        };
        let (min_x, min_y, max_x, max_y) =
            (rect.min.x, rect.min.y, rect.max.x, rect.max.y);
        let mut frame_ops = Ops::new();
        if let Some(p) = power {
            frame_ops.set_power(p);
        }
        if let Some(f) = feed_rate {
            frame_ops.set_feed_rate(f as i32);
        }
        frame_ops.move_to(min_x, min_y, 0.0, None);
        frame_ops.line_to(min_x, max_y, 0.0, None);
        frame_ops.line_to(max_x, max_y, 0.0, None);
        frame_ops.line_to(max_x, min_y, 0.0, None);
        frame_ops.line_to(min_x, min_y, 0.0, None);
        frame_ops
    }
}

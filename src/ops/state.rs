#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CoolantMode {
    #[default]
    Off,
    Flood,
    Mist,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AirAssistMode {
    #[default]
    Off,
    On,
}

/// Laser power mode for subsequent cutting commands.
///
/// ``Dynamic`` lets the controller scale power with head speed (M4 on
/// Grbl-like controllers); ``Constant`` holds the power fixed
/// regardless of speed (M3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PowerMode {
    /// Power scales with head speed (M4).
    #[default]
    Dynamic,
    /// Power stays fixed regardless of head speed (M3).
    Constant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HeadCoolantMode {
    #[default]
    Off,
    On,
}

/// Spindle rotational direction.
///
/// `Cw` corresponds to M3, `Ccw` to M4. Rigid tapping reverses the
/// spindle at the bottom of the hole so the synchronized retract
/// unscrews the tap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpindleDirection {
    /// Clockwise (M3).
    #[default]
    Cw,
    /// Counter-clockwise (M4).
    Ccw,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub power: f64,
    pub feed_rate: Option<i32>,
    /// Feed for vertical plunges into material; when `None`, cutting
    /// systems fall back to [`Self::feed_rate`].
    pub plunge_feed: Option<i32>,
    pub rapid_rate: Option<i32>,
    pub active_head_uid: Option<String>,
    pub frequency: Option<i32>,
    pub pulse_width: Option<f64>,
    pub dwell_ms: Option<f64>,
    pub spindle_rpm: Option<u32>,
    /// Rotational direction of the spindle; `None` behaves as `Cw`.
    pub spindle_direction: Option<SpindleDirection>,
    /// Axial travel (mm) per spindle revolution for subsequent feed
    /// moves. When `Some`, feed moves are spindle-synchronized (rigid
    /// tapping): their effective feed is `sync_pitch × spindle_rpm`
    /// mm/min. `None` = normal feed-per-minute motion.
    pub sync_pitch: Option<f64>,
    pub coolant: Option<CoolantMode>,
    pub air_assist: Option<AirAssistMode>,
    pub head_coolant: Option<HeadCoolantMode>,
    pub power_mode: Option<PowerMode>,
}

impl State {
    pub fn allow_rapid_change(&self, target: &State) -> bool {
        self.coolant == target.coolant
            && self.air_assist == target.air_assist
            && self.head_coolant == target.head_coolant
    }
}

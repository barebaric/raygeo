#[derive(Clone, Debug, Default)]
pub struct State {
    pub power: f64,
    pub air_assist: bool,
    pub cut_speed: Option<i32>,
    pub travel_speed: Option<i32>,
    pub active_laser_uid: Option<String>,
    pub frequency: Option<i32>,
    pub pulse_width: Option<f64>,
    pub dwell_ms: Option<f64>,
}

impl State {
    pub fn allow_rapid_change(&self, target: &State) -> bool {
        self.air_assist == target.air_assist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_default() {
        let s = State::default();
        assert_eq!(s.power, 0.0);
        assert!(!s.air_assist);
        assert!(s.cut_speed.is_none());
        assert!(s.travel_speed.is_none());
        assert!(s.active_laser_uid.is_none());
        assert!(s.frequency.is_none());
        assert!(s.pulse_width.is_none());
    }

    #[test]
    fn test_allow_rapid_change_same_air_assist() {
        let a = State {
            air_assist: true,
            ..Default::default()
        };
        let b = State {
            air_assist: true,
            power: 50.0,
            ..Default::default()
        };
        assert!(a.allow_rapid_change(&b));
    }

    #[test]
    fn test_allow_rapid_change_different_air_assist() {
        let a = State {
            air_assist: true,
            ..Default::default()
        };
        let b = State {
            air_assist: false,
            ..Default::default()
        };
        assert!(!a.allow_rapid_change(&b));
    }
}

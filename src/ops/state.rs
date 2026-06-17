#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CoolantMode {
    #[default]
    Off,
    Flood,
    Mist,
    Air,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub power: f64,
    pub feed_rate: Option<i32>,
    pub rapid_rate: Option<i32>,
    pub active_head_uid: Option<String>,
    pub frequency: Option<i32>,
    pub pulse_width: Option<f64>,
    pub dwell_ms: Option<f64>,
    pub spindle_rpm: Option<u32>,
    pub coolant: Option<CoolantMode>,
}

impl State {
    pub fn allow_rapid_change(&self, target: &State) -> bool {
        self.coolant == target.coolant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_default() {
        let s = State::default();
        assert_eq!(s.power, 0.0);
        assert!(s.feed_rate.is_none());
        assert!(s.rapid_rate.is_none());
        assert!(s.active_head_uid.is_none());
        assert!(s.frequency.is_none());
        assert!(s.pulse_width.is_none());
    }

    #[test]
    fn test_allow_rapid_change_same_coolant() {
        let a = State {
            coolant: Some(CoolantMode::Air),
            ..Default::default()
        };
        let b = State {
            coolant: Some(CoolantMode::Air),
            power: 50.0,
            ..Default::default()
        };
        assert!(a.allow_rapid_change(&b));
    }

    #[test]
    fn test_allow_rapid_change_different_coolant() {
        let a = State {
            coolant: Some(CoolantMode::Air),
            ..Default::default()
        };
        let b = State {
            coolant: Some(CoolantMode::Flood),
            ..Default::default()
        };
        assert!(!a.allow_rapid_change(&b));
    }
}

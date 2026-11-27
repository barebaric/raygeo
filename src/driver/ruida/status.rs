//! Decoding of Ruida ``GET_SETTING`` replies into machine status.

use crate::driver::core::types::DeviceStatus;
use crate::driver::ruida::codec::{self, Reply};

/// The machine state derived from the controller's status replies.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MachineStatus {
    pub running: bool,
    pub paused: bool,
    pub moving: bool,
    /// X, Y, Z and U positions in millimetres.
    pub position: [Option<f64>; 4],
    pub card_id: Option<u32>,
    /// Bed size in millimetres.
    pub bed_size: [Option<f64>; 2],
    pub mainboard_version: Option<String>,
    pub features: Option<u32>,
}

impl MachineStatus {
    /// Applies one reply. Returns whether anything changed.
    pub fn apply(&mut self, reply: &Reply) -> bool {
        let before = self.clone();
        match reply.address {
            0x0400 => {
                let value = codec::to_uint(&reply.data) as u32;
                self.running = value & 0x0000_0001 != 0;
                self.paused = value & 0x0000_0002 != 0;
                self.moving = value & 0x0100_0000 != 0;
            }
            0x0421 => self.position[0] = decode_position(&reply.data),
            0x0431 => self.position[1] = decode_position(&reply.data),
            0x0441 => self.position[2] = decode_position(&reply.data),
            0x0451 => self.position[3] = decode_position(&reply.data),
            0x057E => self.card_id = Some(codec::to_uint(&reply.data) as u32),
            0x057F => self.mainboard_version = Some(decode_string(&reply.data)),
            0x0026 => self.bed_size[0] = decode_position(&reply.data),
            0x0036 => self.bed_size[1] = decode_position(&reply.data),
            0x030F => self.features = Some(codec::to_uint(&reply.data) as u32),
            _ => return false,
        }
        *self != before
    }

    /// The device status the UI should show.
    pub fn device_status(&self) -> DeviceStatus {
        if self.paused {
            DeviceStatus::Hold
        } else if self.running {
            DeviceStatus::Run
        } else {
            DeviceStatus::Idle
        }
    }
}

fn decode_position(data: &[u8]) -> Option<f64> {
    (!data.is_empty()).then(|| codec::to_int(data) as f64 / 1000.0)
}

fn decode_string(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

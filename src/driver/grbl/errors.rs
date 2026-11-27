//! Grbl error and alarm code tables.
//!
//! Source: <https://github.com/gnea/grbl/wiki/Grbl-v1.1-Interface#message-summary>

use crate::driver::core::types::DeviceError;

pub fn error_code_to_device_error(error_code: &str) -> DeviceError {
    match error_code.parse::<i32>() {
        Err(_) => DeviceError::new(
            -1,
            "Unknown Error",
            "Invalid error code reported by machine.",
        ),
        Ok(code) => grbl_error_for_code(code).unwrap_or(DeviceError::new(
            code,
            "Unknown Error",
            "The machine reported an unrecognized error code. Check your machine and firmware documentation.",
        )),
    }
}

pub fn alarm_code_to_device_error(alarm_code: &str) -> DeviceError {
    match alarm_code.parse::<i32>() {
        Err(_) => DeviceError::new(
            -1,
            "Unknown Alarm",
            "Invalid alarm code reported by machine.",
        ),
        Ok(code) => grbl_alarm_for_code(code).unwrap_or(DeviceError::new(
            code,
            "Unknown Alarm",
            "The machine reported an unrecognized alarm code. Check your machine and firmware documentation.",
        )),
    }
}

fn grbl_error_for_code(code: i32) -> Option<DeviceError> {
    let e = match code {
        1 => DeviceError::new(
            1,
            "Missing Command Letter",
            "G-code commands need a letter followed by a value. The command letter was not found.",
        ),
        2 => DeviceError::new(
            2,
            "Invalid Number Format",
            "The value is missing or not in the correct numeric format. Check your G-code syntax.",
        ),
        3 => DeviceError::new(
            3,
            "Unknown Command",
            "This Grbl setting command is not recognized or supported. Check the command syntax.",
        ),
        4 => DeviceError::new(
            4,
            "Negative Value",
            "A positive number is required here, but a negative value was received.",
        ),
        5 => DeviceError::new(
            5,
            "Homing Disabled",
            "Homing is not enabled in settings. Enable homing ($22=1) to use this feature.",
        ),
        6 => DeviceError::new(
            6,
            "Pulse Time Too Short",
            "Minimum step pulse time must be greater than 3 microseconds. Check setting $0.",
        ),
        7 => DeviceError::new(
            7,
            "Memory Error",
            "Settings reset to defaults due to a memory read failure. Reconfigure your settings if needed.",
        ),
        8 => DeviceError::new(
            8,
            "Machine Busy",
            "This command can only be used when the machine is idle. Wait for the current job to finish.",
        ),
        9 => DeviceError::new(
            9,
            "Commands Locked",
            "Cannot send commands while in alarm or jog mode. Clear the alarm state first.",
        ),
        10 => DeviceError::new(
            10,
            "Homing Required",
            "Soft limits cannot be enabled without homing also enabled. Enable homing first ($22=1).",
        ),
        11 => DeviceError::new(
            11,
            "Line Too Long",
            "The command line has too many characters and was ignored. Check your file formatting.",
        ),
        12 => DeviceError::new(
            12,
            "Setting Too High",
            "This setting exceeds the maximum step rate supported. Use a lower value.",
        ),
        13 => DeviceError::new(
            13,
            "Door Open",
            "The safety door was detected as open. Close the door and resume operation.",
        ),
        14 => DeviceError::new(
            14,
            "Line Too Long",
            "Build info or startup line exceeds storage limit. Shorten the line.",
        ),
        15 => DeviceError::new(
            15,
            "Target Out of Range",
            "Jog target is beyond the machine's travel limits. Move to a position within range.",
        ),
        16 => DeviceError::new(
            16,
            "Invalid Jog Command",
            "Jog command is missing '=' or contains prohibited G-code. Check the jog syntax.",
        ),
        17 => DeviceError::new(
            17,
            "Laser Mode Error",
            "Laser mode requires PWM output to work. Check your hardware configuration.",
        ),
        18 => DeviceError::new(
            18,
            "Spindle Not Running",
            "A motion command was issued but the spindle is not running. Start the spindle before motion.",
        ),
        19 => DeviceError::new(
            19,
            "Spindle Speed Mismatch",
            "The current spindle speed does not match the speed required by the command. Wait for the spindle to reach the target speed.",
        ),
        20 => DeviceError::new(
            20,
            "Unsupported Command",
            "This G-code command is not supported by the machine. Check your post-processor settings.",
        ),
        21 => DeviceError::new(
            21,
            "Conflicting Commands",
            "Multiple commands from the same group found on one line. Remove the duplicate command.",
        ),
        22 => DeviceError::new(
            22,
            "Feed Rate Missing",
            "Set a feed rate before using motion commands. Add an F command to specify speed.",
        ),
        23 => DeviceError::new(
            23,
            "Integer Required",
            "This command requires a whole number value. Remove any decimal points.",
        ),
        24 => DeviceError::new(
            24,
            "Axis Conflict",
            "Multiple commands trying to use the same axis. Simplify the command.",
        ),
        25 => DeviceError::new(
            25,
            "Duplicate Word",
            "The same G-code word appears more than once. Remove the duplicate.",
        ),
        26 => DeviceError::new(
            26,
            "Missing Axis",
            "This command requires XYZ axis coordinates. Add the missing axis values.",
        ),
        27 => DeviceError::new(
            27,
            "Line Number Out of Range",
            "Line number must be between 1 and 9,999,999. Use a valid line number.",
        ),
        28 => DeviceError::new(
            28,
            "Missing Value",
            "This command requires a P or L value. Add the missing parameter.",
        ),
        29 => DeviceError::new(
            29,
            "Unsupported Coordinate",
            "Only G54-G59 coordinate systems are supported. Use one of these instead.",
        ),
        30 => DeviceError::new(
            30,
            "Wrong Motion Mode",
            "G53 command requires G0 or G1 motion mode. Set the correct motion mode first.",
        ),
        31 => DeviceError::new(
            31,
            "Unused Axis Words",
            "Axis words present but G80 cancel is active. Remove the unused axis words.",
        ),
        32 => DeviceError::new(
            32,
            "Missing Arc Data",
            "G2/G3 arc command needs XYZ coordinates. Add the axis values for the selected plane.",
        ),
        33 => DeviceError::new(
            33,
            "Invalid Target",
            "Cannot create this arc or probe to current position. Check the target coordinates.",
        ),
        34 => DeviceError::new(
            34,
            "Arc Geometry Error",
            "Arc calculation failed. Try breaking the arc into smaller pieces or use IJK offset instead.",
        ),
        35 => DeviceError::new(
            35,
            "Missing Arc Offset",
            "G2/G3 arc command needs IJK offset values. Add the missing offset for the selected plane.",
        ),
        36 => DeviceError::new(
            36,
            "Unused Words",
            "Some G-code words in this line are not used by any command. Remove the unused words.",
        ),
        37 => DeviceError::new(
            37,
            "Wrong Axis for Offset",
            "Tool length offset only works on the configured axis (usually Z-axis). Check your settings.",
        ),
        38 => DeviceError::new(
            38,
            "Tool Number Too High",
            "Tool number exceeds the maximum supported value. Use a valid tool number.",
        ),
        _ => return None,
    };
    Some(e)
}

fn grbl_alarm_for_code(code: i32) -> Option<DeviceError> {
    let e = match code {
        1 => DeviceError::new(
            1,
            "Hard Limit",
            "A hard limit switch was triggered. The machine has stopped and needs to be reset. Check for obstructions and verify your limit switches.",
        ),
        2 => DeviceError::new(
            2,
            "Soft Limit",
            "The machine would move beyond its configured travel limits. Check that your work area and coordinate offsets are correct.",
        ),
        3 => DeviceError::new(
            3,
            "Abort Cycle",
            "The currently running job was cancelled while in motion. Reset the machine to continue.",
        ),
        4 => DeviceError::new(
            4,
            "Probe Fail — Initial",
            "The probe did not make contact before the maximum travel distance was reached. Check the probe wiring and positioning.",
        ),
        5 => DeviceError::new(
            5,
            "Probe Fail — Final",
            "The probe failed to retract to the target position after contact. Check the probe configuration.",
        ),
        6 => DeviceError::new(
            6,
            "Homing Fail — Reset",
            "Homing was not able to complete because the machine is in an alarm state. Clear the alarm and try again.",
        ),
        7 => DeviceError::new(
            7,
            "Homing Fail — Approach",
            "The homing cycle failed to find the switch within the configured travel distance. Check your switch wiring and pull-off settings.",
        ),
        8 => DeviceError::new(
            8,
            "Homing Fail — Pulloff",
            "The homing cycle failed to successfully pull off the switch after contact. Increase the pull-off distance or check the switch.",
        ),
        9 => DeviceError::new(
            9,
            "Home Without Limits",
            "Homing was commanded but limit switches are not configured. Enable limit switches first.",
        ),
        10 => DeviceError::new(
            10,
            "Homing Fail — Dual Axis",
            "Homing failed on a dual-axis configuration. One or both axes did not reach their limit switches. Check your limit switch wiring and configuration.",
        ),
        _ => return None,
    };
    Some(e)
}

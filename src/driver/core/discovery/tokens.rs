//! Identity tokens for profile matching.
//!
//! Identification strings (USB descriptions, by-id link names,
//! manufacturer strings, device banners) are split into lowercase
//! word tokens. Generic adapter and product words carry no vendor
//! signal and are filtered at match time via [`GENERIC_TOKENS`].

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

use super::ports::SerialPortInfo;
use super::types::DeviceIdentity;

/// Tokens identifying the USB-serial adapter itself: the chip, its
/// driver names, and the generic words USB strings use for it. A
/// device identity built only from these carries no vendor signal.
pub const USB_CHIP_TOKENS: &[&str] = &[
    "usb", "serial", "uart", "bridge", "port", "if00", "cdc", "acm", "modem",
    "ch340", "ch340g", "ch340k", "ch9102", "ch9102f", "cp210x", "cp2101",
    "cp2102", "cp2104", "cp2109", "ftdi", "ft232", "ft232r", "ft231x",
    "ft2232", "prolific", "pl2303", "arduino", "leonardo", "mega2560",
];

/// Generic product words that carry no vendor identity either.
pub const GENERIC_DEVICE_TOKENS: &[&str] =
    &["controller", "device", "machine", "printer", "board"];

/// Corporate-suffix words that appear in manufacturer strings
/// ("Prolific Technology Inc.") but must not mask a genuine brand
/// word next to them. Stripped from profile vendor names before
/// matching, so a vendor like "Atomstack Technology" still matches
/// on "atomstack".
pub const CORPORATE_TOKENS: &[&str] = &[
    "inc",
    "ltd",
    "limited",
    "corp",
    "co",
    "technology",
    "electronics",
    "semiconductor",
    "integrated",
    "circuits",
    "quantities",
    "industry",
    "industrial",
];

/// All tokens that never count as a vendor signal on their own.
pub static GENERIC_TOKENS: LazyLock<BTreeSet<&'static str>> =
    LazyLock::new(|| {
        USB_CHIP_TOKENS
            .iter()
            .chain(GENERIC_DEVICE_TOKENS.iter())
            .copied()
            .collect()
    });

static WORD_SPLIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^0-9a-zA-Z]+").unwrap());

/// Splits arbitrary identification strings (descriptions, by-id link
/// names, manufacturer strings) into lowercase word tokens suitable
/// for containment checks.
pub fn normalize_tokens(texts: &[Option<&str>]) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    for text in texts.iter().flatten() {
        if text.is_empty() {
            continue;
        }
        for word in WORD_SPLIT_RE.split(&text.to_lowercase()) {
            if word.len() >= 3 {
                words.insert(word.to_string());
            }
        }
    }
    words
}

/// Assembles a [`DeviceIdentity`] from serial scan artifacts. A
/// *device_name* extracted from the device's own output (when the
/// recognizer provides one) is the identity banner and contributes
/// matching tokens.
pub fn build_identity(
    firmware: Option<&str>,
    port_info: Option<&SerialPortInfo>,
    device_name: Option<&str>,
) -> DeviceIdentity {
    let tokens = normalize_tokens(&[
        port_info.and_then(|i| i.description.as_deref()),
        port_info.and_then(|i| i.manufacturer.as_deref()),
        device_name,
    ]);
    DeviceIdentity {
        firmware: firmware.map(str::to_string),
        banner: device_name.map(str::to_string),
        usb_vid: port_info.and_then(|i| i.vid),
        usb_pid: port_info.and_then(|i| i.pid),
        tokens,
    }
}

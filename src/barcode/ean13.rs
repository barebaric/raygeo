//! EAN-13 encoding.
//!
//! Twelve digits get their check digit computed; thirteen digits have
//! the supplied check digit verified. The encoder marks the guard-bar
//! columns so the geometry layer can extend them into the printed
//! digits.

use super::{BarcodeError, Encoded, Symbology};

const L_CODES: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001",
    "0101111", "0111011", "0110111", "0001011",
];
const R_CODES: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110",
    "1010000", "1000100", "1001000", "1110100",
];
const G_CODES: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001",
    "0000101", "0010001", "0001001", "0010111",
];
const PARITY: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL",
    "LGLGLG", "LGLGGL", "LGGLGL",
];

/// Encodes 12 digits (adding the check digit) or 13 digits (verifying
/// the supplied check digit) as EAN-13.
pub fn encode(text: &str) -> Result<Encoded, BarcodeError> {
    let digits = text.trim();
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(BarcodeError::Ean13Digits);
    }
    if digits.len() != 12 && digits.len() != 13 {
        return Err(BarcodeError::Ean13Length);
    }

    let expected = check_digit(digits);
    if digits.len() == 13 {
        let actual = digits.as_bytes()[12] - b'0';
        if actual != expected {
            return Err(BarcodeError::Ean13CheckDigit { actual, expected });
        }
    }

    let mut full = String::with_capacity(13);
    full.push_str(&digits[..12]);
    full.push(char::from(b'0' + expected));

    let parity = PARITY[(full.as_bytes()[0] - b'0') as usize];
    let mut bits = String::with_capacity(95);
    bits.push_str("101");
    for (index, byte) in full.as_bytes()[1..7].iter().enumerate() {
        let table = if parity.as_bytes()[index] == b'L' {
            L_CODES
        } else {
            G_CODES
        };
        bits.push_str(table[(byte - b'0') as usize]);
    }
    bits.push_str("01010");
    for byte in &full.as_bytes()[7..] {
        bits.push_str(R_CODES[(byte - b'0') as usize]);
    }
    bits.push_str("101");

    let row = bits.bytes().map(|byte| byte == b'1').collect();
    let guards = (0..3).chain(45..50).chain(92..95).collect();

    Ok(Encoded {
        symbology: Symbology::Ean13,
        modules: vec![row],
        text: full,
        guards,
    })
}

/// The EAN-13 check digit for the first twelve digits.
fn check_digit(digits: &str) -> u8 {
    let total: u32 = digits
        .bytes()
        .take(12)
        .enumerate()
        .map(|(index, byte)| {
            let value = u32::from(byte - b'0');
            if index.is_multiple_of(2) {
                value
            } else {
                value * 3
            }
        })
        .sum();
    ((10 - total % 10) % 10) as u8
}

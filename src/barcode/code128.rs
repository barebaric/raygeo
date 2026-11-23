//! Code 128 encoding with automatic code-set B/C switching.
//!
//! Long digit runs are packed as pairs in code set C; text uses code set
//! B. The encoder switches at run boundaries so symbol values stay
//! compact without requiring the caller to choose a code set.

use super::{BarcodeError, Encoded, Symbology};

const CODE_C: u8 = 99;
const CODE_B: u8 = 100;
const START_B: u8 = 104;
const START_C: u8 = 105;
const STOP: u8 = 106;

/// The 107 Code 128 symbol patterns, in modules (bar/space widths).
const PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213",
    "122312", "132212", "221213", "221312", "231212", "112232", "122132",
    "122231", "113222", "123122", "123221", "223211", "221132", "221231",
    "213212", "223112", "312131", "311222", "321122", "321221", "312212",
    "322112", "322211", "212123", "212321", "232121", "111323", "131123",
    "131321", "112313", "132113", "132311", "211313", "231113", "231311",
    "112133", "112331", "132131", "113123", "113321", "133121", "313121",
    "211331", "231131", "213113", "213311", "213131", "311123", "311321",
    "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214",
    "112412", "122114", "122411", "142112", "142211", "241211", "221114",
    "413111", "241112", "134111", "111242", "121142", "121241", "114212",
    "124112", "124211", "411212", "421112", "421211", "212141", "214121",
    "412121", "111143", "111341", "131141", "114113", "114311", "411113",
    "411311", "113141", "114131", "311141", "411131", "211412", "211214",
    "211232", "2331112",
];

/// Encodes `text` as Code 128, switching between code sets B and C.
///
/// Only ASCII 32 through 127 can be represented.
pub fn encode(text: &str) -> Result<Encoded, BarcodeError> {
    if text.is_empty() {
        return Err(BarcodeError::EmptyContent);
    }
    for character in text.chars() {
        if !(' '..='\u{7f}').contains(&character) {
            return Err(BarcodeError::Code128Character(character));
        }
    }

    let mut values = code_values(text.as_bytes());
    let checksum = values
        .iter()
        .enumerate()
        .skip(1)
        .fold(u32::from(values[0]), |sum, (position, value)| {
            sum + position as u32 * u32::from(*value)
        })
        % 103;
    values.push(checksum as u8);
    values.push(STOP);

    let row = widths_to_modules(&values);
    Ok(Encoded {
        symbology: Symbology::Code128,
        modules: vec![row],
        text: text.to_string(),
        guards: Vec::new(),
    })
}

/// The symbol values before the checksum and stop pattern.
fn code_values(text: &[u8]) -> Vec<u8> {
    let first_run = digit_run(text, 0);
    let mut use_c = first_run >= 4
        || (first_run == text.len()
            && first_run >= 2
            && first_run.is_multiple_of(2));
    let mut values = vec![if use_c { START_C } else { START_B }];

    let mut index = 0;
    while index < text.len() {
        let run = digit_run(text, index);
        if use_c {
            if run >= 2 {
                let value =
                    (text[index] - b'0') * 10 + (text[index + 1] - b'0');
                values.push(value);
                index += 2;
                continue;
            }
            values.push(CODE_B);
            use_c = false;
            continue;
        }

        let at_end = index + run == text.len();
        if run >= 6 || (at_end && run >= 4) {
            if !run.is_multiple_of(2) {
                values.push(b_value(text[index]));
                index += 1;
            }
            values.push(CODE_C);
            use_c = true;
            continue;
        }
        values.push(b_value(text[index]));
        index += 1;
    }
    values
}

/// The length of the digit run starting at `start`.
fn digit_run(text: &[u8], start: usize) -> usize {
    text[start..]
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count()
}

/// Code set B value for an ASCII byte.
fn b_value(byte: u8) -> u8 {
    byte - 32
}

/// Expands symbol values into a row of module booleans.
fn widths_to_modules(values: &[u8]) -> Vec<bool> {
    let mut modules = Vec::new();
    for value in values {
        let mut dark = true;
        for width in PATTERNS[*value as usize].bytes() {
            let width = usize::from(width - b'0');
            modules.extend(std::iter::repeat_n(dark, width));
            dark = !dark;
        }
    }
    modules
}

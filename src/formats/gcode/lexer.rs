//! Lexical analysis for RS274/NGC G-code programs.
//!
//! G-code is line oriented: a block is one line, consisting of words such
//! as `G1` or `X12.5`. Comments use parentheses or a semicolon, whitespace
//! is insignificant outside comments, and commands are case-insensitive.

/// A single word in a G-code block: an uppercase letter and a number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    pub letter: char,
    pub value: f64,
}

/// True when the line consists solely of a `%` program delimiter.
pub fn is_percent(line: &str) -> bool {
    line.trim() == "%"
}

/// True when the line is disabled by the block-delete character.
pub fn is_block_deleted(line: &str) -> bool {
    line.trim_start().starts_with('/')
}

/// True when the line starts an O-word subroutine label.
pub fn is_subroutine_label(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(['O', 'o']) {
        return false;
    }
    trimmed[1..]
        .trim_start()
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit() || ch == '-' || ch == '+')
}

/// Removes parenthetical and semicolon comments from one line.
///
/// Parentheses do not nest in RS274/NGC, and a semicolon inside a
/// parenthetical comment is literal text.
pub fn strip_comments(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_comment = false;
    for ch in line.chars() {
        if in_comment {
            if ch == ')' {
                in_comment = false;
            }
        } else {
            match ch {
                '(' => in_comment = true,
                ';' => break,
                _ => result.push(ch),
            }
        }
    }
    result
}

/// Splits one comment-free line into words.
///
/// Whitespace is significant only as a separator between words, never
/// inside a number: the NIST line format permits `X +0. 12 34` to mean
/// `X+0.1234`. Words with malformed numbers are ignored.
pub fn tokenize(line: &str) -> Vec<Word> {
    let chars: Vec<char> = line.chars().collect();
    let mut words = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let letter = chars[index];
        index += 1;
        if !letter.is_ascii_alphabetic() {
            continue;
        }
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        let mut number = String::new();
        while index < chars.len() {
            let ch = chars[index];
            if ch.is_ascii_digit()
                || ch == '.'
                || ch == '+'
                || ch == '-'
                || ch.is_whitespace()
            {
                if !ch.is_whitespace() {
                    number.push(ch);
                }
                index += 1;
            } else {
                break;
            }
        }
        if let Ok(value) = number.parse::<f64>() {
            words.push(Word {
                letter: letter.to_ascii_uppercase(),
                value,
            });
        }
    }
    words
}

//! Ruida wire codec: rpascript commands, 7-bit value packing,
//! the XOR swizzle, packet checksums and reply framing.
//!
//! Ruida controllers speak a binary protocol built from 7-bit
//! values. A command starts with a prefix byte that keeps bit 7 set;
//! every following byte carries seven data bits. Dimensions travel
//! as signed micrometres, speeds/frequencies/times as thousandths,
//! and power as a 14-bit fraction of ``0x4000``. The byte stream is
//! obfuscated by a per-controller swizzle ("magic") before it goes
//! on the wire; UDP packets additionally carry a two-byte checksum
//! prefix and are acknowledged one packet at a time.
//!
//! The command table, the parameter formats and the swizzle are
//! ports of the Ruida Protocol Analyzer's ``ruida_protocol`` tables.

use std::fmt;

/// Controller accepted a packet.
pub const ACK: u8 = 0xCC;
/// Controller rejected a packet.
pub const ERR: u8 = 0xCD;
/// Keep-alive; reply with ENQ or ACK.
pub const ENQ: u8 = 0xCE;
/// Checksum mismatch; resend the packet.
pub const NAK: u8 = 0xCF;
/// End-of-file marker; part of the file checksum.
pub const EOF: u8 = 0xD7;

const KEYRESS_PREFIX: u8 = 0xA7;
const SETTING_PREFIX: u8 = 0xDA;
const FILE_PREFIX: u8 = 0xE5;
const END_JOB_OPCODE: u8 = 0x05;
const END_JOB_BYTES: usize = 5;

/// A malformed script or parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    UnknownMnemonic(String),
    UnknownMemory(String),
    BadParam { mnemonic: String, value: String },
    BadScript(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMnemonic(name) => {
                write!(f, "unknown rpascript command {name:?}")
            }
            Self::UnknownMemory(name) => {
                write!(f, "unknown memory address {name:?}")
            }
            Self::BadParam { mnemonic, value } => {
                write!(f, "invalid value {value:?} for {mnemonic}")
            }
            Self::BadScript(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// How one rpascript parameter is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    Int7,
    Uint7,
    Int14,
    Uint14,
    Int35,
    Uint35,
    Dim14,
    Dim35,
    Power,
    Frequency,
    Speed,
    Time,
    Bool,
    CString,
    String8,
    Color,
    Memory,
    Index,
    CardId,
    Rel,
    Axis,
}

/// A flattened entry of the Ruida command table.
pub struct CommandDef {
    pub mnemonic: &'static str,
    pub prefix: u8,
    pub mid: Option<u8>,
    pub inner: Option<u8>,
    pub params: &'static [Param],
}

impl CommandDef {
    /// The bytes that precede the parameters.
    fn header(&self) -> Vec<u8> {
        let mut header = vec![self.prefix];
        if let Some(mid) = self.mid {
            header.push(mid & 0x7F);
        }
        if let Some(inner) = self.inner {
            header.push(inner & 0x7F);
        }
        header
    }

    fn is_end_job(&self) -> bool {
        self.prefix == FILE_PREFIX && self.mid == Some(END_JOB_OPCODE)
    }

    fn counts_toward_checksum(&self) -> bool {
        self.prefix != KEYRESS_PREFIX
            && self.prefix != SETTING_PREFIX
            && !self.is_end_job()
    }
}

/// One parsed rpascript line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub mnemonic: String,
    pub params: Vec<String>,
}

/// Looks up a command by mnemonic.
pub fn lookup(mnemonic: &str) -> Option<&'static CommandDef> {
    COMMANDS
        .binary_search_by(|entry| entry.mnemonic.cmp(mnemonic))
        .ok()
        .map(|index| &COMMANDS[index])
}

/// Looks up a memory address mnemonic.
pub fn memory_address(mnemonic: &str) -> Option<u16> {
    MEMORY
        .binary_search_by(|(name, _)| (*name).cmp(mnemonic))
        .ok()
        .map(|index| MEMORY[index].1)
}

/// Parses rpascript lines into commands.
///
/// Comments (``#``, ``\#`` escaped), the ``= expected`` suffix of
/// replay scripts, and the analyzer's ``session``/``server``/
/// ``new_packet`` directives are dropped.
pub fn parse_script(lines: &[String]) -> Result<Vec<Command>, CodecError> {
    let mut commands = Vec::new();
    let mut in_block_comment = false;
    for line in lines {
        let (text, still_open) = strip_block_comment(line, in_block_comment);
        in_block_comment = still_open;
        let text = strip_line_comment(&text);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let mut tokens = text.split_whitespace();
        let Some(mnemonic) = tokens.next() else {
            continue;
        };
        let lower = mnemonic.to_ascii_lowercase();
        if matches!(lower.as_str(), "session" | "server" | "new_packet") {
            continue;
        }
        let mut params = Vec::new();
        for token in tokens {
            if token.starts_with('=') {
                break;
            }
            params.push(token.to_string());
        }
        commands.push(Command {
            mnemonic: mnemonic.to_string(),
            params,
        });
    }
    Ok(commands)
}

fn strip_block_comment(line: &str, mut open: bool) -> (String, bool) {
    if !open && !line.contains("\"\"\"") {
        return (line.to_string(), false);
    }
    let mut out = String::new();
    let mut rest = line;
    loop {
        if open {
            match rest.find("\"\"\"") {
                Some(end) => {
                    open = false;
                    rest = &rest[end + 3..];
                }
                None => break,
            }
        } else {
            match rest.find("\"\"\"") {
                Some(start) => {
                    out.push_str(&rest[..start]);
                    open = true;
                    rest = &rest[start + 3..];
                }
                None => {
                    out.push_str(rest);
                    break;
                }
            }
        }
    }
    (out, open)
}

fn strip_line_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'#') {
            out.push('#');
            chars.next();
        } else if ch == '#' {
            break;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Encodes one command to raw (unswizzled) bytes.
pub fn encode_command(command: &Command) -> Result<Vec<u8>, CodecError> {
    let def = lookup(&command.mnemonic)
        .ok_or_else(|| CodecError::UnknownMnemonic(command.mnemonic.clone()))?;
    let mut raw = def.header();
    for (param, token) in def.params.iter().zip(&command.params) {
        raw.extend(encode_param(*param, token).ok_or_else(|| {
            CodecError::BadParam {
                mnemonic: command.mnemonic.clone(),
                value: token.clone(),
            }
        })?);
    }
    Ok(raw)
}

/// Encodes commands into the raw byte stream, filling every
/// ``END_JOB`` checksum with the sum of the bytes that precede it.
///
/// ``SET_SETTING`` (``0xDA``), keypress (``0xA7``) and ``END_JOB``
/// itself do not count; ``EOF`` does.
pub fn encode_commands(commands: &[Command]) -> Result<Vec<u8>, CodecError> {
    let mut stream = Vec::new();
    let mut checksum_offsets = Vec::new();
    let mut checksum: u64 = 0;
    for command in commands {
        let def = lookup(&command.mnemonic).ok_or_else(|| {
            CodecError::UnknownMnemonic(command.mnemonic.clone())
        })?;
        let mut raw = encode_command(command)?;
        if def.is_end_job() {
            if command.params.len() < def.params.len() {
                raw.extend(from_uint(0, END_JOB_BYTES));
            }
            let start = stream.len() + raw.len() - END_JOB_BYTES;
            checksum_offsets.push(start);
        } else if def.counts_toward_checksum() {
            checksum = checksum.wrapping_add(sum_bytes(&raw));
        }
        stream.extend(raw);
    }
    for start in checksum_offsets {
        stream[start..start + END_JOB_BYTES]
            .copy_from_slice(&from_uint(checksum, END_JOB_BYTES));
    }
    Ok(stream)
}

/// Encodes rpascript lines, applying the file checksum.
pub fn encode_script(lines: &[String]) -> Result<Vec<u8>, CodecError> {
    encode_commands(&parse_script(lines)?)
}

fn sum_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().map(|byte| u64::from(*byte)).sum()
}

fn encode_param(param: Param, token: &str) -> Option<Vec<u8>> {
    let value = split_value(token);
    Some(match param {
        Param::Memory | Param::Index => encode_memory(value)?,
        Param::Rel => from_uint(parse_rel(value)?, 1),
        Param::Axis => from_uint(parse_axis(value)?, 1),
        Param::Bool => vec![u8::from(parse_bool(value)?)],
        Param::Dim14 => {
            from_int(round_half_even(parse_dim(value)? * 1000.0), 2)
        }
        Param::Dim35 => {
            from_int(round_half_even(parse_dim(value)? * 1000.0), 5)
        }
        Param::Power => {
            let percent = parse_float(strip_suffix(value, "%"))?;
            from_uint(
                round_half_even(percent * 16384.0 / 100.0).max(0) as u64,
                2,
            )
        }
        Param::Frequency => {
            let value =
                parse_float(strip_suffixes(value, &["KHz", "khz", "kHz"]))?;
            from_int(round_half_even(value * 1000.0), 5)
        }
        Param::Speed => {
            let value =
                parse_float(strip_suffixes(value, &["mm/S", "mm/s", "MM/S"]))?;
            from_int(round_half_even(value * 1000.0), 5)
        }
        Param::Time => {
            let value =
                parse_float(strip_suffixes(value, &["mS", "ms", "MS"]))?;
            from_int(round_half_even(value * 1000.0), 5)
        }
        Param::CString => {
            let mut bytes = unquote(value).as_bytes().to_vec();
            bytes.push(0);
            bytes
        }
        Param::String8 => encode_string8(unquote(value))?,
        Param::Color => {
            let rgb = parse_int(value)? as u64 & 0xFF_FFFF;
            from_uint(swap_red_blue(rgb), 5)
        }
        Param::CardId => from_uint(parse_card_id(value)?, 5),
        Param::Int7 => from_int(parse_int(value)?, 1),
        Param::Uint7 => from_uint(parse_int(value)? as u64, 1),
        Param::Int14 => from_int(parse_int(value)?, 2),
        Param::Uint14 => from_uint(parse_int(value)? as u64, 2),
        Param::Int35 => from_int(parse_int(value)?, 5),
        Param::Uint35 => from_uint(parse_int(value)? as u64, 5),
    })
}

fn split_value(token: &str) -> &str {
    let value = token.trim();
    let value = match value.split_once('=') {
        Some((_, rest)) => rest,
        None => value,
    };
    strip_label(value).trim()
}

fn strip_label(value: &str) -> &str {
    match value.split_once(':') {
        Some((label, rest)) if !label.is_empty() => {
            if label.chars().all(|ch| ch.is_ascii_alphabetic()) {
                return rest.trim_start();
            }
            value
        }
        _ => value,
    }
}

fn strip_suffix<'a>(value: &'a str, suffix: &str) -> &'a str {
    value.strip_suffix(suffix).unwrap_or(value)
}

fn strip_suffixes<'a>(value: &'a str, suffixes: &[&str]) -> &'a str {
    let lower = value.to_ascii_lowercase();
    for suffix in suffixes {
        if lower.ends_with(&suffix.to_ascii_lowercase()) {
            return &value[..value.len() - suffix.len()];
        }
    }
    value
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_uppercase().as_str() {
        "ON" | "TRUE" | "1" => Some(true),
        "OFF" | "FALSE" | "0" => Some(false),
        _ => None,
    }
}

fn parse_dim(value: &str) -> Option<f64> {
    let lower = value.to_ascii_lowercase();
    let clean = match lower.strip_suffix("mm") {
        Some(_) => &value[..value.len() - 2],
        None => value,
    };
    clean.trim().parse::<f64>().ok()
}

fn parse_float(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn parse_int(value: &str) -> Option<i64> {
    let value = value.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Some(hex) = value.strip_prefix('#') {
        return i64::from_str_radix(hex, 16).ok();
    }
    value.parse::<i64>().ok()
}

fn parse_rel(value: &str) -> Option<u64> {
    match value.to_ascii_uppercase().as_str() {
        "MACHINE" => Some(0),
        "ANCHOR" => Some(1),
        "CURRENT" => Some(2),
        "SET_POINT" => Some(3),
        _ => parse_int(value).map(|value| value as u64),
    }
}

fn parse_axis(value: &str) -> Option<u64> {
    match value.to_ascii_uppercase().as_str() {
        "X" => Some(0),
        "Y" => Some(1),
        "Z" => Some(2),
        "U" => Some(3),
        _ => parse_int(value).map(|value| value as u64),
    }
}

fn parse_card_id(value: &str) -> Option<u64> {
    match value {
        "RDC6442S" => Some(0x6510_6510),
        "RDC8445S" => Some(0x9010_9010),
        _ => parse_int(value).map(|value| value as u64),
    }
}

fn encode_memory(value: &str) -> Option<Vec<u8>> {
    let address = match memory_address(value) {
        Some(address) => address,
        None => {
            let number = parse_int(value)?;
            u16::try_from(number).ok()?
        }
    };
    Some(vec![(address >> 8) as u8 & 0x7F, address as u8 & 0x7F])
}

fn encode_string8(value: &str) -> Option<Vec<u8>> {
    let mut bytes = [0u8; 8];
    let source = value.as_bytes();
    let length = source.len().min(8);
    bytes[..length].copy_from_slice(&source[..length]);
    let mut out = from_uint(
        u64::from_be_bytes([
            0, 0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3],
        ]),
        5,
    );
    out.extend(from_uint(
        u64::from_be_bytes([
            0, 0, 0, 0, bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        5,
    ));
    Some(out)
}

fn swap_red_blue(rgb: u64) -> u64 {
    ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF)
}

fn round_half_even(value: f64) -> i64 {
    let truncated = value.trunc();
    if (value - truncated).abs() == 0.5 {
        let even = truncated as i64;
        if even % 2 == 0 {
            even
        } else {
            (truncated + value.signum()) as i64
        }
    } else {
        value.round() as i64
    }
}

/// Packs an unsigned integer into ``n`` 7-bit bytes, most
/// significant group first.
pub fn from_uint(value: u64, n: usize) -> Vec<u8> {
    (0..n)
        .rev()
        .map(|index| ((value >> (index * 7)) & 0x7F) as u8)
        .collect()
}

/// Packs a signed integer using the protocol's sign convention: the
/// top bit of the first byte is the sign, the data bits are two's
/// complement.
pub fn from_int(value: i64, n: usize) -> Vec<u8> {
    let mut data = if value < 0 {
        let mask = (1i64 << (n * 7 - 1)) - 1;
        mask + 1 + value
    } else {
        value
    };
    if data < 0 {
        data = 0;
    }
    let mut bytes = from_uint(data as u64, n);
    if value < 0 {
        bytes[0] |= 0x40;
    }
    bytes
}

/// Reads ``n`` 7-bit bytes as an unsigned integer.
pub fn to_uint(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0, |value, byte| (value << 7) | u64::from(byte & 0x7F))
}

/// Reads ``n`` 7-bit bytes as a signed integer, including the
/// LightBurn sign-extension workaround.
pub fn to_int(bytes: &[u8]) -> i64 {
    let Some(first) = bytes.first().copied() else {
        return 0;
    };
    let mut first = first;
    if bytes.len() == 5 && first & 0x40 == 0 && first & 0x08 != 0 {
        first |= 0x70;
    }
    let mut value = 0i64;
    for (index, byte) in bytes.iter().enumerate() {
        let mut data = byte & 0x7F;
        if index == 0 {
            data = first & 0x3F;
        }
        value = (value << 7) | i64::from(data);
    }
    if first & 0x40 != 0 {
        let mask = (1i64 << (bytes.len() * 7 - 1)) - 1;
        value = -((!value & mask) + 1);
    }
    value
}

/// Forward-swizzles one byte with the controller's magic.
pub fn swizzle_byte(mut byte: u8, magic: u8) -> u8 {
    byte ^= byte >> 7;
    byte ^= byte << 7;
    byte ^= byte >> 7;
    byte ^= magic;
    byte.wrapping_add(1)
}

/// Reverses [`swizzle_byte`].
pub fn unswizzle_byte(mut byte: u8, magic: u8) -> u8 {
    byte = byte.wrapping_sub(1);
    byte ^= magic;
    byte ^= byte >> 7;
    byte ^= byte << 7;
    byte ^= byte >> 7;
    byte
}

/// Swizzles a byte slice.
pub fn swizzle(data: &[u8], magic: u8) -> Vec<u8> {
    data.iter().map(|byte| swizzle_byte(*byte, magic)).collect()
}

/// Unswizzles a byte slice.
pub fn unswizzle(data: &[u8], magic: u8) -> Vec<u8> {
    data.iter()
        .map(|byte| unswizzle_byte(*byte, magic))
        .collect()
}

/// Builds the table that maps a raw single-byte reply to the magic
/// that makes it an ACK.
pub fn ack_magic_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    for magic in 0..=255u8 {
        table[usize::from(swizzle_byte(ACK, magic))] = magic;
    }
    table
}

/// Packages a raw payload for the wire: swizzle it and, for UDP,
/// prefix the checksum of the swizzled bytes.
pub fn package(payload: &[u8], magic: u8, udp: bool) -> Vec<u8> {
    let swizzled = swizzle(payload, magic);
    if !udp {
        return swizzled;
    }
    let checksum: u16 = swizzled
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)));
    let mut packet = Vec::with_capacity(swizzled.len() + 2);
    packet.push((checksum >> 8) as u8);
    packet.push((checksum & 0xFF) as u8);
    packet.extend(swizzled);
    packet
}

/// A decoded ``GET_SETTING`` reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub address: u16,
    pub data: Vec<u8>,
}

/// Splits unswizzled reply bytes into replies.
///
/// A reply starts with ``0xDA`` and ends at the next byte with bit 7
/// set, because every value byte is 7-bit. Chunks shorter than nine
/// bytes are malformed and skipped; scanning continues.
pub fn unpack_replies(raw: &[u8]) -> Vec<Reply> {
    const MIN_REPLY_LEN: usize = 9;
    let mut replies = Vec::new();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] != 0xDA {
            break;
        }
        let mut end = index + 1;
        while end < raw.len() && raw[end] & 0x80 == 0 {
            end += 1;
        }
        let chunk = &raw[index..end];
        if chunk.len() < MIN_REPLY_LEN {
            index = end;
            continue;
        }
        if chunk[1] != 0x01 {
            break;
        }
        replies.push(Reply {
            address: (u16::from(chunk[2]) << 8) | u16::from(chunk[3]),
            data: chunk[4..].to_vec(),
        });
        index = end;
    }
    replies
}

/// Builds a raw ``GET_SETTING`` command for an address.
pub fn get_setting(address: u16) -> Vec<u8> {
    vec![
        SETTING_PREFIX,
        0x00,
        (address >> 8) as u8 & 0x7F,
        address as u8 & 0x7F,
    ]
}

/// Builds a raw ``SET_SETTING`` command.
pub fn set_setting(address: u16, a: u64, b: u64) -> Vec<u8> {
    let mut command = vec![
        SETTING_PREFIX,
        0x01,
        (address >> 8) as u8 & 0x7F,
        address as u8 & 0x7F,
    ];
    command.extend(from_uint(a, 5));
    command.extend(from_uint(b, 5));
    command
}

pub(crate) static COMMANDS: &[CommandDef] = &[
    CommandDef {
        mnemonic: "ADD_CUT_DELAY",
        prefix: 0xC6,
        mid: Some(0x11),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "AIR_ASSIST_OFF",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x12),
        params: &[],
    },
    CommandDef {
        mnemonic: "AIR_ASSIST_ON",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x13),
        params: &[],
    },
    CommandDef {
        mnemonic: "ARRAY_ADD",
        prefix: 0xE7,
        mid: Some(0x23),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ARRAY_BOTTOM_LEFT",
        prefix: 0xE7,
        mid: Some(0x17),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ARRAY_COPIES",
        prefix: 0xE7,
        mid: Some(0x08),
        inner: None,
        params: &[Param::Uint14, Param::Uint14, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ARRAY_DIRECTION",
        prefix: 0xE7,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Int7],
    },
    CommandDef {
        mnemonic: "ARRAY_END",
        prefix: 0xEB,
        mid: None,
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "ARRAY_EN_MIRROR_CUT",
        prefix: 0xE7,
        mid: Some(0x0B),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ARRAY_EVEN_DISTANCE",
        prefix: 0xE7,
        mid: Some(0x37),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ARRAY_MIRROR",
        prefix: 0xE7,
        mid: Some(0x24),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ARRAY_START",
        prefix: 0xEA,
        mid: None,
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ARRAY_TOP_RIGHT",
        prefix: 0xE7,
        mid: Some(0x13),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "BLOCK_END",
        prefix: 0xE7,
        mid: Some(0x00),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "BLOCK_X_SIZE",
        prefix: 0xE7,
        mid: Some(0x35),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "CALCULATE_DOCUMENT_TIME",
        prefix: 0xE8,
        mid: Some(0x04),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "CUT_FAR_XY",
        prefix: 0xA8,
        mid: None,
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "CUT_NEAR_X",
        prefix: 0xAA,
        mid: None,
        inner: None,
        params: &[Param::Dim14],
    },
    CommandDef {
        mnemonic: "CUT_NEAR_XY",
        prefix: 0xA9,
        mid: None,
        inner: None,
        params: &[Param::Dim14, Param::Dim14],
    },
    CommandDef {
        mnemonic: "CUT_NEAR_Y",
        prefix: 0xAB,
        mid: None,
        inner: None,
        params: &[Param::Dim14],
    },
    CommandDef {
        mnemonic: "CUT_PIERCE",
        prefix: 0xC6,
        mid: Some(0x10),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "CUT_SPEED_LASER_1",
        prefix: 0xC9,
        mid: Some(0x04),
        inner: None,
        params: &[Param::Int7, Param::Speed],
    },
    CommandDef {
        mnemonic: "DB_HEAD",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x14),
        params: &[],
    },
    CommandDef {
        mnemonic: "DEFAULT_COLOR",
        prefix: 0xCA,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Color],
    },
    CommandDef {
        mnemonic: "DELETE_DOCUMENT",
        prefix: 0xE8,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Uint35, Param::Uint35],
    },
    CommandDef {
        mnemonic: "DISPLAY_OFFSET",
        prefix: 0xF1,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "DOCUMENT_BOTTOM_LEFT",
        prefix: 0xE7,
        mid: Some(0x51),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "DOCUMENT_FILE_END",
        prefix: 0xE5,
        mid: Some(0x02),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "DOCUMENT_FILE_UPLOAD",
        prefix: 0xE5,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Uint14, Param::Uint35, Param::Uint35],
    },
    CommandDef {
        mnemonic: "DOCUMENT_NUMBER",
        prefix: 0xE8,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Uint14],
    },
    CommandDef {
        mnemonic: "DOCUMENT_TOP_RIGHT",
        prefix: 0xE7,
        mid: Some(0x50),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ELEMENT_ARRAY_ADD",
        prefix: 0xF2,
        mid: Some(0x06),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ELEMENT_ARRAY_BOTTOM_LEFT",
        prefix: 0xF2,
        mid: Some(0x04),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ELEMENT_ARRAY_MIRROR",
        prefix: 0xF2,
        mid: Some(0x07),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ELEMENT_ARRAY_TOP_RIGHT",
        prefix: 0xF2,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ELEMENT_COPIES",
        prefix: 0xF2,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Uint14, Param::Uint14, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "ELEMENT_INDEX",
        prefix: 0xF2,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ELEMENT_MAX_INDEX",
        prefix: 0xF1,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ELEMENT_NAME",
        prefix: 0xF2,
        mid: Some(0x02),
        inner: None,
        params: &[Param::String8],
    },
    CommandDef {
        mnemonic: "ELEMENT_NAME_INDEX",
        prefix: 0xF2,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ELEMENT_NAME_MAX_INDEX",
        prefix: 0xF1,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "ENABLE_BLOCK_CUTTING",
        prefix: 0xF1,
        mid: Some(0x02),
        inner: None,
        params: &[Param::Bool],
    },
    CommandDef {
        mnemonic: "END_JOB",
        prefix: 0xE5,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Uint35],
    },
    CommandDef {
        mnemonic: "END_POWER_1",
        prefix: 0xC8,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "END_POWER_2",
        prefix: 0xC1,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "END_POWER_3",
        prefix: 0xC3,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "END_POWER_4",
        prefix: 0xC5,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "ENQ",
        prefix: 0xCE,
        mid: None,
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "EN_EX_IO",
        prefix: 0xCA,
        mid: Some(0x10),
        inner: None,
        params: &[Param::Int7],
    },
    CommandDef {
        mnemonic: "EN_LASER_2_OFFSET_0",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x30),
        params: &[],
    },
    CommandDef {
        mnemonic: "EN_LASER_2_OFFSET_1",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x31),
        params: &[],
    },
    CommandDef {
        mnemonic: "EN_LASER_TUBE_START",
        prefix: 0xCA,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Bool],
    },
    CommandDef {
        mnemonic: "EOF",
        prefix: 0xD7,
        mid: None,
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "FEED_AUTO_CALC",
        prefix: 0xF1,
        mid: Some(0x04),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "FEED_INFO",
        prefix: 0xE7,
        mid: Some(0x0A),
        inner: None,
        params: &[Param::Int35],
    },
    CommandDef {
        mnemonic: "FEED_LENGTH",
        prefix: 0xE7,
        mid: Some(0x09),
        inner: None,
        params: &[Param::Int35],
    },
    CommandDef {
        mnemonic: "FEED_REPEAT",
        prefix: 0xE7,
        mid: Some(0x06),
        inner: None,
        params: &[Param::Uint35, Param::Uint35],
    },
    CommandDef {
        mnemonic: "FILE_TRANSFER",
        prefix: 0xE8,
        mid: Some(0x02),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "FOCUS_Z",
        prefix: 0xD8,
        mid: Some(0x2E),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "FORCE_ENG_SPEED",
        prefix: 0xC9,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Speed],
    },
    CommandDef {
        mnemonic: "GET_SETTING",
        prefix: 0xDA,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Memory],
    },
    CommandDef {
        mnemonic: "GET_UNKNOWN",
        prefix: 0xDA,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Index, Param::Uint35],
    },
    CommandDef {
        mnemonic: "HOME_U",
        prefix: 0xD8,
        mid: Some(0x2D),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "HOME_XY",
        prefix: 0xD8,
        mid: Some(0x2A),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "HOME_Z",
        prefix: 0xD8,
        mid: Some(0x2C),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "IMD_POWER_1",
        prefix: 0xC7,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "IMD_POWER_2",
        prefix: 0xC0,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "IMD_POWER_3",
        prefix: 0xC2,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "IMD_POWER_4",
        prefix: 0xC4,
        mid: None,
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "JOB_BOTTOM_LEFT",
        prefix: 0xE7,
        mid: Some(0x07),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOB_COPIES",
        prefix: 0xE7,
        mid: Some(0x04),
        inner: None,
        params: &[Param::Uint14, Param::Uint14, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOB_TOP_RIGHT",
        prefix: 0xE7,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_FEED_AXIS",
        prefix: 0xD9,
        mid: Some(0x0F),
        inner: None,
        params: &[Param::Rel],
    },
    CommandDef {
        mnemonic: "JOG_U",
        prefix: 0xD9,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Rel, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_X",
        prefix: 0xD9,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Rel, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_XY",
        prefix: 0xD9,
        mid: Some(0x10),
        inner: None,
        params: &[Param::Rel, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_XYU",
        prefix: 0xD9,
        mid: Some(0x30),
        inner: None,
        params: &[Param::Rel, Param::Dim35, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_Y",
        prefix: 0xD9,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Rel, Param::Dim35],
    },
    CommandDef {
        mnemonic: "JOG_Z",
        prefix: 0xD9,
        mid: Some(0x02),
        inner: None,
        params: &[Param::Rel, Param::Dim35],
    },
    CommandDef {
        mnemonic: "KEYDOWN_U_BACKWARDS",
        prefix: 0xD8,
        mid: Some(0x27),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_U_FORWARD",
        prefix: 0xD8,
        mid: Some(0x26),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_X_LEFT",
        prefix: 0xD8,
        mid: Some(0x20),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_X_RIGHT",
        prefix: 0xD8,
        mid: Some(0x21),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_Y_BOTTOM",
        prefix: 0xD8,
        mid: Some(0x23),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_Y_TOP",
        prefix: 0xD8,
        mid: Some(0x22),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_Z_DOWN",
        prefix: 0xD8,
        mid: Some(0x25),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYDOWN_Z_UP",
        prefix: 0xD8,
        mid: Some(0x24),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_LEFT",
        prefix: 0xD8,
        mid: Some(0x30),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_RIGHT",
        prefix: 0xD8,
        mid: Some(0x31),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_U_BACKWARDS",
        prefix: 0xD8,
        mid: Some(0x37),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_U_FORWARD",
        prefix: 0xD8,
        mid: Some(0x36),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_Y_BOTTOM",
        prefix: 0xD8,
        mid: Some(0x33),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_Y_TOP",
        prefix: 0xD8,
        mid: Some(0x32),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_Z_DOWN",
        prefix: 0xD8,
        mid: Some(0x35),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "KEYUP_Z_UP",
        prefix: 0xD8,
        mid: Some(0x34),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "LASER_DEVICE_0",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x10),
        params: &[],
    },
    CommandDef {
        mnemonic: "LASER_DEVICE_1",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x11),
        params: &[],
    },
    CommandDef {
        mnemonic: "LASER_OFF_DELAY",
        prefix: 0xC6,
        mid: Some(0x13),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "LASER_OFF_DELAY2",
        prefix: 0xC6,
        mid: Some(0x16),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "LASER_ON_DELAY",
        prefix: 0xC6,
        mid: Some(0x12),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "LASER_ON_DELAY2",
        prefix: 0xC6,
        mid: Some(0x15),
        inner: None,
        params: &[Param::Time],
    },
    CommandDef {
        mnemonic: "LAST_LAYER",
        prefix: 0xCA,
        mid: Some(0x22),
        inner: None,
        params: &[Param::Int7],
    },
    CommandDef {
        mnemonic: "LAYER_ATTRIBUTES",
        prefix: 0xCA,
        mid: Some(0x41),
        inner: None,
        params: &[Param::Int7, Param::Uint7],
    },
    CommandDef {
        mnemonic: "LAYER_BOTTOM_LEFT",
        prefix: 0xE7,
        mid: Some(0x53),
        inner: None,
        params: &[Param::Int7, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "LAYER_COLOR",
        prefix: 0xCA,
        mid: Some(0x06),
        inner: None,
        params: &[Param::Int7, Param::Color],
    },
    CommandDef {
        mnemonic: "LAYER_EX_BOTTOM_LEFT",
        prefix: 0xE7,
        mid: Some(0x62),
        inner: None,
        params: &[Param::Int7, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "LAYER_EX_TOP_RIGHT",
        prefix: 0xE7,
        mid: Some(0x61),
        inner: None,
        params: &[Param::Int7, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "LAYER_FREQUENCY",
        prefix: 0xC6,
        mid: Some(0x60),
        inner: None,
        params: &[Param::Int7, Param::Int7, Param::Frequency],
    },
    CommandDef {
        mnemonic: "LAYER_MAX_POWER_1",
        prefix: 0xC6,
        mid: Some(0x32),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MAX_POWER_2",
        prefix: 0xC6,
        mid: Some(0x42),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MAX_POWER_3",
        prefix: 0xC6,
        mid: Some(0x36),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MAX_POWER_4",
        prefix: 0xC6,
        mid: Some(0x38),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MIN_POWER_1",
        prefix: 0xC6,
        mid: Some(0x31),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MIN_POWER_2",
        prefix: 0xC6,
        mid: Some(0x41),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MIN_POWER_3",
        prefix: 0xC6,
        mid: Some(0x35),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_MIN_POWER_4",
        prefix: 0xC6,
        mid: Some(0x37),
        inner: None,
        params: &[Param::Int7, Param::Power],
    },
    CommandDef {
        mnemonic: "LAYER_OFFSET_AXIS",
        prefix: 0xE7,
        mid: Some(0x55),
        inner: None,
        params: &[Param::Axis, Param::Dim35],
    },
    CommandDef {
        mnemonic: "LAYER_TOP_RIGHT",
        prefix: 0xE7,
        mid: Some(0x52),
        inner: None,
        params: &[Param::Int7, Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "MAX_POWER_1",
        prefix: 0xC6,
        mid: Some(0x02),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MAX_POWER_2",
        prefix: 0xC6,
        mid: Some(0x22),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MAX_POWER_3",
        prefix: 0xC6,
        mid: Some(0x06),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MAX_POWER_4",
        prefix: 0xC6,
        mid: Some(0x08),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MIN_POWER_1",
        prefix: 0xC6,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MIN_POWER_2",
        prefix: 0xC6,
        mid: Some(0x21),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MIN_POWER_3",
        prefix: 0xC6,
        mid: Some(0x05),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MIN_POWER_4",
        prefix: 0xC6,
        mid: Some(0x07),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_A",
        prefix: 0xA0,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_U",
        prefix: 0xA0,
        mid: Some(0x08),
        inner: None,
        params: &[Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_X",
        prefix: 0x80,
        mid: Some(0x00),
        inner: None,
        params: &[Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_XY",
        prefix: 0x88,
        mid: None,
        inner: None,
        params: &[Param::Dim35, Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_Y",
        prefix: 0x80,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_FAR_Z",
        prefix: 0x80,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Dim35],
    },
    CommandDef {
        mnemonic: "MOVE_NEAR_X",
        prefix: 0x8A,
        mid: None,
        inner: None,
        params: &[Param::Dim14],
    },
    CommandDef {
        mnemonic: "MOVE_NEAR_XY",
        prefix: 0x89,
        mid: None,
        inner: None,
        params: &[Param::Dim14, Param::Dim14],
    },
    CommandDef {
        mnemonic: "MOVE_NEAR_Y",
        prefix: 0x8B,
        mid: None,
        inner: None,
        params: &[Param::Dim14],
    },
    CommandDef {
        mnemonic: "OVERSCAN_5",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x55),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_DIAGONAL",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x05),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_H_BI",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x01),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_H_UNI",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x02),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_OFF",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x00),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_V_BI",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x03),
        params: &[],
    },
    CommandDef {
        mnemonic: "OVERSCAN_V_UNI",
        prefix: 0xCA,
        mid: Some(0x01),
        inner: Some(0x04),
        params: &[],
    },
    CommandDef {
        mnemonic: "PAUSE_JOB",
        prefix: 0xD8,
        mid: Some(0x02),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "PEN_OFFSET_AXIS",
        prefix: 0xE7,
        mid: Some(0x54),
        inner: None,
        params: &[Param::Axis, Param::Dim35],
    },
    CommandDef {
        mnemonic: "REF_POINT_CURRENT",
        prefix: 0xD8,
        mid: Some(0x12),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "REF_POINT_MACHINE",
        prefix: 0xD8,
        mid: Some(0x10),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "REF_POINT_ORIGIN",
        prefix: 0xD8,
        mid: Some(0x11),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "REF_POINT_SET",
        prefix: 0xF0,
        mid: None,
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "RESUME_JOB",
        prefix: 0xD8,
        mid: Some(0x03),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "SELECT_DOCUMENT",
        prefix: 0xE8,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "SELECT_LAYER",
        prefix: 0xCA,
        mid: Some(0x02),
        inner: None,
        params: &[Param::Int7],
    },
    CommandDef {
        mnemonic: "SET_ABSOLUTE",
        prefix: 0xE6,
        mid: Some(0x01),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "SET_CURRENT_ELEMENT_INDEX",
        prefix: 0xE7,
        mid: Some(0x60),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "SET_FEED_AUTO_PAUSE",
        prefix: 0xE7,
        mid: Some(0x38),
        inner: None,
        params: &[Param::Bool],
    },
    CommandDef {
        mnemonic: "SET_FILE_EMPTY",
        prefix: 0xE7,
        mid: Some(0x36),
        inner: None,
        params: &[Param::Uint7],
    },
    CommandDef {
        mnemonic: "SET_FILE_NAME",
        prefix: 0xE7,
        mid: Some(0x01),
        inner: None,
        params: &[Param::CString],
    },
    CommandDef {
        mnemonic: "SET_SETTING",
        prefix: 0xDA,
        mid: Some(0x01),
        inner: None,
        params: &[Param::Memory, Param::Uint35, Param::Uint35],
    },
    CommandDef {
        mnemonic: "SKIP",
        prefix: 0xD0,
        mid: Some(0x29),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "SPEED_AXIS",
        prefix: 0xC9,
        mid: Some(0x03),
        inner: None,
        params: &[Param::Speed],
    },
    CommandDef {
        mnemonic: "SPEED_AXIS_MOVE",
        prefix: 0xC9,
        mid: Some(0x06),
        inner: None,
        params: &[Param::Speed],
    },
    CommandDef {
        mnemonic: "SPEED_LASER_1",
        prefix: 0xC9,
        mid: Some(0x02),
        inner: None,
        params: &[Param::Speed],
    },
    CommandDef {
        mnemonic: "START_JOB",
        prefix: 0xD8,
        mid: Some(0x00),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "STOP_JOB",
        prefix: 0xD8,
        mid: Some(0x01),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "THROUGH_POWER_1",
        prefix: 0xC6,
        mid: Some(0x50),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "THROUGH_POWER_2",
        prefix: 0xC6,
        mid: Some(0x51),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "THROUGH_POWER_3",
        prefix: 0xC6,
        mid: Some(0x55),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "THROUGH_POWER_4",
        prefix: 0xC6,
        mid: Some(0x56),
        inner: None,
        params: &[Param::Power],
    },
    CommandDef {
        mnemonic: "UNION_BLOCK_PROPERTY",
        prefix: 0xE7,
        mid: Some(0x3A),
        inner: None,
        params: &[],
    },
    CommandDef {
        mnemonic: "UNKNOWN_E732",
        prefix: 0xE7,
        mid: Some(0x32),
        inner: None,
        params: &[Param::Uint35, Param::Uint35],
    },
    CommandDef {
        mnemonic: "U_FILE_ID",
        prefix: 0xCA,
        mid: Some(0x30),
        inner: None,
        params: &[Param::Uint14],
    },
    CommandDef {
        mnemonic: "X_SIGN_MAP",
        prefix: 0xCA,
        mid: Some(0x04),
        inner: None,
        params: &[Param::Int7],
    },
    CommandDef {
        mnemonic: "ZU_MAP",
        prefix: 0xCA,
        mid: Some(0x40),
        inner: None,
        params: &[Param::Int7],
    },
];

pub(crate) static MEMORY: &[(&str, u16)] = &[
    ("MEM_ACC_G0_RATIO", 0x021C),
    ("MEM_ACC_RATIO", 0x021A),
    ("MEM_ARRAY_FEED_REPAY", 0x0217),
    ("MEM_AUTO_TYPE_SPACE", 0x001E),
    ("MEM_AXIS_BACKLASH_1", 0x002B),
    ("MEM_AXIS_BACKLASH_2", 0x003B),
    ("MEM_AXIS_BACKLASH_3", 0x004B),
    ("MEM_AXIS_BACKLASH_4", 0x005B),
    ("MEM_AXIS_BTN_ACC_1", 0x0028),
    ("MEM_AXIS_BTN_ACC_2", 0x0038),
    ("MEM_AXIS_BTN_ACC_3", 0x0048),
    ("MEM_AXIS_BTN_ACC_4", 0x0058),
    ("MEM_AXIS_BTN_START_VEL_1", 0x0027),
    ("MEM_AXIS_BTN_START_VEL_2", 0x0037),
    ("MEM_AXIS_BTN_START_VEL_3", 0x0047),
    ("MEM_AXIS_BTN_START_VEL_4", 0x0057),
    ("MEM_AXIS_CONTROL_PARA_1", 0x0020),
    ("MEM_AXIS_CONTROL_PARA_2", 0x0030),
    ("MEM_AXIS_CONTROL_PARA_3", 0x0040),
    ("MEM_AXIS_CONTROL_PARA_4", 0x0050),
    ("MEM_AXIS_ESTP_ACC_1", 0x0029),
    ("MEM_AXIS_ESTP_ACC_2", 0x0039),
    ("MEM_AXIS_ESTP_ACC_3", 0x0049),
    ("MEM_AXIS_ESTP_ACC_4", 0x0059),
    ("MEM_AXIS_HOME_OFFSET_1", 0x002A),
    ("MEM_AXIS_HOME_OFFSET_2", 0x003A),
    ("MEM_AXIS_HOME_OFFSET_3", 0x004A),
    ("MEM_AXIS_HOME_OFFSET_4", 0x005A),
    ("MEM_AXIS_MAX_ACC_1", 0x0025),
    ("MEM_AXIS_MAX_ACC_2", 0x0035),
    ("MEM_AXIS_MAX_ACC_3", 0x0045),
    ("MEM_AXIS_MAX_ACC_4", 0x0055),
    ("MEM_AXIS_MAX_VELOCITY_1", 0x0023),
    ("MEM_AXIS_MAX_VELOCITY_2", 0x0033),
    ("MEM_AXIS_MAX_VELOCITY_3", 0x0043),
    ("MEM_AXIS_MAX_VELOCITY_4", 0x0053),
    ("MEM_AXIS_PRECISION_1", 0x0021),
    ("MEM_AXIS_PRECISION_2", 0x0031),
    ("MEM_AXIS_PRECISION_3", 0x0041),
    ("MEM_AXIS_PRECISION_4", 0x0051),
    ("MEM_AXIS_RANGE_3", 0x0046),
    ("MEM_AXIS_RANGE_4", 0x0056),
    ("MEM_AXIS_START_VELOCITY_1", 0x0024),
    ("MEM_AXIS_START_VELOCITY_2", 0x0034),
    ("MEM_AXIS_START_VELOCITY_3", 0x0044),
    ("MEM_AXIS_START_VELOCITY_4", 0x0054),
    ("MEM_BED_SIZE_X", 0x0026),
    ("MEM_BED_SIZE_Y", 0x0036),
    ("MEM_BTN_ACC_U", 0x0058),
    ("MEM_BTN_ACC_X", 0x0028),
    ("MEM_BTN_ACC_Y", 0x0038),
    ("MEM_BTN_ACC_Z", 0x0048),
    ("MEM_BTN_START_VEL_U", 0x0057),
    ("MEM_BTN_START_VEL_X", 0x0027),
    ("MEM_BTN_START_VEL_Y", 0x0037),
    ("MEM_BTN_START_VEL_Z", 0x0047),
    ("MEM_CARD_ID", 0x057E),
    ("MEM_CARD_LANGUAGE", 0x0300),
    ("MEM_CARD_LOCK", 0x0B11),
    ("MEM_CURRENT_POSITION_U", 0x0451),
    ("MEM_CURRENT_POSITION_X", 0x0421),
    ("MEM_CURRENT_POSITION_Y", 0x0431),
    ("MEM_CURRENT_POSITION_Z", 0x0441),
    ("MEM_DOCUMENT_TIME", 0x0710),
    ("MEM_ENG_ACC_RATIO", 0x0237),
    ("MEM_ENG_FACULA", 0x000B),
    ("MEM_ENG_VERT_VELOCITY", 0x000E),
    ("MEM_ESTP_ACC_U", 0x0059),
    ("MEM_ESTP_ACC_X", 0x0029),
    ("MEM_ESTP_ACC_Y", 0x0039),
    ("MEM_ESTP_ACC_Z", 0x0049),
    ("MEM_FEED_DELAY_AFTER", 0x0207),
    ("MEM_FEED_DELAY_PRIOR", 0x020B),
    ("MEM_FOCUS_DEPTH", 0x020E),
    ("MEM_G0_ACC", 0x020A),
    ("MEM_G0_DELAY", 0x0203),
    ("MEM_G0_VELOCITY", 0x0005),
    ("MEM_GO_SCALE_BLANK", 0x020F),
    ("MEM_HOME_OFFSET_U", 0x005A),
    ("MEM_HOME_OFFSET_X", 0x002A),
    ("MEM_HOME_OFFSET_Y", 0x003A),
    ("MEM_HOME_OFFSET_Z", 0x004A),
    ("MEM_HOME_VELOCITY", 0x000C),
    ("MEM_IO_ENABLE", 0x0004),
    ("MEM_LASER_ATTENUATION", 0x0016),
    ("MEM_LASER_MAX_POWER_1", 0x0013),
    ("MEM_LASER_MAX_POWER_2", 0x0019),
    ("MEM_LASER_MAX_POWER_3", 0x0064),
    ("MEM_LASER_MAX_POWER_4", 0x0069),
    ("MEM_LASER_MIN_POWER_1", 0x0012),
    ("MEM_LASER_MIN_POWER_2", 0x0018),
    ("MEM_LASER_MIN_POWER_3", 0x0063),
    ("MEM_LASER_MIN_POWER_4", 0x0068),
    ("MEM_LASER_PWM_FREQUENCY_1", 0x0011),
    ("MEM_LASER_PWM_FREQUENCY_2", 0x0017),
    ("MEM_LASER_PWM_FREQUENCY_3", 0x0065),
    ("MEM_LASER_PWM_FREQUENCY_4", 0x006A),
    ("MEM_LASER_STANDBY_FREQUENCY_1", 0x001A),
    ("MEM_LASER_STANDBY_FREQUENCY_2", 0x001C),
    ("MEM_LASER_STANDBY_FREQUENCY_3", 0x0066),
    ("MEM_LASER_STANDBY_FREQUENCY_4", 0x006B),
    ("MEM_LASER_STANDBY_PULSE_1", 0x001B),
    ("MEM_LASER_STANDBY_PULSE_2", 0x001D),
    ("MEM_LASER_STANDBY_PULSE_3", 0x0067),
    ("MEM_LASER_STANDBY_PULSE_4", 0x006C),
    ("MEM_MACHINE_FEATURES", 0x030F),
    ("MEM_MACHINE_STATUS", 0x0400),
    ("MEM_MACHINE_TYPE_(0X1155,_0XAA55)", 0x0060),
    ("MEM_MAINBOARD_VERSION", 0x057F),
    ("MEM_MANUAL_DIS", 0x020C),
    ("MEM_MANUAL_FAST_SPEED", 0x0231),
    ("MEM_MANUAL_SLOW_SPEED", 0x0232),
    ("MEM_MAX_ACC_U", 0x0055),
    ("MEM_MAX_ACC_X", 0x0025),
    ("MEM_MAX_ACC_Y", 0x0035),
    ("MEM_MAX_ACC_Z", 0x0045),
    ("MEM_MAX_VELOCITY_U", 0x0053),
    ("MEM_MAX_VELOCITY_X", 0x0023),
    ("MEM_MAX_VELOCITY_Y", 0x0033),
    ("MEM_MAX_VELOCITY_Z", 0x0043),
    ("MEM_PC_LOCK_1", 0x0301),
    ("MEM_PC_LOCK_2", 0x0302),
    ("MEM_PC_LOCK_3", 0x0303),
    ("MEM_PC_LOCK_4", 0x0304),
    ("MEM_PC_LOCK_5", 0x0305),
    ("MEM_PC_LOCK_6", 0x0306),
    ("MEM_PC_LOCK_7", 0x0307),
    ("MEM_PRECISION_U", 0x0051),
    ("MEM_PRECISION_X", 0x0021),
    ("MEM_PRECISION_Y", 0x0031),
    ("MEM_PRECISION_Z", 0x0041),
    ("MEM_PRE_WORK_TIME", 0x0408),
    ("MEM_ROTATE_D", 0x0221),
    ("MEM_ROTATE_PULSE", 0x021F),
    ("MEM_SHUT_DOWN_DELAY", 0x020D),
    ("MEM_START_VELOCITY_U", 0x0054),
    ("MEM_START_VELOCITY_X", 0x0024),
    ("MEM_START_VELOCITY_Y", 0x0034),
    ("MEM_START_VELOCITY_Z", 0x0044),
    ("MEM_SYN_ACC", 0x0202),
    ("MEM_SYSTEM_CONTROL_MODE", 0x0010),
    ("MEM_SYSTEM_SETTINGS", 0x0200),
    ("MEM_TOTAL_DOC_NUMBER", 0x0405),
    ("MEM_TOTAL_LASER_WORK_TIME", 0x0311),
    ("MEM_TOTAL_OPEN_TIME", 0x0401),
    ("MEM_TOTAL_WORK_LENGTH_1", 0x0423),
    ("MEM_TOTAL_WORK_LENGTH_2", 0x0433),
    ("MEM_TOTAL_WORK_LENGTH_3", 0x0443),
    ("MEM_TOTAL_WORK_LENGTH_4", 0x0453),
    ("MEM_TOTAL_WORK_NUMBER", 0x0403),
    ("MEM_TOTAL_WORK_TIME", 0x0402),
    ("MEM_TRAVEL_U", 0x0056),
    ("MEM_TRAVEL_Z", 0x0046),
    ("MEM_TURN_ACC", 0x0209),
    ("MEM_TURN_RATIO", 0x021B),
    ("MEM_TURN_VELOCITY", 0x0201),
    ("MEM_UNKNOWN", 0x0B12),
    ("MEM_USER_PARA_1", 0x0226),
    ("MEM_U_HOME_VELOCITY", 0x022C),
    ("MEM_U_WORK_VELOCITY", 0x022D),
    ("MEM_X_ENG_ACC", 0x0225),
    ("MEM_X_MINIMUM_ENG_VELOCITY", 0x0224),
    ("MEM_Y_ENG_ACC", 0x0235),
    ("MEM_Y_MINIMUM_ENG_VELOCITY", 0x0234),
    ("MEM_Z_G0_VELOCITY", 0x022A),
    ("MEM_Z_HOME_VELOCITY", 0x0228),
    ("MEM_Z_PEN_UP_POSITION", 0x022B),
    ("MEM_Z_WORK_VELOCITY", 0x0229),
    ("TBD:Unknown address", 0x0620),
];

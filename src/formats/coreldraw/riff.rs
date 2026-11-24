// Adapted from the TraceDraw project, crates/cdr-format/src/riff.rs
// (https://github.com/tracedraw/tracedraw), MIT OR Apache-2.0,
// Copyright (c) 2026 TraceDraw members and contributors.
// Modified for raygeo: local error type, logging removed, raw-record
// helpers dropped, tests removed (tests live under tests/).

//! RIFF chunk walker. Builds a tree of chunks referencing the source
//! bytes by offset, so nothing is copied until a parser asks for a
//! payload.
//!
//! The RIFF layout is standard: `fourcc u32(size) payload [pad]`, with
//! `LIST` chunks carrying a list type and nested chunks. Two format
//! quirks:
//! - `cmpr` lists hold zlib-compressed sub-streams (CDR 7 to version 13
//!   with compression on): two `CPng` blocks, the first with the chunk
//!   stream and the second with a pool of chunk sizes. Inside the inflated
//!   stream a chunk's size field is an index into that pool, not a byte
//!   count.
//! - Version 16 and later keep most payloads outside the RIFF stream: a
//!   chunk whose declared size is exactly 16 bytes is a redirect record
//!   `stream u32, length u32, offset u32, reserved u32` pointing into one
//!   of the external data streams (`content/data/*.dat`, in
//!   `dataFileList.dat` order); stream `0xFFFF_FFFF` means the payload (at
//!   most 8 bytes) is stored inline right after the length.

use std::sync::Arc;

use crate::formats::coreldraw::error::{Error, Result};

/// A buffer a tree reads from.
pub(crate) type SharedBytes = Arc<Vec<u8>>;

#[derive(Debug, Clone)]
pub(crate) struct Chunk {
    pub id: [u8; 4],
    /// For `LIST`/`RIFF` chunks: the list type (`page`, `layr`, ...).
    pub list_type: Option<[u8; 4]>,
    /// Payload range in the source buffer this chunk belongs to.
    pub start: usize,
    pub end: usize,
    /// Which buffer the range refers to: 0 = main stream, n = stream n in
    /// `Tree::streams`.
    pub stream: usize,
    pub children: Vec<Chunk>,
}

impl Chunk {
    /// Does this chunk match `id`, or for lists, the list type?
    pub fn is(&self, name: &[u8; 4]) -> bool {
        self.list_type.as_ref() == Some(name) || &self.id == name
    }

    pub fn find(&self, name: &[u8; 4]) -> Option<&Chunk> {
        self.children.iter().find(|c| c.is(name))
    }

    /// Depth-first walk over all descendants, including self.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Chunk, usize)) {
        fn go<'a>(
            c: &'a Chunk,
            depth: usize,
            f: &mut dyn FnMut(&'a Chunk, usize),
        ) {
            f(c, depth);
            for ch in &c.children {
                go(ch, depth + 1, f);
            }
        }
        go(self, 0, f);
    }
}

#[derive(Debug)]
pub(crate) struct Tree {
    pub root: Chunk,
    /// Out-of-line buffers, indexed from 1 (0 is the main buffer): first
    /// the external data streams of version 16+ files, then inflated
    /// `cmpr` streams.
    pub streams: Vec<SharedBytes>,
}

/// Parsing context shared down the chunk tree.
struct Ctx {
    /// Number of external data streams (version 16+); 0 for older files.
    externals: usize,
    /// Whether 16-byte chunks are redirect records.
    redirects: bool,
    /// Major format version; 0 when unknown (treated as recent).
    version: u16,
    /// Bytes inflated from compressed blocks so far, against
    /// [`MAX_INFLATED_TOTAL`].
    inflated: std::cell::Cell<u64>,
}

/// The most all the compressed blocks of one file may inflate to
/// together; blocks past it are skipped.
const MAX_INFLATED_TOTAL: u64 = 1 << 30;

/// Size of a redirect record (version 16+).
const REDIRECT_LEN: usize = 16;

impl Tree {
    /// Payload bytes of a chunk.
    pub fn data<'a>(&'a self, main: &'a [u8], c: &Chunk) -> &'a [u8] {
        let buf: &[u8] = if c.stream == 0 {
            main
        } else {
            &self.streams[c.stream - 1]
        };
        let end = c.end.min(buf.len());
        let start = c.start.min(end);
        &buf[start..end]
    }
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Parse a RIFF stream whose chunks may redirect into `externals`
/// (version 16+). `version` is the major format version; redirects are
/// only recognized from 16 on.
pub(crate) fn parse_with_externals(
    main: &[u8],
    externals: Vec<Vec<u8>>,
    version: u16,
) -> Result<Tree> {
    parse_with_shared(
        main,
        externals.into_iter().map(Arc::new).collect(),
        version,
    )
}

fn parse_with_shared(
    main: &[u8],
    externals: Vec<SharedBytes>,
    version: u16,
) -> Result<Tree> {
    if main.len() < 12 || &main[0..4] != b"RIFF" {
        return Err(Error::NotCdr);
    }
    let ctx = Ctx {
        externals: externals.len(),
        redirects: version >= 16,
        version,
        inflated: std::cell::Cell::new(0),
    };
    let mut streams = externals;
    // The header size is advisory: truncated or padded files are common
    // enough that trusting the buffer length is the safer choice.
    let _declared = u32le(main, 4).ok_or(Error::Truncated(4))?;
    let end = main.len();
    let list_type: [u8; 4] =
        main[8..12].try_into().map_err(|_| Error::Truncated(8))?;
    let children =
        parse_children(main, 0, 12, end, &mut streams, &ctx, None, 1);
    Ok(Tree {
        root: Chunk {
            id: *b"RIFF",
            list_type: Some(list_type),
            start: 12,
            end,
            stream: 0,
            children,
        },
        streams,
    })
}

/// How deep lists may nest inside the `RIFF` chunk. Real files nest a few
/// dozen levels; a damaged or hostile file can nest a `LIST` at every 12
/// bytes, and the walker recurses once per level. A list beyond this
/// depth is kept as a leaf.
pub(crate) const MAX_LIST_DEPTH: usize = 256;

/// Parse sibling chunks in `buf[pos..end]`. Malformed data ends the list
/// quietly; whatever was parsed before is kept. Inside a compressed
/// stream `sizes` is the pool of chunk sizes that the size fields index
/// into. `depth` counts the lists above these chunks, the `RIFF` chunk
/// being depth 1; a list at [`MAX_LIST_DEPTH`] gets no children.
#[allow(clippy::too_many_arguments)]
fn parse_children(
    buf: &[u8],
    stream: usize,
    mut pos: usize,
    end: usize,
    streams: &mut Vec<SharedBytes>,
    ctx: &Ctx,
    sizes: Option<&[u32]>,
    depth: usize,
) -> Vec<Chunk> {
    let mut out = Vec::new();
    if depth >= MAX_LIST_DEPTH {
        return out;
    }
    while pos + 8 <= end {
        let id: [u8; 4] = match buf.get(pos..pos + 4) {
            Some(s) => [s[0], s[1], s[2], s[3]],
            None => break,
        };
        let Some(raw) = u32le(buf, pos + 4) else {
            break;
        };
        // In a compressed stream the field is an index into the sizes pool.
        let size = match sizes {
            Some(pool) => match pool.get(raw as usize) {
                Some(s) => *s as usize,
                None => break,
            },
            None => raw as usize,
        };
        let payload = pos + 8;
        let payload_end = payload.saturating_add(size).min(end);
        if payload > end {
            break;
        }
        let is_list = &id == b"LIST";
        // Version 16+ redirect record: resolve where the payload lives.
        let mut body = (stream, payload, payload_end);
        if ctx.redirects
            && size == REDIRECT_LEN
            && payload_end - payload == REDIRECT_LEN
        {
            let s = u32le(buf, payload).unwrap_or(u32::MAX);
            let len = u32le(buf, payload + 4).unwrap_or(0) as usize;
            if s == u32::MAX {
                // Inline payload of up to 8 bytes after the length field.
                let start = payload + 8;
                body =
                    (stream, start, start.saturating_add(len).min(payload_end));
            } else if (s as usize) < ctx.externals {
                let ofs = u32le(buf, payload + 8).unwrap_or(0) as usize;
                let ext_len = streams[s as usize].len();
                let start = ofs.min(ext_len);
                body = (
                    s as usize + 1,
                    start,
                    ofs.saturating_add(len).min(ext_len),
                );
            }
        }
        let (bstream, bstart, bend) = body;
        if is_list {
            // The list body may be in another buffer than the chunk header.
            let shared = if bstream != stream && bstream > 0 {
                streams.get(bstream - 1).cloned()
            } else {
                None
            };
            let bbuf: &[u8] = match &shared {
                Some(t) => t,
                None => buf,
            };
            let lt: Option<[u8; 4]> = bbuf
                .get(bstart..bstart + 4)
                .map(|s| [s[0], s[1], s[2], s[3]]);
            let children = match lt {
                Some(t) if &t == b"cmpr" => inflate_cmpr(
                    bbuf,
                    bstart + 4,
                    bend,
                    streams,
                    ctx,
                    depth + 1,
                ),
                // From version 7 the `stlt` list body is one record, not
                // sub-chunks (the parser reads it from the list's data).
                Some(t)
                    if &t == b"stlt"
                        && (ctx.version >= 7 || ctx.version == 0) =>
                {
                    Vec::new()
                }
                Some(_) => parse_children(
                    bbuf,
                    bstream,
                    bstart + 4,
                    bend,
                    streams,
                    ctx,
                    sizes,
                    depth + 1,
                ),
                None => Vec::new(),
            };
            out.push(Chunk {
                id,
                list_type: lt,
                start: bstart + 4,
                end: bend,
                stream: bstream,
                children,
            });
        } else {
            out.push(Chunk {
                id,
                list_type: None,
                start: bstart,
                end: bend,
                stream: bstream,
                children: Vec::new(),
            });
        }
        // Chunks are word-aligned.
        pos = payload + size + (size & 1);
    }
    out
}

/// A `LIST cmpr` body holds two size pairs (compressed, uncompressed; u32
/// each), then two `CPng` blocks of those compressed sizes. Each block is
/// `CPng`, the bytes `01 00 04 00`, and a zlib stream. The first block
/// inflates to the chunk stream, the second to a pool of u32 chunk sizes
/// that the size fields of the inflated chunks index into.
fn inflate_cmpr(
    buf: &[u8],
    pos: usize,
    end: usize,
    streams: &mut Vec<SharedBytes>,
    ctx: &Ctx,
    depth: usize,
) -> Vec<Chunk> {
    let (Some(c0), Some(c1)) = (u32le(buf, pos), u32le(buf, pos + 8)) else {
        return Vec::new();
    };
    let first = pos + 16;
    let second = first.saturating_add(c0 as usize);
    if ctx.inflated.get() >= MAX_INFLATED_TOTAL {
        return Vec::new();
    }
    let Some(chunks) = inflate_cpng(buf, first, second.min(end)) else {
        return Vec::new();
    };
    ctx.inflated.set(ctx.inflated.get() + chunks.len() as u64);
    // Version 12 files start with a `cmpr` holding two empty streams.
    if chunks.is_empty() {
        return Vec::new();
    }
    let sizes: Vec<u32> =
        inflate_cpng(buf, second, second.saturating_add(c1 as usize).min(end))
            .unwrap_or_default()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|s| u32::from_le_bytes(*s))
            .collect();
    if sizes.is_empty() {
        return Vec::new();
    }
    let inflated = Arc::new(chunks);
    streams.push(inflated.clone());
    let idx = streams.len();
    let len = inflated.len();
    let children = parse_children(
        &inflated,
        idx,
        0,
        len,
        streams,
        ctx,
        Some(&sizes),
        depth + 1,
    );
    vec![Chunk {
        id: *b"cmpr",
        list_type: Some(*b"cmpr"),
        start: 0,
        end: len,
        stream: idx,
        children,
    }]
}

/// Inflate one `CPng` block in `buf[start..end]`.
fn inflate_cpng(buf: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
    let block = buf.get(start..end)?;
    if block.len() < 9 || &block[0..4] != b"CPng" {
        return None;
    }
    let mut inflated = Vec::new();
    let dec = flate2::read::ZlibDecoder::new(&block[8..]);
    std::io::Read::read_to_end(
        &mut std::io::Read::take(dec, MAX_INFLATED_TOTAL),
        &mut inflated,
    )
    .ok()?;
    Some(inflated)
}

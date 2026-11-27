//! The Ruida laser controller protocol.
//!
//! Ruida controllers (RDC6442S, RDC8445S and relatives) speak a
//! binary, 7-bit packet protocol over USB, UDP or TCP. Jobs are
//! written as rpascript command streams; live motion and job control
//! use the same commands outside a job. This module owns the wire
//! codec, the packet transport with its ACK handshake, the status
//! monitor and the driver session.
//!
//! The protocol layers here are pure Rust and do not depend on PyO3.

pub mod codec;
pub mod driver;
pub mod status;
pub mod transport;

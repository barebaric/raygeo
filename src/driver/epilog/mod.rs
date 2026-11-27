//! Epilog laser support: job submission over the BSD LPD protocol.
//!
//! Note on RFC 1179 §3.1: the RFC asks clients to originate from a
//! source port in 721..=731. This session deliberately does not bind
//! one — the shared dial seam picks the port, and CUPS-era daemons
//! (including Epilog's) accept any client port. Binding the range
//! would need privileged-port code outside that seam for no known
//! interoperability gain.
//!
//! Epilog laser cutters (Zing 16, Zing 24/6030, and the wider family
//! that shares the print driver) are print devices: the host uploads
//! a print job to the machine's network print queue and the operator
//! starts it at the control panel. There is no status channel, no
//! homing, jogging or power control over the network.
//!
//! The job body is whatever the encoder produces; this module treats
//! it as opaque bytes and does not interpret the printer language.
//!
//! The transfer is the classic BSD "receive job" exchange on
//! port 515 (RFC 1179 §5.2 and §6):
//!
//! ```text
//! -> \x02<queue><LF>                        receive a job (queue name)
//! <- \x00
//! -> \x02 <len> cfA<n>rayforge <LF>          control file header
//! <- \x00
//! -> <control file bytes> <NUL>              (H/P/J/N/l/U lines)
//! <- \x00
//! -> \x03 <len> dfA<n>rayforge <LF>          data file header
//! <- \x00
//! -> <payload bytes>                         the print job
//! <- \x00
//! ```
//!
//! The queue operand is empty by default: the reference driver sends
//! the receive-job command as `\x02\n`, and the Epilog print servers
//! accept it. The control file's `H` and `P` lines carry the configured
//! submitting host and user, and `J`/`N` carry the job title so the
//! machine can show it in its job list.
//!
//! Every stage must be acknowledged with a single `\x00` byte. A
//! non-zero reply is a rejection; a short read is a disconnect. A
//! partial transfer is never retried, because the machine may have
//! queued a truncated job; instead the client sends the RFC 1179 §6.1
//! abort (`\x01` LF) so the daemon drops what it received.
//!
//! Jobs are handed to the session as the encoder's byte stream (see
//! the `upload` extra rather than the G-code `run` path).

pub mod driver;

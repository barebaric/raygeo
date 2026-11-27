//! Protocol sessions for xTool machines.
//!
//! D-series machines are GRBL-derived but their network API is *not*
//! ESP3D or GRBL-over-TCP: single commands are triggered with
//! `GET /cmd?cmd=…`, multi-line blocks with `POST /cmd`, jobs are
//! uploaded to `/tmp.gcode` via multipart `/upload` and started with
//! `/read`, and pause/resume/stop go through `/cnc/data`.  The HTTP
//! API acknowledges every request with a JSON body whose `result`
//! member is `"ok"` or `"fail"` — it never returns the GRBL response
//! text of the executed command.
//!
//! The D-series WebSocket (default port 8081) is a one-way notification
//! channel: it carries job-state events (`ok:IDLE`, `ok:WORKING_*`,
//! `ok:PAUSING`, `WORK_STOPPED`) and safety alarms (`err:*`).
//!
//! The M2 uses a separate WS-V2 TLS protocol on port 28900. Its
//! session supports state monitoring, motion and job control for jobs
//! already staged on the machine; G-code job upload is not
//! implemented.  The `m2` session needs the `m2` feature (its
//! tokio-tungstenite stack has no wasm build).

pub mod driver;
#[cfg(feature = "m2")]
pub mod m2;

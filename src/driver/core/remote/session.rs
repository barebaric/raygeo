//! Hosted-session abstraction used by the daemon.
//!
//! The daemon owns no protocol code: a [`SessionFactory`] creates a
//! [`HostedSession`] for a protocol name and its JSON config. Pure
//! Rust embedders implement these traits directly; the Python
//! bindings adapt the existing PyO3 session objects.  Errors are
//! the crate's [`DriverError`].

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::driver::core::driver::DriverError;
use crate::driver::core::remote::protocol::WireEvent;
use crate::driver::core::types::DeviceState;

/// Receives events from a hosted session in wire form.
pub trait RemoteEventSink: Send + Sync {
    fn emit(&self, event: WireEvent);
}

/// A session the daemon can host.
#[async_trait]
pub trait HostedSession: Send + Sync {
    fn protocol(&self) -> &str;
    fn state(&self) -> DeviceState;
    fn job_running(&self) -> bool;
    fn resource_uri(&self) -> Option<String>;
    fn buffer_count(&self) -> usize;
    fn rx_buffer_size(&self) -> usize;
    fn pending_commands(&self) -> Vec<String>;
    fn set_event_sink(&self, sink: Arc<dyn RemoteEventSink>);

    async fn connect(&self) -> Result<(), DriverError>;
    async fn disconnect(&self) -> Result<(), DriverError>;

    /// Execute one non-job session method. Arguments arrive as a
    /// JSON object; the result is the method's return value.
    async fn execute(
        &self,
        method: &str,
        args: Value,
    ) -> Result<Value, DriverError>;

    /// Stream a job. Progress and job errors are emitted through the
    /// event sink; the daemon marks the job finished when this
    /// returns or errors.
    async fn run(
        &self,
        gcode: String,
        op_map: Option<HashMap<usize, i64>>,
        op_estimates: Option<Vec<f64>>,
    ) -> Result<(), DriverError>;
}

/// Creates sessions for the daemon.
pub trait SessionFactory: Send + Sync {
    fn create(
        &self,
        protocol: &str,
        config: Value,
        dialect: Value,
        sink: Arc<dyn RemoteEventSink>,
    ) -> Result<Arc<dyn HostedSession>, DriverError>;
}

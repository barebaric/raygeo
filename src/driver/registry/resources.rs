//! The process-wide physical-resource lock.
//!
//! Two drivers must not talk to the same serial port or endpoint at
//! the same time.  A [`ResourceGuard`] holds one `resource_uri` for
//! as long as it lives; dropping it (on disconnect or when the
//! driver is dropped) releases the resource.  Transports without a
//! resource identifier (the mock transport) are not locked.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use crate::driver::core::driver::DriverError;

static ACTIVE: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Holds one physical resource for the lifetime of a connection.
pub struct ResourceGuard {
    resource: Option<String>,
}

impl ResourceGuard {
    /// Take *resource*; `None` means the driver owns no physical
    /// resource and is always granted.
    pub fn acquire(
        resource: Option<&str>,
        owner: Option<&str>,
    ) -> Result<Self, DriverError> {
        let Some(resource) = resource else {
            return Ok(Self { resource: None });
        };
        let mut active = ACTIVE.lock().unwrap();
        if active.contains(resource) {
            return Err(DriverError::ResourceBusy {
                resource: resource.to_string(),
                owner: owner.unwrap_or("another driver").to_string(),
            });
        }
        active.insert(resource.to_string());
        Ok(Self {
            resource: Some(resource.to_string()),
        })
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        if let Some(resource) = self.resource.take() {
            ACTIVE.lock().unwrap().remove(&resource);
        }
    }
}

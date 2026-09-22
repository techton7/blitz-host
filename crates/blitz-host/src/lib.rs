//! # blitz-host
//!
//! Canonical facade, CLI, and host control plane for live Blitz desktop applications.
//!
//! Re-exports the core underlying building blocks:
//! - [`protocol`]: Pure typed JSON schemas for Inspect, Act, and Settle
//! - [`client`] / [`transport`]: Unix Domain Socket transport and DebugClient
//! - [`bridge`]: UI-thread DOM inspector and frame synchronization queues
//! - [`host`]: High-level host integration and lifecycle coordinator

pub use blitz_host_bridge as bridge;
pub use blitz_host_protocol as protocol;
pub use blitz_host_transport as client;
pub use blitz_host_transport as transport;

pub mod host;

pub use host::HostControl;
pub use blitz_host_protocol::{
    ActionRequest, ActionResponse, ControlRequest, ControlResponse, HostDescriptor, InspectRequest,
    InspectResponse, SettleRequest, SettleResponse,
};
pub use blitz_host_transport::DebugClient;

pub mod prelude {
    pub use crate::host::HostControl;
    pub use crate::protocol::{
        ActionRequest, ActionResponse, InspectRequest, InspectResponse, SettleRequest,
        SettleResponse,
    };
    pub use crate::client::DebugClient;
}

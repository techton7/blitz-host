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

#[cfg(feature = "dioxus-native")]
pub mod dioxus;

pub use host::{init_default, init_global, init_if_debug, init_if_debug_default, HostControl};
#[cfg(feature = "dioxus-native")]
pub use dioxus::BlitzHost;

pub use blitz_host_protocol::{
    ActionRequest, ActionResponse, CaptureMetadataResponse, CaptureRequest, CaptureResponse,
    ControlRequest, ControlResponse, HostDescriptor, InspectRequest, InspectResponse, SettleRequest,
    SettleResponse,
};
pub use blitz_host_transport::DebugClient;

pub mod prelude {
    pub use crate::host::{
        init_default, init_global, init_if_debug, init_if_debug_default, HostControl,
    };
    pub use crate::protocol::{
        ActionRequest, ActionResponse, InspectRequest, InspectResponse, SettleRequest,
        SettleResponse,
    };
    pub use crate::client::DebugClient;

    #[cfg(feature = "dioxus-native")]
    pub use crate::dioxus::BlitzHost;
}


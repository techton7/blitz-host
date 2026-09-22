use std::sync::Mutex;

use blitz_dom::BaseDocument;
use blitz_host_bridge::HostBridge;
use blitz_host_protocol::{ActionRequest, ActionResponse, HostDescriptor};
use blitz_host_transport::DebugServer;

static GLOBAL_HOST_CONTROL: Mutex<Option<HostControl>> = Mutex::new(None);

/// High-level host-side debug control lifecycle and service coordinator.
///
/// Encapsulates server startup, bridge event channels, and UI-thread frame polling
/// so host applications (such as runners or debug targets) don't need to manually
/// manage static mutexes, sockets, and raw bridge channels.
pub struct HostControl {
    server: DebugServer,
    bridge: HostBridge,
}

impl HostControl {
    /// Checks whether debug control was explicitly requested via `--debug-control` argument
    /// or `BLITZ_DEBUG_CONTROL` environment variable.
    pub fn is_requested() -> bool {
        std::env::args().any(|arg| arg == "--debug-control")
            || std::env::var("BLITZ_DEBUG_CONTROL").is_ok()
    }

    /// Starts a local HostControl instance if requested by CLI argument or environment variable.
    pub fn start_if_requested(app_name: &str, app_version: &str) -> Option<Self> {
        if !Self::is_requested() {
            return None;
        }

        match DebugServer::start(app_name, app_version) {
            Ok((server, command_rx)) => {
                println!("[blitz-host] Local debug control server initialized");
                println!("  • Socket    : {}", server.socket_path().display());
                println!("  • Instance  : {}", server.descriptor().instance_id);
                println!("  • PID       : {}", server.descriptor().pid);
                Some(Self {
                    server,
                    bridge: HostBridge::new(command_rx),
                })
            }
            Err(e) => {
                eprintln!("[blitz-host] Failed to start debug control server: {e}");
                None
            }
        }
    }

    /// Initializes a process-global HostControl singleton if requested.
    ///
    /// Returns true if debug control was requested and successfully initialized.
    pub fn init_global_if_requested(app_name: &str, app_version: &str) -> bool {
        if !Self::is_requested() {
            return false;
        }
        if let Some(ctrl) = Self::start_if_requested(app_name, app_version) {
            *GLOBAL_HOST_CONTROL.lock().unwrap() = Some(ctrl);
            true
        } else {
            false
        }
    }

    /// Checks if the process-global HostControl singleton is currently active.
    pub fn is_global_active() -> bool {
        GLOBAL_HOST_CONTROL
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// Polls and services pending control requests against the live BaseDocument.
    pub fn poll_and_service<F>(
        &mut self,
        doc: &BaseDocument,
        current_frame: u64,
        dispatch_action: F,
    ) -> usize
    where
        F: FnMut(&ActionRequest, &BaseDocument) -> Result<ActionResponse, String>,
    {
        self.bridge.poll_and_service_with(doc, current_frame, dispatch_action)
    }

    /// Services the process-global HostControl instance on the UI thread for the current frame.
    ///
    /// Returns the number of requests serviced during this turn.
    pub fn service_global_frame<F>(
        doc: &BaseDocument,
        current_frame: u64,
        dispatch_action: F,
    ) -> usize
    where
        F: FnMut(&ActionRequest, &BaseDocument) -> Result<ActionResponse, String>,
    {
        if let Ok(mut guard) = GLOBAL_HOST_CONTROL.lock() {
            if let Some(ctrl) = guard.as_mut() {
                return ctrl.poll_and_service(doc, current_frame, dispatch_action);
            }
        }
        0
    }

    /// Convenience helper for standard click action handling.
    ///
    /// Dispatches a click via the provided closure and constructs a formatted ActionResponse.
    pub fn handle_action_click<F>(
        action_req: &ActionRequest,
        base_doc: &BaseDocument,
        mut click_fn: F,
    ) -> Result<ActionResponse, String>
    where
        F: FnMut(&BaseDocument, u64) -> bool,
    {
        match action_req {
            ActionRequest::Click { node_id } => {
                if click_fn(base_doc, *node_id) {
                    Ok(ActionResponse {
                        success: true,
                        node_id: *node_id,
                        message: Some(format!("Dispatched synthetic click to node #{node_id}")),
                    })
                } else {
                    Err(format!("Node #{node_id} or active listener not found in document"))
                }
            }
        }
    }

    /// Access the server descriptor metadata.
    pub fn descriptor(&self) -> &HostDescriptor {
        self.server.descriptor()
    }
}

/// Convenience function to initialize blitz-host debug control if requested via
/// `--debug-control` CLI argument or `BLITZ_DEBUG_CONTROL` environment variable.
pub fn init_if_debug(app_name: &str, app_version: &str) -> bool {
    HostControl::init_global_if_requested(app_name, app_version)
}

/// Convenience function to initialize blitz-host debug control with default application name
/// inferred from current executable if requested.
pub fn init_if_debug_default() -> bool {
    let app_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "blitz-app".to_string());
    HostControl::init_global_if_requested(&app_name, "0.1.0")
}

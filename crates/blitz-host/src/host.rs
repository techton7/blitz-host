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
    /// Checks whether debug control is enabled.
    ///
    /// In the feature-enabled development lane, debug control is active by default.
    /// It can be explicitly suppressed via `--no-debug-control`, `BLITZ_DEBUG_CONTROL=0`,
    /// or `BLITZ_HOST_DISABLED=1`.
    pub fn is_enabled() -> bool {
        if std::env::args().any(|arg| arg == "--no-debug-control") {
            return false;
        }
        if let Ok(val) = std::env::var("BLITZ_DEBUG_CONTROL") {
            if val == "0" || val.eq_ignore_ascii_case("false") {
                return false;
            }
        }
        if let Ok(val) = std::env::var("BLITZ_HOST_DISABLED") {
            if val == "1" || val.eq_ignore_ascii_case("true") {
                return false;
            }
        }
        true
    }

    /// Checks whether debug control is active or enabled.
    ///
    /// Backward-compatible alias for [`Self::is_enabled`].
    pub fn is_requested() -> bool {
        Self::is_enabled()
    }

    /// Starts a local HostControl instance directly.
    pub fn start(app_name: &str, app_version: &str) -> Option<Self> {
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

    /// Starts a local HostControl instance if enabled in the development lane.
    pub fn start_if_requested(app_name: &str, app_version: &str) -> Option<Self> {
        if !Self::is_enabled() {
            return None;
        }
        Self::start(app_name, app_version)
    }

    /// Initializes a process-global HostControl singleton if enabled.
    ///
    /// Returns true if debug control was successfully initialized or was already active.
    pub fn init_global(app_name: &str, app_version: &str) -> bool {
        if !Self::is_enabled() {
            return false;
        }
        if Self::is_global_active() {
            return true;
        }
        if let Some(ctrl) = Self::start(app_name, app_version) {
            *GLOBAL_HOST_CONTROL.lock().unwrap() = Some(ctrl);
            true
        } else {
            false
        }
    }

    /// Backward-compatible alias for [`Self::init_global`].
    pub fn init_global_if_requested(app_name: &str, app_version: &str) -> bool {
        Self::init_global(app_name, app_version)
    }


    /// Checks if the process-global HostControl singleton is currently active.
    pub fn is_global_active() -> bool {
        GLOBAL_HOST_CONTROL
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// Update primary window and document metadata in descriptor.
    pub fn set_primary_window(&mut self, window_id: Option<u64>, document_id: Option<usize>) {
        let _ = self.server.update_primary_window(window_id, document_id);
    }

    /// Update primary window and document metadata in the process-global singleton if active.
    pub fn set_global_primary_window(window_id: Option<u64>, document_id: Option<usize>) {
        if let Ok(mut guard) = GLOBAL_HOST_CONTROL.lock() {
            if let Some(ctrl) = guard.as_mut() {
                ctrl.set_primary_window(window_id, document_id);
            }
        }
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
        if self.server.descriptor().primary_document_id.is_none() {
            let win_id = self.server.descriptor().primary_window_id;
            let _ = self.server.update_primary_window(win_id, Some(doc.id()));
        }
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
            ActionRequest::Click { node_id, .. } => {
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

/// Convenience function to initialize blitz-host debug control in the development lane.
pub fn init_default() -> bool {
    let app_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "blitz-app".to_string());
    HostControl::init_global(&app_name, "0.1.0")
}

/// Convenience function to initialize blitz-host debug control with explicit metadata.
pub fn init_global(app_name: &str, app_version: &str) -> bool {
    HostControl::init_global(app_name, app_version)
}

/// Backward-compatible alias for [`init_global`].
pub fn init_if_debug(app_name: &str, app_version: &str) -> bool {
    HostControl::init_global(app_name, app_version)
}

/// Backward-compatible alias for [`init_default`].
pub fn init_if_debug_default() -> bool {
    init_default()
}


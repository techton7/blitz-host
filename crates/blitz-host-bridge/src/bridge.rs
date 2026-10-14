use std::sync::mpsc::Receiver;

use blitz_dom::BaseDocument;
use blitz_host_protocol::{
    ActionRequest, ActionResponse, ControlRequest, ControlResponse, SettleResponse,
};
use blitz_host_transport::ControlBridgeRequest;

use crate::inspect::inspect_document;

/// Detailed summary of requests serviced during a host frame poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceResult {
    /// Total number of requests that received an immediate response this frame.
    pub responded: usize,
    /// Number of multi-frame settle requests still awaiting future frames.
    pub pending_settles: usize,
    /// Whether another frame should be rendered (due to pending settles or UI-mutating actions).
    pub needs_redraw: bool,
}

impl ServiceResult {
    /// Total number of requests responded during this turn.
    pub fn serviced(&self) -> usize {
        self.responded
    }
}

/// Pending settle request waiting for a target frame count.
struct PendingSettle {
    bridge_req: ControlBridgeRequest,
    target_frame: u64,
    requested_frames: u32,
}

/// The running-host bridge adapter.
///
/// Owned by the host application on the main/UI thread.
/// Polls incoming requests from the transport server and executes them safely against the live `BaseDocument`.
pub struct HostBridge {
    receiver: Receiver<ControlBridgeRequest>,
    pending_settles: Vec<PendingSettle>,
}

impl HostBridge {
    /// Create a new HostBridge wrapping the transport server's command receiver.
    pub fn new(receiver: Receiver<ControlBridgeRequest>) -> Self {
        Self {
            receiver,
            pending_settles: Vec::new(),
        }
    }

    /// Legacy single-frame inspect poll without action execution.
    pub fn poll_and_service(&mut self, doc: &mut BaseDocument) -> ServiceResult {
        self.poll_and_service_with(doc, 0, |_act, _doc| {
            Err("Action dispatcher not configured".to_string())
        })
    }

    /// Non-blocking check for pending requests with full act + settle + frame support.
    ///
    /// Must be called from the UI thread holding the document (e.g. within a `use_frame` hook or event loop handler).
    /// Returns the structured summary of requests serviced during this poll.
    pub fn poll_and_service_with(
        &mut self,
        doc: &mut BaseDocument,
        current_frame: u64,
        mut dispatch_action: impl FnMut(
            &ActionRequest,
            &mut BaseDocument,
        ) -> Result<ActionResponse, String>,
    ) -> ServiceResult {
        let mut responded = 0;
        let mut needs_redraw = false;

        // 1. Service any pending settle requests that have reached target frame
        let mut remaining_settles = Vec::with_capacity(self.pending_settles.len());
        for settle in self.pending_settles.drain(..) {
            if current_frame >= settle.target_frame {
                settle
                    .bridge_req
                    .respond(ControlResponse::SettleSuccess(SettleResponse {
                        settled: true,
                        frames_waited: settle.requested_frames,
                        current_frame,
                    }));
                responded += 1;
            } else {
                remaining_settles.push(settle);
            }
        }
        self.pending_settles = remaining_settles;
        if !self.pending_settles.is_empty() {
            needs_redraw = true;
        }

        // 2. Poll incoming control requests
        while let Ok(bridge_req) = self.receiver.try_recv() {
            match &bridge_req.request {
                ControlRequest::Inspect(inspect_req) => {
                    let mut response = inspect_document(doc, inspect_req.clone());
                    response.current_frame = Some(current_frame);
                    bridge_req.respond(ControlResponse::InspectSuccess(response));
                    responded += 1;
                }
                ControlRequest::Act(action_req) => {
                    let resp = match dispatch_action(action_req, doc) {
                        Ok(act_resp) => ControlResponse::ActionSuccess(act_resp),
                        Err(err_msg) => ControlResponse::Error(err_msg),
                    };
                    bridge_req.respond(resp);
                    responded += 1;
                    needs_redraw = true;
                }
                ControlRequest::Settle(settle_req) => {
                    let requested_frames = settle_req.frames.max(1);
                    self.pending_settles.push(PendingSettle {
                        bridge_req,
                        target_frame: current_frame + requested_frames as u64,
                        requested_frames,
                    });
                    needs_redraw = true;
                }
                ControlRequest::Capture(capture_req) => {
                    let response = crate::capture::capture_document(doc, capture_req.clone());
                    bridge_req.respond(ControlResponse::CaptureSuccess(response));
                    responded += 1;
                }
            }
        }

        ServiceResult {
            responded,
            pending_settles: self.pending_settles.len(),
            needs_redraw,
        }
    }
}

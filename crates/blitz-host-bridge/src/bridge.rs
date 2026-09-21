use std::sync::mpsc::Receiver;

use blitz_dom::BaseDocument;
use blitz_host_protocol::{
    ActionRequest, ActionResponse, ControlRequest, ControlResponse, SettleResponse,
};
use blitz_host_transport::ControlBridgeRequest;

use crate::inspect::inspect_document;

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
    pub fn poll_and_service(&mut self, doc: &BaseDocument) -> usize {
        self.poll_and_service_with(doc, 0, |_act, _doc| {
            Err("Action dispatcher not configured".to_string())
        })
    }

    /// Non-blocking check for pending requests with full act + settle + frame support.
    ///
    /// Must be called from the UI thread holding the document (e.g. within a `use_frame` hook or event loop handler).
    /// Returns the number of requests serviced during this poll.
    pub fn poll_and_service_with(
        &mut self,
        doc: &BaseDocument,
        current_frame: u64,
        mut dispatch_action: impl FnMut(&ActionRequest, &BaseDocument) -> Result<ActionResponse, String>,
    ) -> usize {
        let mut serviced = 0;

        // 1. Service any pending settle requests that have reached target frame
        let mut remaining_settles = Vec::with_capacity(self.pending_settles.len());
        for settle in self.pending_settles.drain(..) {
            if current_frame >= settle.target_frame {
                settle.bridge_req.respond(ControlResponse::SettleSuccess(SettleResponse {
                    settled: true,
                    frames_waited: settle.requested_frames,
                    current_frame,
                }));
                serviced += 1;
            } else {
                remaining_settles.push(settle);
            }
        }
        self.pending_settles = remaining_settles;

        // 2. Poll incoming control requests
        while let Ok(bridge_req) = self.receiver.try_recv() {
            match &bridge_req.request {
                ControlRequest::Inspect(inspect_req) => {
                    let mut response = inspect_document(doc, inspect_req.clone());
                    response.current_frame = Some(current_frame);
                    bridge_req.respond(ControlResponse::InspectSuccess(response));
                    serviced += 1;
                }
                ControlRequest::Act(action_req) => {
                    let resp = match dispatch_action(action_req, doc) {
                        Ok(act_resp) => ControlResponse::ActionSuccess(act_resp),
                        Err(err_msg) => ControlResponse::Error(err_msg),
                    };
                    bridge_req.respond(resp);
                    serviced += 1;
                }
                ControlRequest::Settle(settle_req) => {
                    let requested_frames = settle_req.frames.max(1);
                    self.pending_settles.push(PendingSettle {
                        bridge_req,
                        target_frame: current_frame + requested_frames as u64,
                        requested_frames,
                    });
                }
            }
        }

        serviced
    }
}
